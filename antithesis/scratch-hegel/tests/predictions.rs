//! Empirical red/green checks for the genesis-azy property baseline.
//! Each test mirrors one scratchbook property; the pass/fail outcome is the
//! "predicted red/green" evidence recorded in the scratchbook.

use genesis::config::ConfigFile;
use genesis::discovery;
use genesis::envelope::{
    parse_envelope_value, Envelope, EnvelopeKind, ErrorResult, RemediationEntry,
};
use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use serde::{Deserialize, Serialize};
use std::path::Path;

fn temp_root(tag: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("genesis-azy-{tag}-"))
        .tempdir()
        .unwrap()
}

// ── P3: config write/read totality (all-default structs) ──────────────

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct AllDefault {
    #[serde(skip_serializing_if = "Option::is_none")]
    opt: Option<String>,
}

impl ConfigFile for AllDefault {
    /// Consumers must join repo_root themselves: the trait doc says "relative
    /// to repo_root", but the provided read()/write() never join it. See P3a.
    fn path(repo_root: &Path) -> std::path::PathBuf {
        repo_root.join("tool.toml")
    }
}

/// Same struct, but path() implemented exactly as the trait doc describes
/// ("relative to repo_root") — the reading a new consumer would code first.
#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
struct RelativePathConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    opt: Option<String>,
}

impl ConfigFile for RelativePathConfig {
    fn path(_repo_root: &Path) -> std::path::PathBuf {
        std::path::PathBuf::from("tool.toml")
    }
}

#[test]
fn p3_config_roundtrip_all_default() {
    let root = temp_root("config");
    let cfg = AllDefault::default();
    AllDefault::write(&cfg, root.path()).unwrap();
    let bytes = std::fs::read_to_string(root.path().join("tool.toml")).unwrap();
    eprintln!("P3 written bytes: {bytes:?}");
    let reread = AllDefault::read(root.path());
    match reread {
        Ok(v) => {
            eprintln!("P3 re-read: {v:?}");
            assert_eq!(cfg, v, "roundtrip value mismatch");
            println!("P3 RESULT: GREEN");
        }
        Err(e) => {
            eprintln!("P3 read error: {e:?}");
            panic!("P3 RESULT: RED — write() output not readable back: {e}");
        }
    }
}

#[test]
fn p3a_write_honors_repo_root() {
    // The documented contract: path() returns the file "relative to
    // repo_root". A consumer coding to the doc writes into the project;
    // the provided write() actually resolves against the process cwd.
    let root = temp_root("config-root");
    let cfg = RelativePathConfig { opt: Some("x".into()) };
    RelativePathConfig::write(&cfg, root.path()).unwrap();
    let in_project = root.path().join("tool.toml").exists();
    let in_cwd = Path::new("tool.toml").exists();
    let _ = std::fs::remove_file("tool.toml"); // cleanup the stray
    assert!(
        in_project,
        "P3a RESULT: RED — write(repo_root) ignored repo_root (documented relative-path contract)"
    );
    assert!(!in_cwd, "write leaked into the process cwd");
    println!("P3a RESULT: GREEN (repo_root honored)");
}

#[test]
fn p3_config_roundtrip_nonempty() {
    let root = temp_root("config2");
    let cfg = AllDefault {
        opt: Some("hello".into()),
    };
    AllDefault::write(&cfg, root.path()).unwrap();
    let v = AllDefault::read(root.path()).unwrap();
    assert_eq!(cfg, v);
    // write→read→write byte stability
    let b1 = std::fs::read_to_string(root.path().join("tool.toml")).unwrap();
    AllDefault::write(&v, root.path()).unwrap();
    let b2 = std::fs::read_to_string(root.path().join("tool.toml")).unwrap();
    assert_eq!(b1, b2, "write→read→write not byte-stable");
    println!("P3b RESULT: GREEN (nonempty configs roundtrip)");
}

// ── P4a: manifest register/unregister convergence ─────────────────────

#[test]
fn p4a_manifest_register_unregister_converges() {
    let root = temp_root("manifest");
    let p = root.path();
    discovery::register(p, "wai", "w", "directory", ".wai").unwrap();
    discovery::register(p, "dont", "d", "directory", ".dont").unwrap();
    assert_eq!(discovery::list_tools(p).len(), 2);
    discovery::unregister(p, "wai").unwrap();
    discovery::unregister(p, "dont").unwrap();
    assert!(
        !discovery::has_manifest(p),
        "manifest left behind after unregistering all tools"
    );
    // unregister on missing manifest is a no-op
    discovery::unregister(p, "ghost").unwrap();
    println!("P4a RESULT: GREEN (convergence)");
}

