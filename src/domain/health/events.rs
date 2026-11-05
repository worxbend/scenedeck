//! Connection and output incidents retain evidence until recovery or acknowledgement.
use super::*;
use crate::domain::output::{OutputKind, OutputRunState, OutputStatus};
const STOP_INTENT_LIMIT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default)]
pub(super) struct OutputWatch {
    seen_active: bool,
    observed_active: Option<bool>,
    expected_stop: Option<Instant>,
}
impl OutputWatch {
    fn observe(&mut self, status: &OutputStatus, at: Instant) -> bool {
        use OutputRunState::*;
        self.observed_active = match status.state {
            Inactive if !status.active => Some(false),
            Active | Paused | Stopping | Reconnecting if status.active => Some(true),
            _ => None,
        };
        if self.observed_active == Some(true) {
            self.seen_active = true;
        } else if self.observed_active == Some(false) {
            let expected = self.expected_stop.is_some_and(|intent| {
                at.checked_duration_since(intent)
                    .is_some_and(|age| age <= STOP_INTENT_LIMIT)
            });
            let unexpected = self.seen_active && !expected;
            self.seen_active = false;
            self.expected_stop = None;
            return unexpected;
        }
        false
    }
}
fn index(kind: OutputKind) -> usize {
    match kind {
        OutputKind::Stream => 0,
        OutputKind::Record => 1,
    }
}
fn stopped(kind: OutputKind) -> AlertKind {
    match kind {
        OutputKind::Stream => AlertKind::StreamStopped,
        OutputKind::Record => AlertKind::RecordStopped,
    }
}

