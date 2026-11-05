//! `DoctorRunner` registration and convenience-constructor tests.

use super::*;

#[test]
fn test_runner_empty() {
    let runner: DoctorRunner = DoctorRunner::new(vec![]);
    assert!(runner.is_empty());
    assert_eq!(runner.len(), 0);

    let dir = tmp();
    let report = runner.run(dir.path(), false).require("value");
    assert!(report.is_healthy());
}

#[test]
fn test_runner_with_fn() {
    let mut runner = DoctorRunner::new(vec![]);
    runner.register(Box::new(PassCheck));
    assert_eq!(runner.len(), 1);
}

#[test]
fn test_runner_fluent_with() {
    let runner = DoctorRunner::new(vec![])
        .with(Box::new(PassCheck))
        .with(Box::new(WarnCheck));
    assert_eq!(runner.len(), 2);
}

#[test]
fn test_runner_convenience_fn() {
    let r = runner(vec![Box::new(PassCheck)]);
    assert_eq!(r.len(), 1);
}
