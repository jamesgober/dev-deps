use dev_deps::{DepCheck, DepResult, DepScope, OutdatedDep, UnusedDep};

#[test]
fn smoke_default_scope_is_all() {
    let c = DepCheck::new("x", "0.1.0");
    assert_eq!(c.dep_scope(), DepScope::All);
}

#[test]
fn smoke_scope_selection() {
    let c = DepCheck::new("x", "0.1.0").scope(DepScope::Unused);
    assert_eq!(c.dep_scope(), DepScope::Unused);
}

#[test]
fn smoke_empty_result_total_findings() {
    let r = DepResult {
        name: "x".into(),
        version: "0.1.0".into(),
        scope: DepScope::All,
        unused: Vec::new(),
        outdated: Vec::new(),
    };
    assert_eq!(r.total_findings(), 0);
}

#[test]
fn smoke_empty_result_passes() {
    let r = DepResult {
        name: "x".into(),
        version: "0.1.0".into(),
        scope: DepScope::All,
        unused: Vec::new(),
        outdated: Vec::new(),
    };
    let report = r.into_report();
    assert!(report.passed());
}

#[test]
fn smoke_unused_produces_warn() {
    let r = DepResult {
        name: "x".into(),
        version: "0.1.0".into(),
        scope: DepScope::All,
        unused: vec![UnusedDep {
            crate_name: "foo".into(),
            kind: "dependencies".into(),
        }],
        outdated: Vec::new(),
    };
    let report = r.into_report();
    assert!(report.warned());
}

#[test]
fn smoke_mixed_findings_count() {
    let r = DepResult {
        name: "x".into(),
        version: "0.1.0".into(),
        scope: DepScope::All,
        unused: vec![UnusedDep {
            crate_name: "foo".into(),
            kind: "dependencies".into(),
        }],
        outdated: vec![OutdatedDep {
            crate_name: "bar".into(),
            current: "1.0.0".into(),
            latest: "2.0.0".into(),
            major_behind: 1,
        }],
    };
    assert_eq!(r.total_findings(), 2);
}
