//! One Hegel-driven run of the managed-block property, to validate the
//! engine-fit claim recorded in the genesis-azy scratchbook. Research
//! artifact only.

use genesis::managed_block::{BlockDef, BlockInjector, BlockRegistry};
use hegel::generators as gs;
use hegel::TestCase;

const RAW_START_LINE: &str = "<!-- WAI:START -->\n";
const RAW_END_LINE: &str = "<!-- WAI:END -->\n";
const QUOTED_START_LINE: &str = "docs quoting <!-- WAI:START --> here\n";

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

#[hegel::test(test_cases = 200)]
fn inject_keeps_exactly_one_ordered_pair(tc: TestCase) {
    let picks: Vec<usize> = tc.draw(gs::vecs(gs::integers::<usize>().max_value(5)).max_size(8));
    let file: String = picks.iter().map(|i| POOL[*i]).collect();

    let dir = tempfile::tempdir().unwrap();
    let f = dir.path().join("AGENTS.md");
    std::fs::write(&f, &file).unwrap();

    let inj = injector();
    inj.inject(&f, "WAI", "generated\n").unwrap();
    let after = std::fs::read_to_string(&f).unwrap();

    let starts = after.matches("<!-- WAI:START -->").count();
    let ends = after.matches("<!-- WAI:END -->").count();
    assert!(
        starts == 1 && ends == 1,
        "marker pair corrupted (starts={starts}, ends={ends})"
    );
    let s = after.find("<!-- WAI:START -->").unwrap();
    let e = after.find("<!-- WAI:END -->").unwrap();
    assert!(s < e, "START after END");

    inj.inject(&f, "WAI", "generated\n").unwrap();
    let again = std::fs::read_to_string(&f).unwrap();
    assert_eq!(after, again, "re-inject not byte-identical");
}
