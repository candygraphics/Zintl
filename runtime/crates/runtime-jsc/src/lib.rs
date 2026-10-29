//! `JavaScriptCore` implementation of the engine-neutral Rust backend contract.
//!
//! Swift owns `JavaScriptCore` and every engine value on the `ZjsHost` thread.
//! This crate exposes only the safe [`JavaScriptCoreBackend`]; its opaque FFI
//! pointer and unsafe calls remain private.

#![warn(missing_docs)]

use runtime_engine::{
    EngineConfiguration, EngineError, EngineEvent, EngineNotifier, EngineObjectKind, EvaluationId,
    EvaluationRequest, HostCompletion, HostErrorCode, HostRequestId, JavaScriptEngineBackend,
};
use std::sync::Arc;

/// `JavaScriptCore` backend implemented by the statically linked Swift adapter.
#[cfg(target_os = "macos")]
pub struct JavaScriptCoreBackend {
    engine: Option<std::ptr::NonNull<std::ffi::c_void>>,
    notifier: Option<Box<NotifierState>>,
    event_buffer: Vec<u8>,
}

#[cfg(target_os = "macos")]
struct NotifierState {
    notifier: Arc<dyn EngineNotifier>,
}

#[cfg(target_os = "macos")]
impl JavaScriptCoreBackend {
    /// Creates an unstarted `JavaScriptCore` backend.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            engine: None,
            notifier: None,
            event_buffer: Vec::new(),
        }
    }
}

#[cfg(target_os = "macos")]
impl Default for JavaScriptCoreBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_os = "macos")]
impl JavaScriptEngineBackend for JavaScriptCoreBackend {
    fn start(
        &mut self,
        configuration: EngineConfiguration,
        notifier: Arc<dyn EngineNotifier>,
    ) -> Result<(), EngineError> {
        if self.engine.is_some() {
            return Err(EngineError::InvalidState);
        }
        let mut state = Box::new(NotifierState { notifier });
        let user_data = std::ptr::from_mut(state.as_mut()).cast();
        let mut engine = std::ptr::null_mut();
        // SAFETY: all scalar limits are checked by Swift; `state` remains boxed
        // until after engine shutdown/free; `engine` is a valid out pointer.
        let status = unsafe {
            ffi::zjsc_engine_new(
                u32_limit(configuration.maximum_pending_evaluations)?,
                u32_limit(configuration.maximum_pending_host_requests)?,
                u32_limit(configuration.maximum_source_bytes)?,
                u32_limit(configuration.maximum_event_bytes)?,
                Some(notify_bridge),
                user_data,
                &raw mut engine,
            )
        };
        status_result(status)?;
        let engine = std::ptr::NonNull::new(engine).ok_or(EngineError::Backend)?;
        // SAFETY: `engine` was returned live by `zjsc_engine_new`.
        if let Err(error) = status_result(unsafe { ffi::zjsc_engine_start(engine.as_ptr()) }) {
            // SAFETY: startup failed but ownership remains with this caller.
            unsafe { ffi::zjsc_engine_free(engine.as_ptr()) };
            return Err(error);
        }
        self.engine = Some(engine);
        self.notifier = Some(state);
        Ok(())
    }

    fn submit_evaluation(&mut self, request: EvaluationRequest) -> Result<(), EngineError> {
        let engine = self.engine.ok_or(EngineError::InvalidState)?;
        // SAFETY: engine is live and Swift copies the source during this call.
        status_result(unsafe {
            ffi::zjsc_engine_submit(
                engine.as_ptr(),
                request.id.0,
                request.source.as_ptr(),
                request.source.len(),
            )
        })
    }

    fn next_event(&mut self) -> Result<Option<EngineEvent>, EngineError> {
        let engine = self.engine.ok_or(EngineError::InvalidState)?;
        let mut required = 0;
        // SAFETY: engine and required out pointer are live; null/zero is the
        // documented non-consuming size query.
        let status = unsafe {
            ffi::zjsc_engine_next_event(engine.as_ptr(), std::ptr::null_mut(), 0, &raw mut required)
        };
        if status == ffi::EMPTY {
            return Ok(None);
        }
        if status != ffi::BUFFER_TOO_SMALL || required == 0 {
            return Err(map_status(status));
        }
        self.event_buffer.resize(required, 0);
        let mut written = 0;
        // SAFETY: buffer is writable for its capacity and the engine remains live.
        status_result(unsafe {
            ffi::zjsc_engine_next_event(
                engine.as_ptr(),
                self.event_buffer.as_mut_ptr(),
                self.event_buffer.len(),
                &raw mut written,
            )
        })?;
        if written != required {
            return Err(EngineError::Backend);
        }
        runtime_engine::wire::decode_event(&self.event_buffer).map(Some)
    }

