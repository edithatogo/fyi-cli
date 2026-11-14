//! Process-local MCP dispatch measurements, separate from source HTTP budgets.
//!
//! Storage has exactly eight rows and five terminal outcomes. No caller data is
//! accepted as a label. Correlation IDs belong only to events/spans, never metrics.

use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tracing::Span;

const DURATION_BOUNDS_MS: [u64; 6] = [10, 100, 1_000, 10_000, 60_000, 120_000];
const UNAVAILABLE: &str = "remote operation telemetry is unavailable";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum RemoteOperation {
    #[serde(rename = "remote_health")]
    Health,
    #[serde(rename = "remote_version")]
    Version,
    #[serde(rename = "remote_search_requests")]
    SearchRequests,
    #[serde(rename = "remote_get_request")]
    GetRequest,
    #[serde(rename = "remote_list_authorities")]
    ListAuthorities,
    #[serde(rename = "remote_create_request")]
    CreateRequest,
    #[serde(rename = "remote_add_correspondence")]
    AddCorrespondence,
    #[serde(rename = "remote_update_state")]
    UpdateState,
}

impl RemoteOperation {
    const ALL: [Self; 8] = [
        Self::Health,
        Self::Version,
        Self::SearchRequests,
        Self::GetRequest,
        Self::ListAuthorities,
        Self::CreateRequest,
        Self::AddCorrespondence,
        Self::UpdateState,
    ];