impl HealthMonitor {
    fn latch(&mut self, kind: AlertKind) {
        if !self.events.contains_key(&kind) {
            self.next_event = self.next_event.wrapping_add(1);
            self.events.insert(kind, self.next_event);
        }
    }
    pub fn connection_requested(&mut self) {
        self.expected_disconnect = true;
    }
    pub fn connected(&mut self) {
        self.audio = Default::default();
        self.clear_samples();
        self.events.clear();
        self.outputs = Default::default();
        self.was_connected = true;
        self.expected_disconnect = false;
    }
    pub fn disconnected(&mut self) {
        self.audio = Default::default();
        self.clear_samples();
        self.outputs = Default::default();
        if self.was_connected && !self.expected_disconnect {
            self.latch(AlertKind::ConnectionLost);
        }
        self.was_connected = false;
        self.expected_disconnect = false;
    }
    /// Called only after the controller admits an output command, not on a click.
    pub fn output_pending(&mut self, kind: OutputKind, status: &OutputStatus, at: Instant) {
        if !self.was_connected {
            return;
        }
        self.outputs[index(kind)].expected_stop =
            (status.state == OutputRunState::Stopping).then_some(at);
    }
    pub fn output_failed(&mut self, kind: OutputKind, uncertain: bool) {
        if !uncertain {
            self.outputs[index(kind)].expected_stop = None;
        } else {
            self.output_unobserved(kind);
        }
    }
    pub fn output_activity(&self, kind: OutputKind) -> Option<bool> {
        self.outputs[index(kind)].observed_active
    }
    pub fn output_unobserved(&mut self, kind: OutputKind) {
        self.outputs[index(kind)].observed_active = None;
    }
    /// Only actual output observations may establish a stopped-output incident.
    pub fn observe_output(&mut self, kind: OutputKind, status: &OutputStatus, at: Instant) {
        if !self.was_connected {
            return;
        }
        let watch = &mut self.outputs[index(kind)];
        let was_active = watch.seen_active;
        let unexpected = watch.observe(status, at);
        if was_active != watch.seen_active {
            if let Some(previous) = &mut self.previous {
                previous.encoding = None;
                if kind == OutputKind::Stream {
                    previous.stream = None;
                }
            }
        }
        if watch.observed_active == Some(true) {
            self.events.remove(&stopped(kind));
        }
        if unexpected {
            self.latch(stopped(kind));
        }
    }
    /// A stale UI callback cannot acknowledge a different, newer incident.
    pub fn acknowledge(&mut self, alert: Alert) {
        if alert.event_id.is_some() && self.events.get(&alert.kind).copied() == alert.event_id {
            self.events.remove(&alert.kind);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_unrequested_connection_loss_latches_and_retry_does_not_hide_it() {
        let mut health = HealthMonitor::default();
        health.disconnected();
        assert!(health.alerts(Instant::now()).is_empty());
        health.connected();
        health.connection_requested();
        health.disconnected();
        assert!(health.alerts(Instant::now()).is_empty());
        health.connected();
        health.disconnected();
        let alert = health.alerts(Instant::now())[0];
        assert_eq!(alert.kind, AlertKind::ConnectionLost);
        health.connection_requested();
        health.disconnected();
        assert_eq!(health.alerts(Instant::now()), [alert]);
        health.connected();
        assert!(health.alerts(Instant::now()).is_empty());
        health.disconnected();
        health.acknowledge(alert);
        assert_eq!(
            health.alerts(Instant::now()).len(),
            1,
            "old acknowledgement cannot dismiss new loss"
        );
        health.acknowledge(health.alerts(Instant::now())[0]);
        assert!(health.alerts(Instant::now()).is_empty());
    }
    #[test]
    fn unexpected_stop_needs_observed_active_then_inactive_and_is_not_repeated() {
        let now = Instant::now();
        let mut health = HealthMonitor::default();
        health.connected();
        for kind in [OutputKind::Stream, OutputKind::Record] {
            health.observe_output(kind, &OutputStatus::inactive(), now);
            assert!(health.alerts(now).is_empty());
            health.observe_output(kind, &OutputStatus::active(), now);
            health.observe_output(
                kind,
                &OutputStatus::new(false, OutputRunState::Unknown),
                now,
            );
            assert!(health.alerts(now).is_empty());
            health.observe_output(kind, &OutputStatus::inactive(), now);
            let alert = health.alerts(now)[0];
            assert_eq!(alert.kind, stopped(kind));
            health.observe_output(kind, &OutputStatus::inactive(), now);
            assert_eq!(health.alerts(now), [alert]);
            health.acknowledge(alert);
            health.observe_output(kind, &OutputStatus::inactive(), now);
            assert!(health.alerts(now).is_empty());
        }
    }
    #[test]
    fn admitted_stop_is_expected_but_failures_and_expired_intent_are_not_completion() {
        let now = Instant::now();
        let mut health = HealthMonitor::default();
        health.connected();
        let pending = OutputStatus::new(true, OutputRunState::Stopping);
        for kind in [OutputKind::Stream, OutputKind::Record] {
            health.observe_output(kind, &OutputStatus::active(), now);
            health.output_pending(kind, &pending, now);
            assert!(health.alerts(now).is_empty());
            health.observe_output(
                kind,
                &OutputStatus::inactive(),
                now + Duration::from_secs(1),
            );
            assert!(health.alerts(now).is_empty());
            health.observe_output(kind, &OutputStatus::active(), now);
            health.output_pending(kind, &pending, now);
            health.output_failed(kind, false);
            assert!(
                health.alerts(now).is_empty(),
                "failure/fallback alone is not observed termination"
            );
            health.observe_output(
                kind,
                &OutputStatus::inactive(),
                now + Duration::from_secs(1),
            );
            assert_eq!(health.alerts(now)[0].kind, stopped(kind));
            health.observe_output(kind, &OutputStatus::active(), now);
            assert!(health.alerts(now).is_empty());
            health.output_pending(kind, &pending, now);
            health.observe_output(
                kind,
                &OutputStatus::inactive(),
                now + Duration::from_secs(31),
            );
            assert_eq!(health.alerts(now)[0].kind, stopped(kind));
            health.observe_output(kind, &OutputStatus::active(), now);
        }
    }
    #[test]
    fn losing_connection_keeps_already_observed_stop_incidents_until_new_session() {
        let now = Instant::now();
        let mut health = HealthMonitor::default();
        health.connected();
        health.observe_output(OutputKind::Record, &OutputStatus::active(), now);
        health.observe_output(OutputKind::Record, &OutputStatus::inactive(), now);
        health.disconnected();
        assert_eq!(
            health
                .alerts(now)
                .iter()
                .map(|alert| alert.kind)
                .collect::<Vec<_>>(),
            [AlertKind::ConnectionLost, AlertKind::RecordStopped]
        );
        health.connected();
        assert!(health.alerts(now).is_empty());
    }

    #[test]
    fn session_retirement_cannot_invent_stops_or_carry_intent_into_a_new_session() {
        let now = Instant::now();
        let mut health = HealthMonitor::default();
        health.connected();
        health.observe_output(OutputKind::Record, &OutputStatus::active(), now);
        health.output_pending(
            OutputKind::Record,
            &OutputStatus::new(true, OutputRunState::Stopping),
            now,
        );
        health.output_failed(OutputKind::Record, true);
        health.disconnected();
        health.observe_output(OutputKind::Record, &OutputStatus::inactive(), now);
        assert_eq!(
            health
                .alerts(now)
                .iter()
                .map(|alert| alert.kind)
                .collect::<Vec<_>>(),
            [AlertKind::ConnectionLost]
        );
        health.connected();
        health.observe_output(OutputKind::Record, &OutputStatus::inactive(), now);
        assert!(health.alerts(now).is_empty());
        health.observe_output(OutputKind::Record, &OutputStatus::active(), now);
        health.observe_output(OutputKind::Record, &OutputStatus::inactive(), now);
        assert_eq!(health.alerts(now)[0].kind, AlertKind::RecordStopped);
    }
}
