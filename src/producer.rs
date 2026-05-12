//! [`Producer`] integration: wrap a [`DepCheck`] and emit a [`Report`]
//! every time the producer runs.
//!
//! [`Producer`]: dev_report::Producer
//! [`Report`]: dev_report::Report

use dev_report::{CheckResult, Producer, Report, Severity};

use crate::DepCheck;

/// `Producer` adapter that runs a [`DepCheck`] and converts the result
/// into a `Report`.
///
/// Subprocess failures map to a single failing `CheckResult` named
/// `deps::health` with `Severity::Critical` — no panics.
///
/// # Example
///
/// ```no_run
/// use dev_deps::{DepCheck, DepProducer, DepScope};
/// use dev_report::Producer;
///
/// let producer = DepProducer::new(
///     DepCheck::new("my-crate", "0.1.0").scope(DepScope::All),
/// );
/// let report = producer.produce();
/// println!("{}", report.to_json().unwrap());
/// ```
pub struct DepProducer {
    check: DepCheck,
}

impl DepProducer {
    /// Build a producer that drives the given [`DepCheck`].
    pub fn new(check: DepCheck) -> Self {
        Self { check }
    }
}

impl Producer for DepProducer {
    fn produce(&self) -> Report {
        match self.check.execute() {
            Ok(result) => result.into_report(),
            Err(e) => {
                let mut report = Report::new(self.check.subject(), self.check.subject_version())
                    .with_producer("dev-deps");
                let check = CheckResult::fail("deps::health", Severity::Critical)
                    .with_detail(e.to_string())
                    .with_tag("deps")
                    .with_tag("subprocess");
                report.push(check);
                report.finish();
                report
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::DepScope;

    #[test]
    fn produce_returns_report_when_tool_missing() {
        // Default runner image won't have cargo-udeps + cargo-outdated
        // installed; the producer should surface that as a failing
        // CheckResult rather than panicking.
        let producer = DepProducer::new(DepCheck::new("self", "0.0.0").scope(DepScope::Unused));
        let report = producer.produce();
        assert_eq!(report.subject, "self");
        assert!(!report.checks.is_empty());
    }
}
