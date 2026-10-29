//! V8 implementation of the engine-neutral JavaScript backend contract.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use runtime_engine::{
    EngineConfiguration, EngineError, EngineEvent, EngineNotifier, EngineObjectKind, EvaluationId,
    EvaluationOutcome, EvaluationRequest, HostCompletion, HostErrorCode, HostRequest,
    HostRequestId, JavaScriptEngineBackend, JavaScriptException, MountRequest,
};
use std::cell::RefCell;
use std::collections::{HashMap, VecDeque};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::{Arc, Once};

const MAXIMUM_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAXIMUM_SAFE_INTEGER_F64: f64 = 9_007_199_254_740_991.0;
const MAXIMUM_MOUNT_URL_BYTES: usize = 4096;
const MAXIMUM_SLEEP_MILLISECONDS: u64 = 86_400_000;

/// A single-thread-owned V8 implementation of [`JavaScriptEngineBackend`].
pub struct V8Backend {
    isolate: Option<v8::OwnedIsolate>,
    context: Option<v8::Global<v8::Context>>,
    evaluate: Option<v8::Global<v8::Function>>,
    state: Rc<RefCell<BackendState>>,
    local_only: PhantomData<Rc<()>>,
}

struct PendingPromise {
    resolve: v8::Global<v8::Function>,
    reject: v8::Global<v8::Function>,
}

struct BackendState {
    configuration: EngineConfiguration,
    notifier: Option<Arc<dyn EngineNotifier>>,
    events: VecDeque<EngineEvent>,
    terminal_events: VecDeque<EngineEvent>,
    queued_event_bytes: usize,
    /// Maps backend-private settlement tokens to public evaluation IDs.
    pending_evaluations: HashMap<u64, u64>,
    pending_promises: HashMap<u64, PendingPromise>,
    next_evaluation_token: u64,
    next_host_id: u64,
    shutting_down: bool,
}

impl BackendState {
    fn unstarted() -> Self {
        Self {
            configuration: EngineConfiguration::default(),
            notifier: None,
            events: VecDeque::new(),
            terminal_events: VecDeque::new(),
            queued_event_bytes: 0,
            pending_evaluations: HashMap::new(),
            pending_promises: HashMap::new(),
            next_evaluation_token: 1,
            next_host_id: 1,
            shutting_down: false,
        }
    }

    fn enqueue(&mut self, event: EngineEvent) -> bool {
        let bytes = event.payload_len();
        let accepted = self.terminal_events.is_empty()
            && bytes <= self.configuration.maximum_event_bytes
            && self.queued_event_bytes <= self.configuration.maximum_event_bytes - bytes;
        if accepted {
            self.queued_event_bytes += bytes;
            self.events.push_back(event);
            self.notify();
        }
        accepted
    }

    fn token_for(&self, id: u64) -> Option<u64> {
        self.pending_evaluations
            .iter()
            .find_map(|(token, pending)| (*pending == id).then_some(*token))
    }

    /// Settles the evaluation behind `token`; stale tokens are ignored.
    fn settle_token(&mut self, token: u64, outcome: EvaluationOutcome) {
        if let Some(id) = self.pending_evaluations.remove(&token) {
            self.push_settlement(id, outcome);
        }
    }

    fn push_settlement(&mut self, id: u64, outcome: EvaluationOutcome) {
        let event = EngineEvent::EvaluationSettled {
            id: EvaluationId(id),
            outcome,
        };
        if self.enqueue(event) {
            return;
        }
        let fallback = EngineEvent::EvaluationSettled {
            id: EvaluationId(id),
            outcome: EvaluationOutcome::Exception(JavaScriptException {
                name: "QuotaExceeded".to_owned(),
                message: "Engine event queue exhausted".to_owned(),
            }),
        };
        if self.terminal_events.len() < self.configuration.maximum_pending_evaluations {
            self.terminal_events.push_back(fallback);
            self.notify();
        }
    }

    fn notify(&self) {
        if let Some(notifier) = &self.notifier {
            notifier.notify();
        }
    }
}

impl V8Backend {
    /// Creates an unstarted V8 backend.
    #[must_use]
    pub fn new() -> Self {
        Self {
            isolate: None,
            context: None,
            evaluate: None,
            state: Rc::new(RefCell::new(BackendState::unstarted())),
            local_only: PhantomData,
        }
    }

