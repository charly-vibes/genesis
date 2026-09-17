//! Golden-file guard for envelope serialization (add-aix-eval-loop §1, task 1.2).
//!
//! The committed snapshot pins the byte-for-byte JSON of an envelope without
//! a receipt, proving the `receipt: Option<ReceiptMeta>` addition is
//! invisible to consumers that don't opt in. If this test fails, the change
//! to `Envelope` broke wire compatibility and downstream tools (wai, dont,
//! espectacular, testaruda, vampiro, DDL) will see shifted output.

use genesis::envelope::{Envelope, EnvelopeKind};
use std::fs;
use std::path::PathBuf;

#[test]
fn envelope_without_receipt_serializes_byte_identically() {
    let env = Envelope::success("my-tool/1.0.0", EnvelopeKind::Ok, "hello", vec![], vec![]);
    let actual = serde_json::to_string_pretty(&env).unwrap() + "\n";

    let golden_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden/envelope_without_receipt.json");
    let golden = fs::read_to_string(&golden_path)
        .unwrap_or_else(|e| panic!("cannot read golden file {}: {e}", golden_path.display()));

    assert!(
        !actual.contains("\"receipt\""),
        "envelope without receipt must not serialize a receipt key"
    );
    assert_eq!(
        actual, golden,
        "envelope wire format drifted from the golden snapshot"
    );
}
