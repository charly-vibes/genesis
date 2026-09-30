//! Feature-gate guard: the HTTP stack must stay optional.
//!
//! Runs ungated (default features) on purpose — it must pass precisely when
//! `update-check` is NOT enabled, guarding against `ureq` accidentally
//! becoming a hard dependency of every genesis consumer.

use std::process::Command;

#[test]
fn feature_gate_keeps_http_stack_optional() {
    let manifest = std::fs::read_to_string("Cargo.toml").expect("read Cargo.toml from crate root");

    let deps_section = manifest
        .split("[dependencies]")
        .nth(1)
        .and_then(|rest| rest.split("[features]").next())
        .expect("find [dependencies] section");

    for (dep, name) in [("ureq", "ureq"), ("semver", "semver")] {
        let line = deps_section
            .lines()
            .find(|l| l.starts_with(&format!("{dep} = ")))
            .unwrap_or_else(|| panic!("{name} must be declared as a dependency"));
        assert!(
            line.contains("optional = true"),
            "{name} must be optional (selected only by the update-check feature), got: {line}"
        );
    }

    let features_section = manifest
        .split("[features]")
        .nth(1)
        .and_then(|rest| rest.split("[dev-dependencies]").next())
        .expect("find [features] section");

    let update_check_line = features_section
        .lines()
        .find(|l| l.starts_with("update-check = "))
        .expect("update-check feature must be declared");
    for dep in ["ureq", "semver"] {
        assert!(
            update_check_line.contains(&format!("dep:{dep}")),
            "update-check feature must select dep:{dep}, got: {update_check_line}"
        );
    }
}

#[test]
fn crate_builds_without_the_feature() {
    // `cargo check --no-default-features` must succeed: dependents that do
    // not opt in must not inherit the HTTP stack. Slow-ish; run the check.
    let status = Command::new("cargo")
        .args(["check", "--no-default-features"])
        .status()
        .expect("spawn cargo check");
    assert!(
        status.success(),
        "crate must build without the update-check feature"
    );
}