    fn enter<R>(
        &mut self,
        operation: impl FnOnce(&mut v8::PinScope<'_, '_>) -> Result<R, EngineError>,
    ) -> Result<R, EngineError> {
        let context = self.context.as_ref().ok_or(EngineError::InvalidState)?;
        let isolate = self.isolate.as_mut().ok_or(EngineError::InvalidState)?;
        v8::scope!(let handle_scope, isolate);
        let context = v8::Local::new(handle_scope, context);
        let scope = &mut v8::ContextScope::new(handle_scope, context);
        operation(scope)
    }
}

impl Default for V8Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl JavaScriptEngineBackend for V8Backend {
    fn start(
        &mut self,
        configuration: EngineConfiguration,
        notifier: Arc<dyn EngineNotifier>,
    ) -> Result<(), EngineError> {
        if self.isolate.is_some() {
            return Err(EngineError::InvalidState);
        }
        validate_configuration(configuration)?;
        initialize_v8();

        self.state = Rc::new(RefCell::new(BackendState {
            configuration,
            notifier: Some(notifier),
            ..BackendState::unstarted()
        }));
        let mut isolate = v8::Isolate::new(v8::CreateParams::default());
        isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
        if !isolate.set_slot(self.state.clone()) {
            return Err(EngineError::Backend);
        }

        let (context_global, evaluate) = {
            v8::scope!(let handle_scope, &mut isolate);
            let context = v8::Context::new(handle_scope, v8::ContextOptions::default());
            let context_global = v8::Global::new(handle_scope, context);
            let scope = &mut v8::ContextScope::new(handle_scope, context);
            install_callback(scope, "__zintlEvaluationDone", evaluation_done)?;
            install_callback(scope, "__zintlEvaluationFailed", evaluation_failed)?;
            install_callback(scope, "__zintlInvoke", invoke)?;
            install_callback(scope, "__zintlSleep", sleep)?;
            install_callback(scope, "__zintlReadFile", read_file)?;
            install_callback(scope, "__zintlMountOperation", mount_operation)?;
            install_callback(scope, "__zintlConsole", console_output)?;

            v8::tc_scope!(let try_catch, scope);
            let source = v8::String::new(try_catch, include_str!("bootstrap.js"))
                .ok_or(EngineError::Backend)?;
            let script =
                v8::Script::compile(try_catch, source, None).ok_or(EngineError::Backend)?;
            let bridge = script.run(try_catch).ok_or(EngineError::Backend)?;
            let bridge =
                v8::Local::<v8::Object>::try_from(bridge).map_err(|_| EngineError::Backend)?;
            let key = v8::String::new(try_catch, "evaluate").ok_or(EngineError::Backend)?;
            let evaluate = bridge
                .get(try_catch, key.into())
                .ok_or(EngineError::Backend)?;
            let evaluate =
                v8::Local::<v8::Function>::try_from(evaluate).map_err(|_| EngineError::Backend)?;
            (context_global, v8::Global::new(try_catch, evaluate))
        };

        self.context = Some(context_global);
        self.evaluate = Some(evaluate);
        self.isolate = Some(isolate);
        Ok(())
    }

    fn submit_evaluation(&mut self, request: EvaluationRequest) -> Result<(), EngineError> {
        let evaluate = self
            .evaluate
            .as_ref()
            .ok_or(EngineError::InvalidState)?
            .clone();
        let configuration = self.state.borrow().configuration;
        if request.id.0 == 0
            || request.id.0 > MAXIMUM_SAFE_INTEGER
            || request.source.len() > configuration.maximum_source_bytes
        {
            return Err(EngineError::InvalidRequest);
        }
        let token = {
            let mut state = self.state.borrow_mut();
            if state.shutting_down {
                return Err(EngineError::InvalidState);
            }
            if state.token_for(request.id.0).is_some() {
                return Err(EngineError::InvalidRequest);
            }
            if state.pending_evaluations.len() >= configuration.maximum_pending_evaluations
                || state.terminal_events.len() >= configuration.maximum_pending_evaluations
                || state.next_evaluation_token > MAXIMUM_SAFE_INTEGER
            {
                return Err(EngineError::QuotaExceeded);
            }
            let token = state.next_evaluation_token;
            state.next_evaluation_token += 1;
            state.pending_evaluations.insert(token, request.id.0);
            token
        };

        let entered = self.enter(|scope| {
            v8::tc_scope!(let try_catch, scope);
            let evaluate = v8::Local::new(try_catch, &evaluate);
            let receiver = v8::undefined(try_catch).into();
            let token_value = safe_integer_number(try_catch, token).into();
            let source = v8::String::new(try_catch, &request.source)
                .ok_or(EngineError::Backend)?
                .into();
            evaluate
                .call(try_catch, receiver, &[token_value, source])
                .ok_or(EngineError::Backend)?;
            Ok(())
        });
        // The evaluation was accepted, so a bridge failure settles it instead of
        // being reported twice.
        if entered.is_err() {
            self.state.borrow_mut().settle_token(
                token,
                EvaluationOutcome::Exception(JavaScriptException {
                    name: "Error".to_owned(),
                    message: "JavaScript evaluation failed".to_owned(),
                }),
            );
        }
        Ok(())
    }

