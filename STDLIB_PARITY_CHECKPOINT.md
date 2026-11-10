# CLI/native parity checkpoint — 2026-11-10

Evidence root: `target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.

## Latest gate: failed, no active job

`parity-full-rust-resource-20261110/` completed all four full checks in 2,448.27s:
original Rust positive and exact negative, each native then CLI of its exact export.
**All four hit the 600s whole-program limit.** Native/CLI positive command times
609.05/607.21s; negatives 611.66/606.58s. The 720s outer watchdog never fired.
No positive passed; neither intended negative diagnostic was reached. Timeouts
are unresolved, never successful negative controls. See `review.json`/`results.json`.

Both paths explicitly used `--proverEnableTimeBounds=false` and
`--z3ResourcesPerMillisecond=10000`, assertion/check nominal 30000/100, one worker,
branch false. No logging/instrumentation. Frozen corrected compiler/CLI, same
Rust metadata IDs, and exact prior proof text excluding comments. This resource-mode
gate is **not** a qualified solution and has not been adopted or integrated.

Original temporary H1 source is now restored: all 2,992 source hashes, PCG 791e4ca
and 187 saved dependency hashes match. Frozen binaries and six pending main files
remain unchanged. Source and dependency restoration are captured in `restoration.json`.

Next evidence: profile the full resource-mode positive under its unchanged 600s
limit to locate expensive obligations and repeated work. Compare required proof
structure before any repair. Restore a passing full Rust positive and exact
negative, then affected regressions before integration. No automatic successor queue.

## What is established

- Printer fix `931a2d55831ca3dffc34444e8ff0f88750f20bbc` is pushed on Ferrum
  `fix/stdlib-ast-roundtrip`;18 focused tests pass and the new regression fails
  with the old printer. Boolean fixity now matches the existing parser.
- Both actual frontend paths already share preparation. Prepared text/settings
  match except comments/input handling. Comparison-only empty Seqn normalization
  aligns 73,012 node records except two ReadPerm comparisons that use the same
  solver less-than builder. Callback internals remain opaque.
- First observed timed-query divergence occurs after 39,757 commands modulo source
  qids; identical CLI inputs can prove or time out. Earlier native outcomes vary too.
- Earliest known variable query 2839 is a 500ms quantified permission depletion check.
  Proving all remaining permission zero removes the chunk; unknown conservatively
  retains it. Required consumption and restoration proofs remain mandatory.
- Six timer/resource replays completed in 1,406.69s. Identical timed inputs and
  response histories yield different work. All four resource-native/CLI replays
  match 2,839 responses and 129,144,583 total work units, with target work 1,130,340.
  Search counters match; time/memory/allocations vary. This supports wall-clock
  cancellation changing incremental search progress, not an established ownership
  encoding defect. Learned clauses were not directly captured.
- Fixed-work target execution still ranges 745.52–1,105.65ms, with CPU tracking wall
  time. Specific hardware/cache/frequency cause is unmeasured. Resource mapping
  permits different work than nominal milliseconds and can be more expensive.

## Provenance and boundary

Integration `c2631cfb0` / `stdlib-restoration-baseline-20261102` remains unqualified;
no ownership repair integrated. Main has six preserved pending encoder files.
Compiler candidate `/home/will/wk/exp/prusti-stdlib-parity-fix` uses isolated Ferrum
`/home/will/wk/exp/ferrum-stdlib-roundtrip-fix`, based on 0e048a4 with printer fix.
Original H1 is `/tmp/prusti-restoration-prefix-h1`. Shared Ferrum is untouched.

Retain 180/600 qualification limits and exact negatives; no waived assertions,
permissions, frames, alias facts or obligations. Final complete stdlib matrix and
Silicon qualification remain required. One worker, branch parallelism off. Stop
before D1–D6; no scheduled automation.

Key evidence: `parity-full-rust-resource-20261110/`,
`parity-variability-resource-20261110/`, `parity-corrected-solver-trace-20261108/`,
`parity-entry-capture-20261106/`, `parity-printer-fix-20261106/resume-pinned-lock/`.
Original captures: `prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`.
Plan: `STDLIB_EXTERN_SPEC.md`; acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`.
