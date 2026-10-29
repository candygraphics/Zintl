//! Message-loop-owned JavaScript host with no OS event loop or I/O resources.

#![forbid(unsafe_code)]

pub use runtime_engine::{
    DriveBudget, EngineConfiguration, EngineError, EngineEvent, EngineNotifier, EvaluationId,
    EvaluationOutcome, EvaluationRequest, HostCompletion, HostRequest, HostRequestId,
    JavaScriptEngineBackend,
};
use std::collections::VecDeque;
use std::sync::Arc;

/// Engine work observed during one bounded host turn.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HostTurn {
    /// Events moved into the host queue.
    pub events: usize,
    /// Payload bytes of those events.
    pub payload_bytes: usize,
    /// Whether the budget ended the turn before the backend ran out of events.
    pub has_more: bool,
}

/// Builder for a message-loop-owned [`ZjsHost`].
pub struct ZjsHostBuilder {
    backend: Box<dyn JavaScriptEngineBackend>,
    configuration: EngineConfiguration,
    notifier: Arc<dyn EngineNotifier>,
}

impl ZjsHostBuilder {
    #[must_use]
    pub fn new(
        backend: Box<dyn JavaScriptEngineBackend>,
        notifier: Arc<dyn EngineNotifier>,
    ) -> Self {
        Self {
            backend,
            configuration: EngineConfiguration::default(),
            notifier,
        }
    }

    #[must_use]
    pub const fn configuration(mut self, configuration: EngineConfiguration) -> Self {
        self.configuration = configuration;
        self
    }

    /// Starts the engine and binds it to a new host.
    ///
    /// # Errors
    /// Returns invalid configuration or backend startup failures.
    pub fn build(mut self) -> Result<ZjsHost, EngineError> {
        self.backend
            .start(self.configuration, self.notifier.clone())?;
        Ok(ZjsHost {
            backend: self.backend,
            events: VecDeque::new(),
            deferred: None,
        })
    }
}

/// Owns only the JavaScript engine and its engine-local runtime state.
///
/// The non-`Send` backend keeps this value on its owning message-loop thread.
pub struct ZjsHost {
    backend: Box<dyn JavaScriptEngineBackend>,
    events: VecDeque<EngineEvent>,
    deferred: Option<EngineEvent>,
}

impl ZjsHost {
    /// Enters JavaScript to evaluate source and immediately reaches a microtask checkpoint.
    ///
    /// # Errors
    /// Returns an engine lifecycle, quota, or evaluation submission error.
    pub fn evaluate(&mut self, request: EvaluationRequest) -> Result<(), EngineError> {
        self.backend.submit_evaluation(request)?;
        self.perform_microtask_checkpoint()
    }

    /// Resolves or rejects one engine-owned Promise, then drains its microtasks.
    ///
    /// # Errors
    /// Returns an error for an unknown request or failed engine entry.
    pub fn complete_host_request(
        &mut self,
        request_id: HostRequestId,
        completion: HostCompletion,
    ) -> Result<(), EngineError> {
        self.backend.complete_host_request(request_id, completion)?;
        self.perform_microtask_checkpoint()
    }

    /// Runs Promise jobs and `queueMicrotask` in the engine's single logical queue.
    ///
    /// # Errors
    /// Returns a backend checkpoint failure.
    pub fn perform_microtask_checkpoint(&mut self) -> Result<(), EngineError> {
        self.backend.perform_microtask_checkpoint()
    }

    /// Moves a bounded number of already-produced engine events into host state.
    ///
    /// An event larger than the byte budget is delivered alone in its own turn,
    /// so the budget never drops an event.
    ///
    /// # Errors
    /// Reports malformed backend events or unavailable backend state.
    pub fn drain_events(&mut self, budget: DriveBudget) -> Result<HostTurn, EngineError> {
        let mut turn = HostTurn::default();
        while turn.events < budget.maximum_events {
            let event = match self.deferred.take() {
                Some(event) => event,
                None => match self.backend.next_event()? {
                    Some(event) => event,
                    None => break,
                },
            };
            let bytes = event.payload_len();
            if turn.events > 0 && bytes.saturating_add(turn.payload_bytes) > budget.maximum_bytes {
                self.deferred = Some(event);
                turn.has_more = true;
                break;
            }
            turn.events += 1;
            turn.payload_bytes += bytes;
            self.events.push_back(event);
        }
        turn.has_more |= turn.events == budget.maximum_events || self.deferred.is_some();
        Ok(turn)
    }