    fn next_event(&mut self) -> Result<Option<EngineEvent>, EngineError> {
        if self.isolate.is_none() {
            return Err(EngineError::InvalidState);
        }
        let mut state = self.state.borrow_mut();
        if let Some(event) = state.events.pop_front() {
            state.queued_event_bytes = state.queued_event_bytes.saturating_sub(event.payload_len());
            return Ok(Some(event));
        }
        Ok(state.terminal_events.pop_front())
    }

    fn complete_host_request(
        &mut self,
        request_id: HostRequestId,
        completion: HostCompletion,
    ) -> Result<(), EngineError> {
        if request_id.0 == 0 {
            return Err(EngineError::InvalidRequest);
        }
        let promise = self
            .state
            .borrow_mut()
            .pending_promises
            .remove(&request_id.0)
            .ok_or(EngineError::InvalidRequest)?;
        self.enter(|scope| complete_promise(scope, &promise, completion))
    }

    fn perform_microtask_checkpoint(&mut self) -> Result<(), EngineError> {
        self.enter(|scope| {
            scope.perform_microtask_checkpoint();
            Ok(())
        })
    }

    fn cancel_evaluation(&mut self, evaluation_id: EvaluationId) -> Result<(), EngineError> {
        let mut state = self.state.borrow_mut();
        let token = state
            .token_for(evaluation_id.0)
            .ok_or(EngineError::InvalidRequest)?;
        state.settle_token(token, EvaluationOutcome::Cancelled);
        Ok(())
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        let Some(mut isolate) = self.isolate.take() else {
            return Ok(());
        };
        {
            let mut state = self.state.borrow_mut();
            state.shutting_down = true;
            state.pending_promises.clear();
            state.pending_evaluations.clear();
            state.notifier = None;
        }
        self.evaluate.take();
        self.context.take();
        isolate.remove_slot::<Rc<RefCell<BackendState>>>();
        drop(isolate);
        Ok(())
    }
}

