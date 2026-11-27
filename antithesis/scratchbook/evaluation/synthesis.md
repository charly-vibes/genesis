---
sut_path: /var/home/sasha/para/areas/dev/gh/charly/genesis
commit: ddf412d289620067066a0523ec077249d557141b
updated: 2026-11-27
external_references:
  - path: https://hegel.dev/
    why: Engine under adoption decision; per-property fit evidence summarized here.
  - path: /var/home/sasha/para/areas/dev/gh/charly/reports/hegel-eval-2026-11-16/README.md
    why: Prior suite-wide Hegel evaluation this synthesis extends.
  - path: /var/home/sasha/para/areas/dev/gh/charly/antithesis/scratchbook/property-catalog.md
    why: The catalog evaluated in this pass.
---

# Evaluation synthesis — genesis property catalog (genesis-azy)

## Evaluation setup

This pass evaluated the four-property catalog against the prior suite-wide
evaluation (`reports/hegel-eval-2026-11-16/README.md` +
`charly/antithesis/scratchbook/`), with one change of method: every
red/green prediction was **executed** (scratch crate
`antithesis/scratch-hegel/`, genesis as a path dependency), including one
`#[hegel::test]` run — the prior pass predicted red/green analytically and
marked two genesis entries "example-territory" (decidable by a fixed
example, hence weak evidence for Hegel).

## Findings and actions taken

1. **Finding: the 2026-11-16 catalog is stale in three places.** The
   envelope reader now unions both hint channels (prior RED → GREEN); the
   managed-block END-before-START defect is fixed and regression-tested
   (prior RED → GREEN on that mode). Action: catalog entries rewritten at
   current commit `ddf412d`, prior findings marked resolved in evidence
   files' investigation logs.
2. **Finding: "example-territory" was partially wrong for genesis.** The
   prior pass classified `error-envelope-exposes-its-fix` and
   `config-file-round-trip-is-total` as decidable by a fixed example. The
   envelope entry still is (a two-line JSON decides the deserialize
   bypass); the config entry is not — the `repo_root` placement defect was
   found only by testing the *other* contract reading, and the totality
   edge is a generator-reachable class (any all-default struct), not one
   fixed input. Action: envelope entry kept as a deterministic regression
   test with an optional cheap Hegel wrapper; config entry upgraded to
   needs-PBT.
3. **Finding: the ticket's managed-block invariant is stronger than the
   codebase's documented contract.** The existing regression test pins
   stray-END survival (2 END markers) as intended, so "exactly one ordered
   marker pair" cannot be asserted without first deciding the policy
   question. Action: property restated ("at most one … any START precedes
   any END, no user text destroyed, idempotent") with the policy decision
   escalated as `(needs human input)` — this is a bias/decision escalation,
   not something the analysis can settle.
4. **Finding (new, high value): three live RED defects at HEAD.** (a)
   managed-block inject duplicates START markers or destroys user text on
   quote-bearing files (Hegel-shrunk minimal case in 8.7 s);
   (b) all-default configs write a file they cannot read back;
   (c) manifest bytes are hash-order nondeterministic (flaky across
   otherwise identical runs). None were in the prior catalog. Action: all
   three cataloged with generators and seams; no production code touched
   (per ticket).
5. **Coverage gap check.** The four ticket-named areas are covered; areas
   deliberately left out are recorded (write durability / crash faults,
   `ConfigStore::discover` marker semantics, `suggestions`, `doctor`,
   `evals` beyond the envelope reader). Action: noted in sut-analysis;
   not expanded (ticket scope).

## Gaps filled via targeted discovery

The manifest property was added (absent from the 2026-11-16 catalog), with
an evidence file and a check-mode analysis explaining why its
byte-determinism half needs cross-process testing rather than an in-process
generator.

## Escalations to the human owner

- Managed-block policy: prepend vs. adopt for start-without-end files;
  normalize stray-END or keep as pinned.
- Config path contract: join in read/write vs. fix the doc (migration cost:
  three in-tree `path()` impls).
- Manifest: is byte-determinism a requirement? Is corrupt-manifest
  fresh-start (dropping sibling entries) intended?
- Envelope: enforce Invariant 3.2.5 on ingress, or accept the constructor
  as the only enforcement point?

## Final recommendation — whether/how to use Hegel in this repo

**Yes, narrowly: adopt Hegel (`hegeltest = "0.52.7"`, `static-engine`
feature) for exactly one property class — the managed-block rewrite
property — as a `tests/hegel_*.rs` file in a follow-up ticket; use plain
deterministic tests for the other three. Do not put `hegeltest` in
`[dependencies]` or any shipped path.**

- **Managed block → Hegel.** The property is genuinely open-ended (arbitrary
  file text), the defect space had three distinct failure modes reachable
  from short generated text, and the engine found and *shrank* one to the
  minimal case (`picks = [2]`) with a working `reproduce_failure` blob on a
  cold 8.7 s build (`static-engine`; no nested engine build, no runtime
  `.so`, nextest-compatible per the prior evaluation). This confirms the
  2026-11-16 verdict's "ideal Hegel fit" for this entry — the prior pass
  predicted it; this pass executed it.
- **Envelope → deterministic regression test.** The live defect (deserialize
  bypass of Invariant 3.2.5) is decided by one fixed payload; a Hegel
  wrapper adds engine coupling for zero additional discovery. Add the fixed
  case as a plain test when the fix ticket lands.
- **Config → plain property test is enough; Hegel optional.** The two RED
  edges are generator-reachable but small-domain; a plain test looping over
  default/populated structs × both path-contract readings covers them. If
  the repo adopts Hegel for managed blocks anyway, expressing this as a
  Hegel test costs little and keeps one engine in the crate — either is
  defensible; the catalog entry works in both modes.
- **Manifest → not a Hegel property.** The defect is cross-process hash-seed
  nondeterminism; an in-process Hegel generator sees it only stochastically
  (observed 2-of-5 runs green). The right instrument is a `BTreeMap` fix
  plus a deterministic two-tempdir comparison (or a pinned hasher), in
  plain `#[test]`s.
- **Sequencing constraints.** Follow the suite-wide verdict: Hegel tests
  are additive; existing suites stay. Mind the consumer-version skew
  ([consumer-version-skew.md](consumer-version-skew.md)): fixes land for
  the five `^0.14` consumers; `whisper`/`gently`/`specodelic`/`bajan`
  continue on old behavior until they bump. Dev-dep-only adoption keeps
  the test addition skew-free.
