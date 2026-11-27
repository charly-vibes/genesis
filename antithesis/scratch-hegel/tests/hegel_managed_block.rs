//! One Hegel-driven run of the managed-block property, to validate the
//! engine-fit claim recorded in the genesis-azy scratchbook. Research
//! artifact only.

use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use hegel::generators as gs;
use hegel::TestCase;

const START_MARKER: &str = "<!-- WAI:START -->";
const END_MARKER: &str = "<!-- WAI:END -->";
const RAW_START_LINE: &str = "<!-- WAI:START -->\n";
const RAW_END_LINE: &str = "<!-- WAI:END -->\n";
const QUOTED_START_LINE: &str = "docs quoting <!-- WAI:START --> here\n";
const GENERATED: &str = "generated\n";
const FILE_NAME: &str = "AGENTS.md";

const POOL: [&str; 6] = [
    "plain prose line\n",
    "# a heading\n",
    RAW_START_LINE,
    RAW_END_LINE,
    QUOTED_START_LINE,
    "\n",
];

fn injector() -> BlockInjector {
    let mut reg = BlockRegistry::new();
    reg.register(BlockDef::new("WAI"));
    BlockInjector::new(reg)
}

/// Inject `content` once into a tempdir file holding `body`.
fn write_and_inject(body: &str, content: &str) -> (tempfile::TempDir, std::path::PathBuf, String) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(FILE_NAME);
    std::fs::write(&path, body).unwrap();
    injector().inject(&path, "WAI", content).unwrap();
    let after = std::fs::read_to_string(&path).unwrap();
    (dir, path, after)
}

/// The catalog invariant: at most one START and one END marker, with the
/// START first. `inject` always places a block, so both markers exist.
fn assert_single_ordered_pair(text: &str) {
    let starts = text.matches(START_MARKER).count();
    let ends = text.matches(END_MARKER).count();
    assert!(
        starts == 1 && ends == 1,
        "marker pair corrupted (starts={starts}, ends={ends})"
    );
    let first_start = text.find(START_MARKER).unwrap();
    let first_end = text.find(END_MARKER).unwrap();
    assert!(first_start < first_end, "START after END");
}

/// Re-inject identical content and return the file text after it.
fn reinject(path: &std::path::Path) -> String {
    injector().inject(path, "WAI", GENERATED).unwrap();
    std::fs::read_to_string(path).unwrap()
}

#[hegel::test(test_cases = 200)]
fn inject_keeps_exactly_one_ordered_pair(tc: TestCase) {
    let picks: Vec<usize> = tc.draw(gs::vecs(gs::integers::<usize>().max_value(5)).max_size(8));
    let body: String = picks.iter().map(|i| POOL[*i]).collect();
    let (_dir, path, after) = write_and_inject(&body, GENERATED);
    assert_single_ordered_pair(&after);
    assert_eq!(after, reinject(&path), "re-inject not byte-identical");
}