impl Drop for V8Backend {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

fn validate_configuration(configuration: EngineConfiguration) -> Result<(), EngineError> {
    if configuration.maximum_pending_evaluations == 0
        || configuration.maximum_pending_host_requests == 0
        || configuration.maximum_source_bytes == 0
        || configuration.maximum_event_bytes == 0
    {
        Err(EngineError::InvalidConfiguration)
    } else {
        Ok(())
    }
}

fn initialize_v8() {
    static INITIALIZE: Once = Once::new();
    INITIALIZE.call_once(|| {
        let platform = v8::new_default_platform(0, false).make_shared();
        v8::V8::initialize_platform(platform);
        v8::V8::initialize();
    });
}

fn install_callback<F>(
    scope: &mut v8::PinScope<'_, '_>,
    name: &str,
    callback: F,
) -> Result<(), EngineError>
where
    F: v8::MapFnTo<v8::FunctionCallback>,
{
    let function = v8::Function::new(scope, callback).ok_or(EngineError::Backend)?;
    let name = v8::String::new(scope, name).ok_or(EngineError::Backend)?;
    let context = scope.get_current_context();
    if context
        .global(scope)
        .set(scope, name.into(), function.into())
        == Some(true)
    {
        Ok(())
    } else {
        Err(EngineError::Backend)
    }
}

fn callback_state(scope: &v8::PinScope<'_, '_>) -> Option<Rc<RefCell<BackendState>>> {
    scope.get_slot::<Rc<RefCell<BackendState>>>().map(Rc::clone)
}

#[allow(clippy::needless_pass_by_value)]
fn evaluation_done(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(token) = exact_u64(scope, arguments.get(0)) else {
        return;
    };
    let outcome = rust_string(scope, arguments.get(1)).map_or_else(
        || {
            EvaluationOutcome::Exception(JavaScriptException {
                name: "Error".to_owned(),
                message: "Unprintable evaluation result".to_owned(),
            })
        },
        |value| EvaluationOutcome::Value(value.into_bytes()),
    );
    if let Some(state) = callback_state(scope) {
        state.borrow_mut().settle_token(token, outcome);
    }
}

#[allow(clippy::needless_pass_by_value)]
fn evaluation_failed(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(token) = exact_u64(scope, arguments.get(0)) else {
        return;
    };
    let name = rust_string(scope, arguments.get(1)).unwrap_or_else(|| "Error".to_owned());
    let message = rust_string(scope, arguments.get(2)).unwrap_or_default();
    if let Some(state) = callback_state(scope) {
        state.borrow_mut().settle_token(
            token,
            EvaluationOutcome::Exception(JavaScriptException { name, message }),
        );
    }
}

#[allow(clippy::needless_pass_by_value)]
fn invoke(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(name) = rust_string(scope, arguments.get(0)) else {
        reject_callback(
            scope,
            arguments.get(3),
            "Invalid operation input",
            "InvalidRequest",
        );
        return;
    };
    let maximum =
        callback_state(scope).map_or(0, |state| state.borrow().configuration.maximum_event_bytes);
    let Some(input) = byte_array(scope, arguments.get(1), maximum) else {
        reject_callback(
            scope,
            arguments.get(3),
            "Invalid operation input",
            "InvalidRequest",
        );
        return;
    };
    submit_host(
        scope,
        HostRequest::Invoke {
            name,
            version: 1,
            input,
        },
        arguments.get(2),
        arguments.get(3),
    );
}

#[allow(clippy::needless_pass_by_value)]
fn sleep(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(milliseconds) = exact_u64(scope, arguments.get(0)) else {
        reject_callback(
            scope,
            arguments.get(2),
            "Invalid timer delay",
            "InvalidRequest",
        );
        return;
    };
    if milliseconds > MAXIMUM_SLEEP_MILLISECONDS {
        reject_callback(
            scope,
            arguments.get(2),
            "Invalid timer delay",
            "InvalidRequest",
        );
        return;
    }
    submit_host(
        scope,
        HostRequest::Sleep {
            nanoseconds: milliseconds * 1_000_000,
        },
        arguments.get(1),
        arguments.get(2),
    );
}

#[allow(clippy::needless_pass_by_value)]
fn read_file(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(url) = rust_string(scope, arguments.get(0)) else {
        reject_callback(
            scope,
            arguments.get(3),
            "Invalid mount read request",
            "InvalidRequest",
        );
        return;
    };
    let Some(maximum_bytes) =
        exact_u64(scope, arguments.get(1)).and_then(|value| usize::try_from(value).ok())
    else {
        reject_callback(
            scope,
            arguments.get(3),
            "Invalid mount read request",
            "InvalidRequest",
        );
        return;
    };
    if maximum_bytes == 0 || url.len() > MAXIMUM_MOUNT_URL_BYTES {
        reject_callback(
            scope,
            arguments.get(3),
            "Invalid mount read request",
            "InvalidRequest",
        );
        return;
    }
    submit_host(
        scope,
        HostRequest::Mount(MountRequest::ReadFile { url, maximum_bytes }),
        arguments.get(2),
        arguments.get(3),
    );
}

#[allow(clippy::needless_pass_by_value)]
fn mount_operation(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(kind) = exact_u64(scope, arguments.get(0)) else {
        reject_callback(
            scope,
            arguments.get(4),
            "Invalid mount operation",
            "InvalidRequest",
        );
        return;
    };
    let Some(first) = rust_string(scope, arguments.get(1)) else {
        reject_callback(
            scope,
            arguments.get(4),
            "Invalid mount operation",
            "InvalidRequest",
        );
        return;
    };
    if first.len() > MAXIMUM_MOUNT_URL_BYTES {
        reject_callback(
            scope,
            arguments.get(4),
            "Invalid mount operation",
            "InvalidRequest",
        );
        return;
    }
    let maximum =
        callback_state(scope).map_or(0, |state| state.borrow().configuration.maximum_event_bytes);
    let request = match kind {
        11 => byte_array(scope, arguments.get(2), maximum)
            .map(|bytes| HostRequest::Mount(MountRequest::WriteFile { url: first, bytes })),
        12 => Some(HostRequest::Mount(MountRequest::CreateDirectory {
            url: first,
        })),
        13 => Some(HostRequest::Mount(MountRequest::RemoveFile { url: first })),
        14 => Some(HostRequest::Mount(MountRequest::RemoveDirectory {
            url: first,
        })),
        15 => rust_string(scope, arguments.get(2))
            .filter(|to| to.len() <= MAXIMUM_MOUNT_URL_BYTES)
            .map(|to| HostRequest::Mount(MountRequest::Rename { from: first, to })),
        _ => None,
    };
    let Some(request) = request else {
        reject_callback(
            scope,
            arguments.get(4),
            "Invalid mount operation",
            "InvalidRequest",
        );
        return;
    };
    submit_host(scope, request, arguments.get(3), arguments.get(4));
}

#[allow(clippy::needless_pass_by_value)]
fn console_output(
    scope: &mut v8::PinScope<'_, '_>,
    arguments: v8::FunctionCallbackArguments,
    _: v8::ReturnValue,
) {
    let Some(message) = rust_string(scope, arguments.get(0)) else {
        return;
    };
    if let Some(state) = callback_state(scope) {
        state
            .borrow_mut()
            .enqueue(EngineEvent::ConsoleOutput(message.into_bytes()));
    }
}

fn submit_host(
    scope: &mut v8::PinScope<'_, '_>,
    request: HostRequest,
    resolve: v8::Local<'_, v8::Value>,
    reject: v8::Local<'_, v8::Value>,
) {
    let Ok(resolve) = v8::Local::<v8::Function>::try_from(resolve) else {
        return;
    };
    let Ok(reject) = v8::Local::<v8::Function>::try_from(reject) else {
        return;
    };
    let Some(state) = callback_state(scope) else {
        return;
    };
    let mut state = state.borrow_mut();
    if state.shutting_down
        || state.pending_promises.len() >= state.configuration.maximum_pending_host_requests
        || state.next_host_id > MAXIMUM_SAFE_INTEGER
    {
        drop(state);
        reject_function(
            scope,
            reject,
            "Host request limit exceeded",
            "QuotaExceeded",
        );
        return;
    }
    let id = state.next_host_id;
    state.next_host_id += 1;
    let event = EngineEvent::HostRequest {
        id: HostRequestId(id),
        request,
    };
    if !state.enqueue(event) {
        drop(state);
        reject_function(
            scope,
            reject,
            "Engine event queue exhausted",
            "QuotaExceeded",
        );
        return;
    }
    state.pending_promises.insert(
        id,
        PendingPromise {
            resolve: v8::Global::new(scope, resolve),
            reject: v8::Global::new(scope, reject),
        },
    );
}

fn complete_promise(
    scope: &mut v8::PinScope<'_, '_>,
    promise: &PendingPromise,
    completion: HostCompletion,
) -> Result<(), EngineError> {
    let receiver = v8::undefined(scope).into();
    match completion {
        HostCompletion::Unit => {
            let resolve = v8::Local::new(scope, &promise.resolve);
            resolve
                .call(scope, receiver, &[])
                .ok_or(EngineError::Backend)?;
        }
        HostCompletion::Bytes(bytes) => {
            let values = bytes
                .into_iter()
                .map(|byte| v8::Integer::new_from_unsigned(scope, u32::from(byte)).into())
                .collect::<Vec<v8::Local<'_, v8::Value>>>();
            let value = v8::Array::new_with_elements(scope, &values).into();
            let resolve = v8::Local::new(scope, &promise.resolve);
            resolve
                .call(scope, receiver, &[value])
                .ok_or(EngineError::Backend)?;
        }
        HostCompletion::Object { id, kind } => {
            let object = v8::Object::new(scope);
            let id_key = v8::Private::for_api(scope, v8::String::new(scope, "zintl.object.id"));
            let kind_key = v8::Private::for_api(scope, v8::String::new(scope, "zintl.object.kind"));
            let id_value = safe_integer_number(scope, id.0).into();
            let kind_value = v8::Integer::new(
                scope,
                match kind {
                    EngineObjectKind::Directory => 1,
                    EngineObjectKind::File => 2,
                },
            )
            .into();
            object.set_private(scope, id_key, id_value);
            object.set_private(scope, kind_key, kind_value);
            let resolve = v8::Local::new(scope, &promise.resolve);
            resolve
                .call(scope, receiver, &[object.into()])
                .ok_or(EngineError::Backend)?;
        }
        HostCompletion::Failed(error) => {
            let reject = v8::Local::new(scope, &promise.reject);
            let value = error_value(scope, &error.message, host_error_name(error.code))?;
            reject
                .call(scope, receiver, &[value])
                .ok_or(EngineError::Backend)?;
        }
    }
    Ok(())
}

fn reject_callback(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
    message: &str,
    code: &str,
) {
    if let Ok(reject) = v8::Local::<v8::Function>::try_from(value) {
        reject_function(scope, reject, message, code);
    }
}

fn reject_function(
    scope: &mut v8::PinScope<'_, '_>,
    reject: v8::Local<'_, v8::Function>,
    message: &str,
    code: &str,
) {
    let Ok(value) = error_value(scope, message, code) else {
        return;
    };
    let receiver = v8::undefined(scope).into();
    let _ = reject.call(scope, receiver, &[value]);
}

fn error_value<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    message: &str,
    code: &str,
) -> Result<v8::Local<'s, v8::Value>, EngineError> {
    let message = v8::String::new(scope, message).ok_or(EngineError::Backend)?;
    let error = v8::Exception::error(scope, message);
    let object = v8::Local::<v8::Object>::try_from(error).map_err(|_| EngineError::Backend)?;
    let key = v8::String::new(scope, "code").ok_or(EngineError::Backend)?;
    let code = v8::String::new(scope, code).ok_or(EngineError::Backend)?;
    object.set(scope, key.into(), code.into());
    Ok(error)
}