    fn complete_host_request(
        &mut self,
        request_id: HostRequestId,
        completion: HostCompletion,
    ) -> Result<(), EngineError> {
        let engine = self.engine.ok_or(EngineError::InvalidState)?;
        let (kind, object_id, payload) = encode_completion(completion);
        // SAFETY: engine is live and Swift copies payload bytes during the call.
        status_result(unsafe {
            ffi::zjsc_engine_complete(
                engine.as_ptr(),
                request_id.0,
                kind,
                object_id,
                payload.as_ptr(),
                payload.len(),
            )
        })
    }

    fn perform_microtask_checkpoint(&mut self) -> Result<(), EngineError> {
        let engine = self.engine.ok_or(EngineError::InvalidState)?;
        // SAFETY: engine is live and the host calls every engine entry point on
        // its owning message-loop thread.
        status_result(unsafe { ffi::zjsc_engine_microtask_checkpoint(engine.as_ptr()) })
    }

    fn cancel_evaluation(&mut self, evaluation_id: EvaluationId) -> Result<(), EngineError> {
        let engine = self.engine.ok_or(EngineError::InvalidState)?;
        // SAFETY: engine is live and evaluation ID is an inert scalar.
        status_result(unsafe { ffi::zjsc_engine_cancel(engine.as_ptr(), evaluation_id.0) })
    }

    fn shutdown(&mut self) -> Result<(), EngineError> {
        let Some(engine) = self.engine.take() else {
            return Ok(());
        };
        // SAFETY: engine is uniquely owned; shutdown is idempotent and free is
        // called only after its serial queue has released JavaScriptCore state.
        let result = status_result(unsafe { ffi::zjsc_engine_shutdown(engine.as_ptr()) });
        // SAFETY: `engine` was taken above, so this is its only free.
        unsafe { ffi::zjsc_engine_free(engine.as_ptr()) };
        self.notifier.take();
        result
    }
}

#[cfg(target_os = "macos")]
impl Drop for JavaScriptCoreBackend {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[cfg(target_os = "macos")]
unsafe extern "C" fn notify_bridge(user_data: *mut std::ffi::c_void) {
    if user_data.is_null() {
        return;
    }
    // SAFETY: Swift invokes this only while the boxed NotifierState supplied at
    // engine creation remains alive. The callback performs wake notification only.
    let state = unsafe { &*user_data.cast::<NotifierState>() };
    state.notifier.notify();
}

/// Compile-time unsupported JavaScriptCore backend for non-macOS targets.
#[cfg(not(target_os = "macos"))]
#[derive(Default)]
pub struct JavaScriptCoreBackend;

#[cfg(not(target_os = "macos"))]
impl JavaScriptCoreBackend {
    /// Creates an unsupported backend placeholder.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[cfg(not(target_os = "macos"))]
impl JavaScriptEngineBackend for JavaScriptCoreBackend {
    fn start(
        &mut self,
        _: EngineConfiguration,
        _: Arc<dyn EngineNotifier>,
    ) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn submit_evaluation(&mut self, _: EvaluationRequest) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn next_event(&mut self) -> Result<Option<EngineEvent>, EngineError> {
        Err(EngineError::Unsupported)
    }
    fn complete_host_request(
        &mut self,
        _: HostRequestId,
        _: HostCompletion,
    ) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn perform_microtask_checkpoint(&mut self) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn cancel_evaluation(&mut self, _: EvaluationId) -> Result<(), EngineError> {
        Err(EngineError::Unsupported)
    }
    fn shutdown(&mut self) -> Result<(), EngineError> {
        Ok(())
    }
}

#[cfg(target_os = "macos")]
fn encode_completion(completion: HostCompletion) -> (u32, u64, Vec<u8>) {
    match completion {
        HostCompletion::Unit => (0, 0, Vec::new()),
        HostCompletion::Bytes(bytes) => (1, 0, bytes),
        HostCompletion::Object { id, kind } => (
            match kind {
                EngineObjectKind::Directory => 2,
                EngineObjectKind::File => 3,
            },
            id.0,
            Vec::new(),
        ),
        HostCompletion::Failed(error) => {
            let code = host_error_name(error.code).as_bytes();
            let mut payload = Vec::with_capacity(2 + code.len() + error.message.len());
            payload.extend_from_slice(
                &u16::try_from(code.len())
                    .expect("host error names fit in u16")
                    .to_be_bytes(),
            );
            payload.extend_from_slice(code);
            payload.extend_from_slice(error.message.as_bytes());
            (4, 0, payload)
        }
    }
}

#[cfg(target_os = "macos")]
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

#[cfg(target_os = "macos")]
fn u32_limit(value: usize) -> Result<u32, EngineError> {
    u32::try_from(value).map_err(|_| EngineError::InvalidConfiguration)
}

#[cfg(target_os = "macos")]
fn status_result(status: u32) -> Result<(), EngineError> {
    if status == ffi::OK {
        Ok(())
    } else {
        Err(map_status(status))
    }
}

#[cfg(target_os = "macos")]
const fn map_status(status: u32) -> EngineError {
    match status {
        ffi::INVALID_ARGUMENT => EngineError::InvalidRequest,
        ffi::INVALID_STATE => EngineError::InvalidState,
        ffi::QUOTA_EXCEEDED => EngineError::QuotaExceeded,
        _ => EngineError::Backend,
    }
}

#[cfg(target_os = "macos")]
mod ffi {
    use std::ffi::c_void;