    pub(crate) fn from_tool(name: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|operation| operation.as_str() == name)
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Health => "remote_health",
            Self::Version => "remote_version",
            Self::SearchRequests => "remote_search_requests",
            Self::GetRequest => "remote_get_request",
            Self::ListAuthorities => "remote_list_authorities",
            Self::CreateRequest => "remote_create_request",
            Self::AddCorrespondence => "remote_add_correspondence",
            Self::UpdateState => "remote_update_state",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum DispatchOutcome {
    Success,
    Failed,
    TimedOut,
    ResultRejected,
    Cancelled,
}

impl DispatchOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Success => "success",
            Self::Failed => "failed",
            Self::TimedOut => "timed_out",
            Self::ResultRejected => "result_rejected",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Outcomes {
    success: u64,
    failed: u64,
    timed_out: u64,
    result_rejected: u64,
    cancelled: u64,
}

impl Outcomes {
    fn record(&mut self, outcome: DispatchOutcome) {
        let count = match outcome {
            DispatchOutcome::Success => &mut self.success,
            DispatchOutcome::Failed => &mut self.failed,
            DispatchOutcome::TimedOut => &mut self.timed_out,
            DispatchOutcome::ResultRejected => &mut self.result_rejected,
            DispatchOutcome::Cancelled => &mut self.cancelled,
        };
        *count = count.saturating_add(1);
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
struct Counters {
    started: u64,
    in_flight: u64,
    outcomes: Outcomes,
    serialized_result_bytes: u64,
    duration_sum_ms: u64,
    /// Cumulative <= bounds followed by the all-duration (+infinity) bucket.
    duration_bucket_counts: [u64; 7],
}

impl Counters {
    fn complete(&mut self, outcome: DispatchOutcome, bytes: u64, elapsed_ms: u64) {
        self.in_flight = self.in_flight.saturating_sub(1);
        self.outcomes.record(outcome);
        self.serialized_result_bytes = self.serialized_result_bytes.saturating_add(bytes);
        self.duration_sum_ms = self.duration_sum_ms.saturating_add(elapsed_ms);
        for (index, count) in self.duration_bucket_counts.iter_mut().enumerate() {
            if DURATION_BOUNDS_MS
                .get(index)
                .is_none_or(|bound| elapsed_ms <= *bound)
            {
                *count = count.saturating_add(1);
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct OperationMetrics {
    operation: RemoteOperation,
    #[serde(flatten)]
    counters: Counters,
}

#[derive(Debug, Serialize)]
pub(crate) struct MetricsSnapshot {
    schema_version: u8,
    scope: &'static str,
    measurement: &'static str,
    duration_bucket_upper_bounds_ms: [u64; 6],
    operations: Vec<OperationMetrics>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RemoteTelemetry {
    counters: Arc<Mutex<[Counters; 8]>>,
}

impl RemoteTelemetry {
    /// Begin observing one already-admitted operation; never authorizes I/O.
    pub(crate) fn begin(
        &self,
        operation: RemoteOperation,
    ) -> Result<DispatchObservation, &'static str> {
        {
            let mut counters = self.counters.lock().map_err(|_| UNAVAILABLE)?;
            let row = &mut counters[operation as usize];
            row.started = row.started.saturating_add(1);
            row.in_flight = row.in_flight.saturating_add(1);
        }
        let correlation_id = super::next_correlation_id();
        let span = tracing::info_span!(
            "mcp_remote_dispatch",
            schema_version = 1u8,
            correlation_id = correlation_id.as_str(),
            operation = operation.as_str(),
        );
        span.in_scope(|| {
            tracing::info!(
                target: "fyi_mcp::dispatch",
                schema_version = 1u8,
                event = "started",
                correlation_id = correlation_id.as_str(),
                operation = operation.as_str(),
                "remote MCP dispatch"
            );
        });
        Ok(DispatchObservation {
            telemetry: self.clone(),
            operation,
            correlation_id,
            started_at: Instant::now(),
            span,
            finished: false,
        })
    }

    /// One consistent, fixed-size snapshot; reading never resets counters.
    pub(crate) fn snapshot(&self) -> Result<MetricsSnapshot, &'static str> {
        let counters = self.counters.lock().map_err(|_| UNAVAILABLE)?;
        Ok(MetricsSnapshot {
            schema_version: 1,
            scope: "process",
            measurement: "mcp_admitted_operations",
            duration_bucket_upper_bounds_ms: DURATION_BOUNDS_MS,
            operations: RemoteOperation::ALL
                .into_iter()
                .map(|operation| OperationMetrics {
                    operation,
                    counters: counters[operation as usize],
                })
                .collect(),
        })
    }
}

/// The async caller instruments its source future with this guard's span.
/// Drop records cancellation even if the future is abandoned before returning.
pub(crate) struct DispatchObservation {
    telemetry: RemoteTelemetry,
    operation: RemoteOperation,
    correlation_id: String,
    started_at: Instant,
    span: Span,
    finished: bool,
}

impl DispatchObservation {
    pub(crate) fn span(&self) -> Span {
        self.span.clone()
    }

    pub(crate) fn finish(
        mut self,
        outcome: DispatchOutcome,
        bytes: u64,
    ) -> Result<(), &'static str> {
        self.finished = true;
        self.record(outcome, bytes, self.started_at.elapsed())
    }

    fn record(
        &self,
        outcome: DispatchOutcome,
        bytes: u64,
        elapsed: Duration,
    ) -> Result<(), &'static str> {
        let duration_ms = u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX);
        {
            let mut counters = self.telemetry.counters.lock().map_err(|_| UNAVAILABLE)?;
            counters[self.operation as usize].complete(outcome, bytes, duration_ms);
        }
        self.span.in_scope(|| {
            macro_rules! completed {
                ($level:expr) => {
                    tracing::event!(
                        target: "fyi_mcp::dispatch",
                        $level,
                        schema_version = 1u8,
                        event = "completed",
                        correlation_id = self.correlation_id.as_str(),
                        operation = self.operation.as_str(),
                        outcome = outcome.as_str(),
                        duration_ms,
                        serialized_result_bytes = bytes,
                        "remote MCP dispatch"
                    )
                };
            }
            match outcome {
                DispatchOutcome::Success => completed!(tracing::Level::INFO),
                _ => completed!(tracing::Level::WARN),
            }
        });
        Ok(())
    }
}

impl Drop for DispatchObservation {
    fn drop(&mut self) {
        if !self.finished
            && self
                .record(DispatchOutcome::Cancelled, 0, self.started_at.elapsed())
                .is_err()
        {
            tracing::warn!(target: "fyi_mcp::dispatch", "remote operation telemetry is unavailable");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn duration_buckets_are_cumulative_and_include_boundary_values() {
        let mut counters = Counters::default();
        for duration in [
            0, 10, 11, 100, 101, 1_000, 1_001, 10_000, 10_001, 60_000, 60_001, 120_000, 120_001,
        ] {
            counters.complete(DispatchOutcome::Success, 0, duration);
        }
        assert_eq!(counters.duration_bucket_counts, [2, 4, 6, 8, 10, 12, 13]);
        assert_eq!(counters.outcomes.success, 13);
    }

    #[test]
    fn lifetime_counters_saturate_and_never_wrap() {
        let mut counters = Counters {
            serialized_result_bytes: u64::MAX - 1,
            duration_sum_ms: u64::MAX - 1,
            duration_bucket_counts: [u64::MAX; 7],
            outcomes: Outcomes {
                success: u64::MAX,
                ..Outcomes::default()
            },
            ..Counters::default()
        };
        counters.complete(DispatchOutcome::Success, u64::MAX, u64::MAX);
        assert_eq!(counters.serialized_result_bytes, u64::MAX);
        assert_eq!(counters.duration_sum_ms, u64::MAX);
        assert_eq!(counters.duration_bucket_counts, [u64::MAX; 7]);
        assert_eq!(counters.outcomes.success, u64::MAX);
        assert_eq!(counters.in_flight, 0);
    }

    #[test]
    fn concurrent_clones_share_exact_fixed_cardinality_counters() {
        let telemetry = RemoteTelemetry::default();
        std::thread::scope(|scope| {
            for operation in RemoteOperation::ALL {
                let telemetry = &telemetry;
                scope.spawn(move || {
                    for _ in 0..100 {
                        telemetry
                            .begin(operation)
                            .unwrap()
                            .finish(DispatchOutcome::Success, 1)
                            .unwrap();
                    }
                });
            }
        });
        let snapshot = telemetry.clone().snapshot().unwrap();
        assert_eq!(snapshot.operations.len(), 8);
        for row in snapshot.operations {
            assert_eq!(row.counters.started, 100);
            assert_eq!(row.counters.outcomes.success, 100);
            assert_eq!(row.counters.in_flight, 0);
            assert_eq!(row.counters.serialized_result_bytes, 100);
            assert_eq!(row.counters.duration_bucket_counts[6], 100);
        }
    }

    #[test]
    fn unavailable_measurements_fail_closed_without_panicking_on_drop() {
        let telemetry = RemoteTelemetry::default();
        let observation = telemetry.begin(RemoteOperation::Version).unwrap();
        let completion = telemetry.begin(RemoteOperation::Version).unwrap();
        let _ = std::panic::catch_unwind(|| {
            let _lock = telemetry.counters.lock().unwrap();
            panic!("synthetic mutex poison");
        });
        assert!(telemetry.begin(RemoteOperation::Version).is_err());
        assert!(telemetry.snapshot().is_err());
        assert!(completion.finish(DispatchOutcome::Success, 0).is_err());
        drop(observation);
    }
}