fn rust_string(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
) -> Option<String> {
    value
        .to_string(scope)
        .map(|value| value.to_rust_string_lossy(scope))
}

fn exact_u64(scope: &mut v8::PinScope<'_, '_>, value: v8::Local<'_, v8::Value>) -> Option<u64> {
    let value = value.number_value(scope)?;
    if value.is_finite()
        && (0.0..=MAXIMUM_SAFE_INTEGER_F64).contains(&value)
        && value.fract() == 0.0
    {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        Some(value as u64)
    } else {
        None
    }
}

#[allow(clippy::cast_precision_loss)]
fn safe_integer_number<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: u64,
) -> v8::Local<'s, v8::Number> {
    debug_assert!(value <= MAXIMUM_SAFE_INTEGER);
    v8::Number::new(scope, value as f64)
}

fn byte_array(
    scope: &mut v8::PinScope<'_, '_>,
    value: v8::Local<'_, v8::Value>,
    maximum: usize,
) -> Option<Vec<u8>> {
    let array = v8::Local::<v8::Array>::try_from(value).ok()?;
    let length = usize::try_from(array.length()).ok()?;
    if length > maximum {
        return None;
    }
    let mut bytes = Vec::with_capacity(length);
    for index in 0..array.length() {
        let value = array.get_index(scope, index)?.uint32_value(scope)?;
        bytes.push(u8::try_from(value).ok()?);
    }
    Some(bytes)
}