#[test]
fn p4b_manifest_register_is_byte_deterministic() {
    let a = temp_root("manifest-a");
    let b = temp_root("manifest-b");
    // Same logical end state, different registration order.
    discovery::register(a.path(), "wai", "Workflow manager", "directory", ".wai").unwrap();
    discovery::register(a.path(), "dont", "Decision-logged conventions", "directory", ".dont")
        .unwrap();
    discovery::register(b.path(), "dont", "Decision-logged conventions", "directory", ".dont")
        .unwrap();
    discovery::register(b.path(), "wai", "Workflow manager", "directory", ".wai").unwrap();
    let ba = std::fs::read_to_string(a.path().join(".genesis/tools.toml")).unwrap();
    let bb = std::fs::read_to_string(b.path().join(".genesis/tools.toml")).unwrap();
    eprintln!("P4b A:\n{ba}\nP4b B:\n{bb}");
    assert_eq!(ba, bb, "register order leaked into serialized bytes");
    println!("P4b RESULT: GREEN (byte-deterministic)");
}

// ── P2: managed-block marker uniqueness + idempotency ─────────────────

fn injector() -> BlockInjector {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    BlockInjector::new(reg)
}

fn marker_counts(text: &str, marker: &str) -> usize {
    text.matches(marker).count()
}

#[test]
fn p2_inject_idempotent_on_clean_file() {
    let root = temp_root("mb-idem");
    let f = root.path().join("AGENTS.md");
    std::fs::write(&f, "# header\n\nbody text\n").unwrap();
    let inj = injector();
    inj.inject(&f, "WAI", "generated content\n").unwrap();
    let once = std::fs::read_to_string(&f).unwrap();
    inj.inject(&f, "WAI", "generated content\n").unwrap();
    let twice = std::fs::read_to_string(&f).unwrap();
    assert_eq!(once, twice, "re-inject not byte-identical");
    assert_eq!(marker_counts(&once, "<!-- WAI:START -->"), 1);
    assert_eq!(marker_counts(&once, "<!-- WAI:END -->"), 1);
    let start = once.find("<!-- WAI:START -->").unwrap();
    let end = once.find("<!-- WAI:END -->").unwrap();
    assert!(start < end, "markers out of order");
    println!("P2a RESULT: GREEN (idempotent on clean file)");
}

#[test]
fn p2_end_before_start_file() {
    // The defect the 2026-11-16 scan flagged: END before START.
    let root = temp_root("mb-reorder");
    let f = root.path().join("AGENTS.md");
    std::fs::write(
        &f,
        "<!-- WAI:END -->\nstray\n<!-- WAI:START -->\nbody\n<!-- WAI:END -->\n",
    )
    .unwrap();
    let inj = injector();
    inj.inject(&f, "WAI", "new content\n").unwrap();
    let after = std::fs::read_to_string(&f).unwrap();
    eprintln!("P2b after:\n{after}");
    assert_eq!(
        marker_counts(&after, "<!-- WAI:START -->"),
        1,
        "duplicated START markers"
    );
    assert_eq!(marker_counts(&after, "<!-- WAI:END -->"), 1, "duplicated END markers");
    let s = after.find("<!-- WAI:START -->").unwrap();
    let e = after.find("<!-- WAI:END -->").unwrap();
    assert!(s < e, "START after END after inject");
    println!("P2b RESULT: GREEN (end-before-start handled)");
}

#[test]
fn p2_quoted_start_marker_no_end() {
    // A file that mentions the start marker (docs quoting the dialect) but
    // has no real block: inject must not create a second START marker.
    let root = temp_root("mb-quoted");
    let f = root.path().join("AGENTS.md");
    std::fs::write(
        &f,
        "# Docs\n\nBlocks open with <!-- WAI:START --> and close with a matching end marker.\n",
    )
    .unwrap();
    let inj = injector();
    inj.inject(&f, "WAI", "generated\n").unwrap();
    let after = std::fs::read_to_string(&f).unwrap();
    eprintln!("P2c after:\n{after}");
    assert_eq!(
        marker_counts(&after, "<!-- WAI:START -->"),
        1,
        "quoted start marker produced a second START"
    );
    println!("P2c RESULT: GREEN (quoted marker safe)");
}

