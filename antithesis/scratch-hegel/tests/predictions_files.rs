//! genesis-azy predictions, part 2 of 2: managed-block (P2) and manifest
//! (P4) seams. Each test mirrors one scratchbook property; the pass/fail
//! outcome is the "predicted red/green" evidence recorded there.

mod common;

use common::temp_root;
use genesis::discovery;
use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use std::path::Path;

const START: &str = "<!-- WAI:START -->";
const END: &str = "<!-- WAI:END -->";
const FILE_NAME: &str = "AGENTS.md";

fn injector() -> BlockInjector {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    BlockInjector::new(reg)
}

fn inject_and_read(path: &Path, content: &str) -> String {
    common::require(injector().inject(path, "WAI", content), "inject WAI block");
    common::require(std::fs::read_to_string(path), "read injected file")
}

fn marker_counts(text: &str, marker: &str) -> usize {
    text.matches(marker).count()
}

#[test]
fn p2_inject_idempotent_on_clean_file() {
    let root = temp_root("mb-idem");
    let f = root.path().join(FILE_NAME);
    common::require(std::fs::write(&f, "# header\n\nbody text\n"), "seed clean file");
    let once = inject_and_read(&f, "generated content\n");
    let twice = inject_and_read(&f, "generated content\n");
    assert_eq!(once, twice, "re-inject not byte-identical");
    assert_eq!(marker_counts(&once, START), 1);
    assert_eq!(marker_counts(&once, END), 1);
    let (s, e) = match (once.find(START), once.find(END)) {
        (Some(s), Some(e)) => (s, e),
        _ => panic!("markers missing after clean-file inject"),
    };
    assert!(s < e, "markers out of order");
    println!("P2a RESULT: GREEN (idempotent on clean file)");
}

/// The defect the 2026-11-16 scan flagged: END before START.
#[test]
fn p2_end_before_start_file() {
    let root = temp_root("mb-reorder");
    let f = root.path().join(FILE_NAME);
    common::require(std::fs::write(&f, format!("{END}\nstray\n{START}\nbody\n{END}\n")), "seed end-before-start file");
    let after = inject_and_read(&f, "new content\n");
    eprintln!("P2b after:\n{after}");
    assert_eq!(marker_counts(&after, START), 1, "duplicated START markers");
    assert_eq!(marker_counts(&after, END), 1, "duplicated END markers");
    let (s, e) = match (after.find(START), after.find(END)) {
        (Some(s), Some(e)) => (s, e),
        _ => panic!("markers missing after inject"),
    };
    assert!(s < e, "START after END after inject");
    println!("P2b RESULT: GREEN (end-before-start handled)");
}

/// A file that mentions the start marker (docs quoting the dialect) but
/// has no real block: inject must not create a second START marker.
#[test]
fn p2_quoted_start_marker_no_end() {
    let root = temp_root("mb-quoted");
    let f = root.path().join(FILE_NAME);
    common::require(
        std::fs::write(
            &f,
            "# Docs\n\nBlocks open with <!-- WAI:START --> and close with a matching end marker.\n",
        ),
        "write quoted-marker docs file",
    );
    let after = inject_and_read(&f, "generated\n");
    eprintln!("P2c after:\n{after}");
    assert_eq!(
        marker_counts(&after, START),
        1,
        "quoted start marker produced a second START"
    );
    println!("P2c RESULT: GREEN (quoted marker safe)");
}

/// Quoted start marker appears BEFORE the file's real block: the span
/// search must not splice away everything between the quote and the
/// real block's end marker.
#[test]
fn p2_quoted_start_before_real_block() {
    let root = temp_root("mb-splice");
    let f = root.path().join(FILE_NAME);
    let original = "intro <!-- WAI:START --> quoted\n\n# Real section\n\nprecious user notes\n\n<!-- WAI:START -->\nreal\n<!-- WAI:END -->\n";
    common::require(std::fs::write(&f, original), "seed quoted-before-block file");
    let after = inject_and_read(&f, "generated\n");
    eprintln!("P2d after:\n{after}");
    assert!(
        after.contains("precious user notes"),
        "inject destroyed user text between a quoted start marker and the real block"
    );
    println!("P2d RESULT: GREEN (quote before real block safe)");
}

fn register_pair(root: &Path, order: [&str; 2]) {
    let specs = [
        ("wai", "Workflow manager", "directory", ".wai"),
        ("dont", "Decision-logged conventions", "directory", ".dont"),
    ];
    for name in order {
        let (n, desc, dtype, dpath) = match specs.iter().find(|s| s.0 == name) {
            Some(spec) => *spec,
            None => panic!("unknown tool in registration order: {name}"),
        };
        common::require(discovery::register(root, n, desc, dtype, dpath), "register tool");
    }
}

#[test]
fn p4a_manifest_register_unregister_converges() {
    let root = temp_root("manifest");
    let p = root.path();
    register_pair(p, ["wai", "dont"]);
    assert_eq!(discovery::list_tools(p).len(), 2);
    common::require(discovery::unregister(p, "wai"), "unregister wai");
    common::require(discovery::unregister(p, "dont"), "unregister dont");
    assert!(
        !discovery::has_manifest(p),
        "manifest left behind after unregistering all tools"
    );
    common::require(discovery::unregister(p, "ghost"), "unregister missing manifest is a no-op");
    println!("P4a RESULT: GREEN (convergence)");
}

/// Same logical end state, different registration order.
#[test]
fn p4b_manifest_register_is_byte_deterministic() {
    let a = temp_root("manifest-a");
    let b = temp_root("manifest-b");
    register_pair(a.path(), ["wai", "dont"]);
    register_pair(b.path(), ["dont", "wai"]);
    let ba = common::require(std::fs::read_to_string(a.path().join(".genesis/tools.toml")), "read manifest A");
    let bb = common::require(std::fs::read_to_string(b.path().join(".genesis/tools.toml")), "read manifest B");
    eprintln!("P4b A:\n{ba}\nP4b B:\n{bb}");
    assert_eq!(ba, bb, "register order leaked into serialized bytes");
    println!("P4b RESULT: GREEN (byte-deterministic)");
}