const fn host_error_name(error: HostErrorCode) -> &'static str {
    match error {
        HostErrorCode::PermissionDenied => "PermissionDenied",
        HostErrorCode::InvalidRequest => "InvalidRequest",
        HostErrorCode::QuotaExceeded => "QuotaExceeded",
        HostErrorCode::Cancelled => "Cancelled",
        HostErrorCode::TimedOut => "TimedOut",
        HostErrorCode::ShuttingDown => "ShuttingDown",
        HostErrorCode::OperationFailed => "OperationFailed",
    }
}

#[cfg(test)]
mod tests {
    use super::V8Backend;
    use runtime_engine::{
        EngineConfiguration, EngineError, EngineEvent, EngineNotifier, EngineObjectId,
        EngineObjectKind, EvaluationId, EvaluationOutcome, EvaluationRequest, HostCompletion,
        HostErrorCode, HostFailure, HostRequest, HostRequestId, JavaScriptEngineBackend,
        JavaScriptException, MountRequest,
    };
    use std::sync::Arc;

    struct NoopNotifier;

    impl EngineNotifier for NoopNotifier {
        fn notify(&self) {}
    }

    fn started() -> V8Backend {
        let mut backend = V8Backend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
    }

    fn submit(backend: &mut V8Backend, id: u64, source: &str) -> Result<(), EngineError> {
        backend.submit_evaluation(EvaluationRequest {
            id: EvaluationId(id),
            source: source.to_owned(),
        })
    }

    fn drain(backend: &mut V8Backend) -> Vec<EngineEvent> {
        backend.perform_microtask_checkpoint().expect("checkpoint");
        std::iter::from_fn(|| backend.next_event().expect("event")).collect()
    }

    fn settled(id: u64, outcome: EvaluationOutcome) -> EngineEvent {
        EngineEvent::EvaluationSettled {
            id: EvaluationId(id),
            outcome,
        }
    }

    fn exception(name: &str, message: &str) -> EvaluationOutcome {
        EvaluationOutcome::Exception(JavaScriptException {
            name: name.to_owned(),
            message: message.to_owned(),
        })
    }