#[test]
fn p2_quoted_start_before_real_block() {
    // Quoted start marker appears BEFORE the file's real block: the span
    // search must not splice away everything between the quote and the
    // real block's end marker.
    let root = temp_root("mb-splice");
    let f = root.path().join("AGENTS.md");
    let original = "intro <!-- WAI:START --> quoted\n\n# Real section\n\nprecious user notes\n\n<!-- WAI:START -->\nreal\n<!-- WAI:END -->\n";
    std::fs::write(&f, original).unwrap();
    let inj = injector();
    inj.inject(&f, "WAI", "generated\n").unwrap();
    let after = std::fs::read_to_string(&f).unwrap();
    eprintln!("P2d after:\n{after}");
    assert!(
        after.contains("precious user notes"),
        "inject destroyed user text between a quoted start marker and the real block"
    );
    println!("P2d RESULT: GREEN (quote before real block safe)");
}

// ── P1: error envelope exposes its remediation ────────────────────────

#[test]
fn p1_error_envelope_remediation_visible() {
    let err = ErrorResult::new(
        "E_TEST",
        "something failed",
        None,
        None,
        None,
        vec![],
        vec![RemediationEntry {
            command: "genesis doctor --fix".into(),
            description: "run the fix".into(),
        }],
    )
    .unwrap();
    let env = Envelope::error("1.0.0", "my-tool/0.1.0", err, vec![]);
    let json = serde_json::to_value(&env).unwrap();
    let outcome = parse_envelope_value(&json).unwrap();
    match outcome {
        genesis::envelope::EnvelopeOutcome::Error {
            code,
            hint_commands,
            ..
        } => {
            assert_eq!(code.as_deref(), Some("E_TEST"));
            assert!(
                hint_commands.iter().any(|c| c.contains("doctor --fix")),
                "remediation command invisible to parse_envelope_value: {hint_commands:?}"
            );
            println!("P1a RESULT: GREEN (constructor path exposes remediation)");
        }
        genesis::envelope::EnvelopeOutcome::Ok => panic!("error envelope parsed as Ok"),
    }
}

#[test]
fn p1_deserialized_error_envelope_invariant() {
    // Invariant 3.2.5 says remediation must be non-empty. The constructor
    // enforces it; the Deserialize impl is the second construction path.
    let raw = r#"{
        "ok": false,
        "envelope_version": "1.0.0",
        "cli_version": "t/0.1.0",
        "envelope_kind": "error",
        "data": {
            "code": "E_X",
            "message": "m",
            "rule_name": null,
            "spec_ref": null,
            "entity_id": null,
            "unmet_clauses": [],
            "remediation": []
        },
        "warnings": [],
        "hints": null,
        "meta": { "duration_ms": 0, "tx": null, "request_id": null }
    }"#;
    let parsed: Result<Envelope<ErrorResult>, _> = serde_json::from_str(raw);
    match parsed {
        Ok(_) => panic!(
            "P1b RESULT: RED — Deserialize bypasses Invariant 3.2.5 (empty remediation accepted)"
        ),
        Err(_) => println!("P1b RESULT: GREEN (deserialize upholds Invariant 3.2.5)"),
    }
}

#[test]
fn p1_error_envelope_with_no_fix_anywhere_is_visible() {
    // A deserialize-made error envelope with empty remediation and no hints:
    // does the reader at least report it faithfully (zero commands, not a lie)?
    let raw = r#"{
        "ok": false,
        "envelope_version": "1.0.0",
        "cli_version": "t/0.1.0",
        "envelope_kind": "error",
        "data": {
            "code": "E_X",
            "message": "m",
            "rule_name": null,
            "spec_ref": null,
            "entity_id": null,
            "unmet_clauses": [],
            "remediation": []
        },
        "warnings": [],
        "hints": null,
        "meta": { "duration_ms": 0, "tx": null, "request_id": null }
    }"#;
    let v: serde_json::Value = serde_json::from_str(raw).unwrap();
    let outcome = parse_envelope_value(&v).unwrap();
    match outcome {
        genesis::envelope::EnvelopeOutcome::Error {
            hint_commands, ..
        } => {
            assert!(
                hint_commands.is_empty(),
                "phantom commands synthesized from nothing: {hint_commands:?}"
            );
            println!("P1c RESULT: GREEN (faithful zero-command report)");
        }
        genesis::envelope::EnvelopeOutcome::Ok => panic!("ok:false envelope parsed as Ok"),
    }
}