    pub const OK: u32 = 0;
    pub const EMPTY: u32 = 1;
    pub const BUFFER_TOO_SMALL: u32 = 2;
    pub const INVALID_ARGUMENT: u32 = 3;
    pub const INVALID_STATE: u32 = 4;
    pub const QUOTA_EXCEEDED: u32 = 5;

    unsafe extern "C" {
        pub fn zjsc_engine_new(
            max_evaluations: u32,
            max_host_requests: u32,
            max_source_bytes: u32,
            max_event_bytes: u32,
            notify: Option<unsafe extern "C" fn(*mut c_void)>,
            user_data: *mut c_void,
            out_engine: *mut *mut c_void,
        ) -> u32;
        pub fn zjsc_engine_start(engine: *mut c_void) -> u32;
        pub fn zjsc_engine_submit(
            engine: *mut c_void,
            evaluation_id: u64,
            source: *const u8,
            source_len: usize,
        ) -> u32;
        pub fn zjsc_engine_next_event(
            engine: *mut c_void,
            output: *mut u8,
            capacity: usize,
            out_required: *mut usize,
        ) -> u32;
        pub fn zjsc_engine_complete(
            engine: *mut c_void,
            request_id: u64,
            completion_kind: u32,
            object_id: u64,
            payload: *const u8,
            payload_len: usize,
        ) -> u32;
        pub fn zjsc_engine_microtask_checkpoint(engine: *mut c_void) -> u32;
        pub fn zjsc_engine_cancel(engine: *mut c_void, evaluation_id: u64) -> u32;
        pub fn zjsc_engine_shutdown(engine: *mut c_void) -> u32;
        pub fn zjsc_engine_free(engine: *mut c_void);
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::JavaScriptCoreBackend;
    use runtime_engine::{
        EngineConfiguration, EngineError, EngineEvent, EngineNotifier, EngineObjectId,
        EngineObjectKind, EvaluationId, EvaluationOutcome, EvaluationRequest, HostCompletion,
        HostErrorCode, HostFailure, HostRequest, HostRequestId, JavaScriptEngineBackend,
        JavaScriptException, MountRequest,
    };
    use std::sync::Arc;
    use std::thread;
    use std::time::{Duration, Instant};

    struct NoopNotifier;

    impl EngineNotifier for NoopNotifier {
        fn notify(&self) {}
    }

    fn started() -> JavaScriptCoreBackend {
        let mut backend = JavaScriptCoreBackend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
    }

    fn submit(
        backend: &mut JavaScriptCoreBackend,
        id: u64,
        source: &str,
    ) -> Result<(), EngineError> {
        backend.submit_evaluation(EvaluationRequest {
            id: EvaluationId(id),
            source: source.to_owned(),
        })
    }

    fn drain(backend: &mut JavaScriptCoreBackend) -> Vec<EngineEvent> {
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

    fn value(json: &str) -> EvaluationOutcome {
        EvaluationOutcome::Value(json.as_bytes().to_vec())
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
    // Verifies non-Error thrown values settle as exceptions with a readable message.
    fn thrown_primitives_settle_as_exceptions() {
        let mut backend = started();
        submit(&mut backend, 1, "throw null").expect("submit");
        submit(&mut backend, 2, "throw 'boom'").expect("submit");
        assert_eq!(
            drain(&mut backend),
            vec![
                settled(1, exception("Error", "null")),
                settled(2, exception("Error", "boom")),
            ]
        );
    }

    #[test]
    // Verifies a colon in an exception message crosses the wire unchanged.
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
        submit(&mut backend, 2, "'still works'").expect("submit");
        assert_eq!(
            drain(&mut backend),
            vec![
                settled(1, value(r#"{"type":"value","value":0}"#)),
                settled(2, value(r#"{"type":"value","value":"still works"}"#)),
            ]
        );
    }

    #[test]
    // Verifies a duplicate ID is rejected without dropping the evaluation that owns it.
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
            vec![settled(7, value(r#"{"type":"value","value":"first"}"#))]
        );
    }

    #[test]
    // Verifies cancellation settles immediately and a late JavaScript result cannot reach a reused ID.
    fn cancel_settles_immediately_and_ignores_late_results() {
        let mut backend = started();
        submit(&mut backend, 1, "Zintl.sleep(1).then(() => 'late')").expect("submit");
        let (request_id, _) = host_request(&drain(&mut backend));
        backend.cancel_evaluation(EvaluationId(1)).expect("cancel");
        assert_eq!(
            drain(&mut backend),
            vec![settled(1, EvaluationOutcome::Cancelled)]
        );
        submit(&mut backend, 1, "new Promise(() => {})").expect("reuse");
        backend
            .complete_host_request(request_id, HostCompletion::Unit)
            .expect("complete");
        assert_eq!(drain(&mut backend), vec![]);
    }

    #[test]
    // Verifies an opaque object completion resolves the Promise with an object.
    fn object_completion_resolves() {
        let mut backend = started();
        submit(
            &mut backend,
            1,
            "Zintl.invoke('x').then(o => typeof o, e => e.code)",
        )
        .expect("submit");
        let (request_id, _) = host_request(&drain(&mut backend));
        backend
            .complete_host_request(
                request_id,
                HostCompletion::Object {
                    id: EngineObjectId(5),
                    kind: EngineObjectKind::Directory,
                },
            )
            .expect("complete");
        assert_eq!(
            drain(&mut backend),
            vec![settled(1, value(r#"{"type":"value","value":"object"}"#))]
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
    // Verifies Rust can evaluate a Promise through the statically linked Swift/JSC backend.
    fn evaluates_promise_through_swift_ffi() {
        let mut backend = JavaScriptCoreBackend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "Promise.resolve(6 * 7)".to_owned(),
            })
            .expect("submit");
        let deadline = Instant::now() + Duration::from_secs(2);
        let outcome = loop {
            if let Some(EngineEvent::EvaluationSettled { outcome, .. }) =
                backend.next_event().expect("event")
            {
                break outcome;
            }
            assert!(Instant::now() < deadline, "evaluation timed out");
            thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(
            outcome,
            EvaluationOutcome::Value(br#"{"type":"value","value":42}"#.to_vec())
        );
        backend.shutdown().expect("shutdown");
    }

    #[test]
    // Verifies JavaScript console output crosses the bounded engine event protocol.
    fn emits_console_output_through_swift_ffi() {
        let mut backend = JavaScriptCoreBackend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "console.debug('hello', {value: 42}); 7".to_owned(),
            })
            .expect("submit");
        let deadline = Instant::now() + Duration::from_secs(2);
        let mut console = None;
        let mut settled = false;
        while !settled {
            match backend.next_event().expect("event") {
                Some(EngineEvent::ConsoleOutput(bytes)) => console = Some(bytes),
                Some(EngineEvent::EvaluationSettled { .. }) => settled = true,
                Some(EngineEvent::HostRequest { .. }) | None => {}
            }
            assert!(Instant::now() < deadline, "evaluation timed out");
            thread::sleep(Duration::from_millis(1));
        }
        assert_eq!(console, Some(br#"hello {"value":42}"#.to_vec()));
        backend.shutdown().expect("shutdown");
    }

    #[test]
    // Verifies the Swift bridge reconstructs a host failure's stable code and sanitized message.
    fn exposes_structured_host_failure_through_swift_ffi() {
        let mut backend = JavaScriptCoreBackend::new();
        backend
            .start(EngineConfiguration::default(), Arc::new(NoopNotifier))
            .expect("start");
        backend
            .submit_evaluation(EvaluationRequest {
                id: EvaluationId(1),
                source: "Zintl.invoke('dev.zintl.fail').catch(error => ({ code: error.code, message: error.message }))".to_owned(),
            })
            .expect("submit");
        let deadline = Instant::now() + Duration::from_secs(2);
        let request_id = loop {
            if let Some(EngineEvent::HostRequest { id, .. }) = backend.next_event().expect("event")
            {
                break id;
            }
            assert!(Instant::now() < deadline, "host request timed out");
            thread::sleep(Duration::from_millis(1));
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
        let outcome = loop {
            if let Some(EngineEvent::EvaluationSettled { outcome, .. }) =
                backend.next_event().expect("event")
            {
                break outcome;
            }
            assert!(Instant::now() < deadline, "evaluation timed out");
            thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(
            outcome,
            EvaluationOutcome::Value(
                br#"{"type":"value","value":{"code":"InvalidRequest","message":"Mount not found"}}"#
                    .to_vec(),
            )
        );
        backend.shutdown().expect("shutdown");
    }
}