    /// Pops the next event a previous [`Self::drain_events`] moved into the host.
    #[must_use]
    pub fn next_event(&mut self) -> Option<EngineEvent> {
        self.events.pop_front()
    }

    /// Cancels a pending evaluation.
    ///
    /// # Errors
    /// Returns an error for an unknown evaluation or unavailable engine.
    pub fn cancel_evaluation(&mut self, id: EvaluationId) -> Result<(), EngineError> {
        self.backend.cancel_evaluation(id)
    }

    /// Releases all engine-owned state.
    ///
    /// # Errors
    /// Returns a sanitized backend shutdown failure.
    pub fn shutdown(&mut self) -> Result<(), EngineError> {
        self.backend.shutdown()
    }
}

impl Drop for ZjsHost {
    fn drop(&mut self) {
        let _ = self.backend.shutdown();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct NoopNotifier;
    impl EngineNotifier for NoopNotifier {
        fn notify(&self) {}
    }

    #[derive(Default)]
    struct TraceBackend {
        trace: Arc<Mutex<Vec<&'static str>>>,
        events: VecDeque<EngineEvent>,
    }

    impl JavaScriptEngineBackend for TraceBackend {
        fn start(
            &mut self,
            _: EngineConfiguration,
            _: Arc<dyn EngineNotifier>,
        ) -> Result<(), EngineError> {
            self.trace.lock().unwrap().push("start");
            Ok(())
        }
        fn submit_evaluation(&mut self, _: EvaluationRequest) -> Result<(), EngineError> {
            self.trace.lock().unwrap().push("evaluate");
            Ok(())
        }
        fn next_event(&mut self) -> Result<Option<EngineEvent>, EngineError> {
            Ok(self.events.pop_front())
        }
        fn complete_host_request(
            &mut self,
            _: HostRequestId,
            _: HostCompletion,
        ) -> Result<(), EngineError> {
            self.trace.lock().unwrap().push("complete");
            Ok(())
        }
        fn perform_microtask_checkpoint(&mut self) -> Result<(), EngineError> {
            self.trace.lock().unwrap().push("microtasks");
            Ok(())
        }
        fn cancel_evaluation(&mut self, _: EvaluationId) -> Result<(), EngineError> {
            Ok(())
        }
        fn shutdown(&mut self) -> Result<(), EngineError> {
            self.trace.lock().unwrap().push("shutdown");
            Ok(())
        }
    }

    #[test]
    fn javascript_entries_end_with_a_microtask_checkpoint() {
        // Verifies evaluation and Promise completion preserve task/microtask ordering.
        let trace = Arc::new(Mutex::new(Vec::new()));
        let backend = TraceBackend {
            trace: trace.clone(),
            events: VecDeque::new(),
        };
        let mut host = ZjsHostBuilder::new(Box::new(backend), Arc::new(NoopNotifier))
            .build()
            .unwrap();
        host.evaluate(EvaluationRequest {
            id: EvaluationId(1),
            source: "1".into(),
        })
        .unwrap();
        host.complete_host_request(HostRequestId(1), HostCompletion::Unit)
            .unwrap();
        assert_eq!(
            *trace.lock().unwrap(),
            vec!["start", "evaluate", "microtasks", "complete", "microtasks"]
        );
    }

    #[test]
    fn oversized_event_is_delivered_alone_instead_of_dropped() {
        // Verifies an event over the byte budget still reaches the host, one per turn.
        let console = |text: &str| EngineEvent::ConsoleOutput(text.as_bytes().to_vec());
        let backend = TraceBackend {
            events: VecDeque::from([console("small"), console(&"x".repeat(100)), console("tail")]),
            ..TraceBackend::default()
        };
        let mut host = ZjsHostBuilder::new(Box::new(backend), Arc::new(NoopNotifier))
            .build()
            .unwrap();
        let budget = DriveBudget::new(8, 10).unwrap();

        let turn = host.drain_events(budget).unwrap();
        assert_eq!((turn.events, turn.has_more), (1, true));
        assert_eq!(host.next_event(), Some(console("small")));

        let turn = host.drain_events(budget).unwrap();
        assert_eq!(
            (turn.events, turn.payload_bytes, turn.has_more),
            (1, 100, true)
        );
        assert_eq!(host.next_event(), Some(console(&"x".repeat(100))));

        let turn = host.drain_events(budget).unwrap();
        assert_eq!((turn.events, turn.has_more), (1, false));
        assert_eq!(host.next_event(), Some(console("tail")));
    }
}