    fn host_request(events: &[EngineEvent]) -> (HostRequestId, HostRequest) {
        events
            .iter()
            .find_map(|event| match event {
                EngineEvent::HostRequest { id, request } => Some((*id, request.clone())),
                _ => None,
            })
            .expect("host request")
    }

    #[test]
    // Verifies non-Error thrown values settle once as exceptions instead of failing submission.
    fn thrown_primitives_settle_as_exceptions() {
        let mut backend = started();
        submit(&mut backend, 1, "throw null").expect("submit");
        submit(&mut backend, 2, "throw 'boom'").expect("submit");
        submit(&mut backend, 3, "Promise.reject(undefined)").expect("submit");
        assert_eq!(
            drain(&mut backend),
            vec![
                settled(1, exception("Error", "null")),
                settled(2, exception("Error", "boom")),
                settled(3, exception("Error", "undefined")),
            ]
        );
    }

    #[test]
    // Verifies a colon in an exception message is not mistaken for the name separator.
    fn exception_message_keeps_colons() {
        let mut backend = started();
        submit(&mut backend, 1, "throw new TypeError('see http://x')").expect("submit");
        assert_eq!(
            drain(&mut backend),
            vec![settled(1, exception("TypeError", "see http://x"))]
        );
    }

    #[test]
    // Verifies results JSON cannot encode still settle instead of leaking a pending slot.
    fn unserializable_results_settle_as_exceptions() {
        let mut backend = started();
        submit(&mut backend, 1, "1n").expect("submit");
        submit(&mut backend, 2, "const a = {}; a.self = a; a").expect("submit");
        let events = drain(&mut backend);
        assert_eq!(events.len(), 2);
        for event in events {
            assert!(matches!(
                event,
                EngineEvent::EvaluationSettled {
                    outcome: EvaluationOutcome::Exception(JavaScriptException { ref name, .. }),
                    ..
                } if name == "TypeError"
            ));
        }
    }

    #[test]
    // Verifies user code replacing globals cannot stop later evaluations from settling.
    fn replaced_globals_do_not_break_the_bridge() {
        let mut backend = started();
        submit(
            &mut backend,
            1,
            "Promise.resolve = () => ({ then() {} }); JSON.stringify = () => { throw 1 }; \
             Array.from = null; globalThis.eval = null; Uint8Array = null; 0",
        )
        .expect("submit");
        submit(&mut backend, 2, "Zintl.invoke('dev.zintl.echo')").expect("submit");
        let events = drain(&mut backend);
        assert!(events.contains(&settled(
            1,
            EvaluationOutcome::Value(br#"{"type":"value","value":0}"#.to_vec())
        )));
        let (request_id, _) = host_request(&events);
        backend
            .complete_host_request(request_id, HostCompletion::Bytes(vec![7]))
            .expect("complete");
        assert_eq!(
            drain(&mut backend),
            vec![settled(
                2,
                EvaluationOutcome::Value(br#"{"type":"bytes","value":[7]}"#.to_vec())
            )]
        );
    }

    #[test]
    // Verifies a duplicate ID is rejected without disturbing the evaluation that owns it.
    fn duplicate_id_keeps_the_original_evaluation() {
        let mut backend = started();
        submit(&mut backend, 7, "Zintl.sleep(1).then(() => 'first')").expect("submit");
        assert_eq!(
            submit(&mut backend, 7, "'second'"),
            Err(EngineError::InvalidRequest)
        );
        let (request_id, _) = host_request(&drain(&mut backend));
        backend
            .complete_host_request(request_id, HostCompletion::Unit)
            .expect("complete");
        assert_eq!(
            drain(&mut backend),
            vec![settled(
                7,
                EvaluationOutcome::Value(br#"{"type":"value","value":"first"}"#.to_vec())
            )]
        );
    }

    #[test]
    // Verifies cancellation settles immediately and a late JavaScript result cannot reach a reused ID.
    fn cancel_settles_immediately_and_ignores_late_results() {
        let mut backend = started();
        submit(&mut backend, 1, "new Promise(() => {})").expect("submit");
        backend.cancel_evaluation(EvaluationId(1)).expect("cancel");
        assert_eq!(
            drain(&mut backend),
            vec![settled(1, EvaluationOutcome::Cancelled)]
        );
        assert_eq!(
            backend.cancel_evaluation(EvaluationId(1)),
            Err(EngineError::InvalidRequest)
        );

        submit(&mut backend, 2, "Zintl.sleep(1).then(() => 'late')").expect("submit");
        let (request_id, _) = host_request(&drain(&mut backend));
        backend.cancel_evaluation(EvaluationId(2)).expect("cancel");
        submit(&mut backend, 2, "new Promise(() => {})").expect("reuse");
        backend
            .complete_host_request(request_id, HostCompletion::Unit)
            .expect("complete");
        assert_eq!(
            drain(&mut backend),
            vec![settled(2, EvaluationOutcome::Cancelled)]
        );
    }

    #[test]
    // Verifies submitting before start fails without reserving the evaluation ID.
    fn submit_before_start_does_not_reserve_the_id() {
        let mut backend = V8Backend::new();
        assert_eq!(submit(&mut backend, 1, "1"), Err(EngineError::InvalidState));
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        submit(&mut backend, 1, "1").expect("submit");
        assert_eq!(drain(&mut backend).len(), 1);
    }

    #[test]
    // Verifies an opaque object completion resolves the JavaScript Promise.
    fn object_completion_resolves() {
        let mut backend = started();
        submit(
            &mut backend,
            1,
            "Zintl.invoke('x').then(() => 'resolved', e => e.code)",
        )
        .expect("submit");
        let (request_id, _) = host_request(&drain(&mut backend));
        backend
            .complete_host_request(
                request_id,
                HostCompletion::Object {
                    id: EngineObjectId(5),
                    kind: EngineObjectKind::File,
                },
            )
            .expect("complete");
        assert_eq!(
            drain(&mut backend),
            vec![settled(
                1,
                EvaluationOutcome::Value(br#"{"type":"value","value":"resolved"}"#.to_vec())
            )]
        );
    }

    #[test]
    // Verifies a lone surrogate is written as U+FFFD instead of invalid UTF-8.
    fn write_file_replaces_lone_surrogates() {
        let mut backend = started();
        submit(
            &mut backend,
            1,
            "Zintl.writeFile('mount://p/a', 'a\\uD800b')",
        )
        .expect("submit");
        let (_, request) = host_request(&drain(&mut backend));
        assert_eq!(
            request,
            HostRequest::Mount(MountRequest::WriteFile {
                url: "mount://p/a".to_owned(),
                bytes: "a\u{FFFD}b".as_bytes().to_vec(),
            })
        );
    }

    #[test]
    // Verifies V8 evaluates a Promise while the host controls the microtask checkpoint.
    fn evaluates_promise_at_explicit_checkpoint() {
        let mut backend = V8Backend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "Promise.resolve(6 * 7)".to_owned(),
            })
            .expect("submit");
        assert_eq!(backend.next_event().expect("event"), None);
        backend.perform_microtask_checkpoint().expect("checkpoint");
        assert_eq!(
            backend.next_event().expect("event"),
            Some(EngineEvent::EvaluationSettled {
                id: EvaluationId(1),
                outcome: EvaluationOutcome::Value(br#"{"type":"value","value":42}"#.to_vec()),
            })
        );
    }

    #[test]
    // Verifies console output and the terminal result preserve their enqueue order.
    fn emits_console_before_settlement() {
        let mut backend = V8Backend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "console.log('hello'); 7".to_owned(),
            })
            .expect("submit");
        backend.perform_microtask_checkpoint().expect("checkpoint");
        assert_eq!(
            backend.next_event().expect("event"),
            Some(EngineEvent::ConsoleOutput(b"hello".to_vec()))
        );
        assert!(matches!(
            backend.next_event().expect("event"),
            Some(EngineEvent::EvaluationSettled { .. })
        ));
    }

    #[test]
    // Verifies a host failure preserves both its stable code and sanitized message in JavaScript.
    fn exposes_structured_host_failure() {
        let mut backend = V8Backend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "Zintl.invoke('dev.zintl.fail').catch(error => ({ code: error.code, message: error.message }))".to_owned(),
            })
            .expect("submit");
        let request_id = match backend.next_event().expect("event") {
            Some(EngineEvent::HostRequest { id, .. }) => id,
            event => panic!("expected host request, got {event:?}"),
        };
        backend
            .complete_host_request(
                request_id,
                HostCompletion::Failed(HostFailure {
                    code: HostErrorCode::InvalidRequest,
                    message: "Mount not found".to_owned(),
                }),
            )
            .expect("complete");
        backend.perform_microtask_checkpoint().expect("checkpoint");
        assert_eq!(
            backend.next_event().expect("event"),
            Some(EngineEvent::EvaluationSettled {
                id: EvaluationId(1),
                outcome: EvaluationOutcome::Value(
                    br#"{"type":"value","value":{"code":"InvalidRequest","message":"Mount not found"}}"#
                        .to_vec(),
                ),
            })
        );
    }
}
