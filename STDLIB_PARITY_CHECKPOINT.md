# CLI/native parity checkpoint — 2026-11-08

Read this first. Evidence root:
`target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.

## Active bounded experiment

- Runner: `parity-qid-replay-20261108/run.py` under the evidence root.
- Started 2026-11-08 16:18:53 UTC, PID 2713804; total cap 2,700s.
- Read `status.json`, then `result.json`, `summary.json` and per-run `results.json`.
- Fresh Z3 processes replay native/CLI/native-label variants in N,C,C,N order.
  The two preserved prefixes contain 39,757 identical commands except source qids.
  All original 3,052 earlier responses match. The target retains its 500ms timeout;
  each replay has a 600s outer watchdog. One worker; no compiler/source changes.
- Identical echo markers and final reason-unknown query record target time/result.
  Protocol sanity check passed. Full responses, earlier deviations, machine load,
  solver/prefix hashes and original prefixes are preserved. No successor queue.
- Look for repeated label-dependent separation versus within-input variability.
  Report earlier response changes. Replay omits verifier computation/think time;
  it remains diagnostic and cannot qualify the original Rust proof.

## Current evidence

Printer fix `931a2d55831ca3dffc34444e8ff0f88750f20bbc` is pushed on Ferrum branch
`fix/stdlib-ast-roundtrip`: 5 roundtrip, 8 printer and 5 frontend tests pass. The
roundtrip test fails on the old printer. The parser already associates `&&`/`||`
to the right; corrected printer metadata preserves expression grouping.

Both real frontend paths share preparation. Four actual `Session::verify` entry
captures finished in 706.36s. Prepared text/settings match except comments and
frontend input handling. After comparison-only empty/single-statement Seqn
normalization, 73,012 node records align except two ReadPerm `LtCmp`/`PermLtCmp`
expressions. Both use `TermFactory::less`. Native expression metadata is stable
across repeats; callback internals are opaque. This is not full semantic parity.

The latest corrected logged run found the first meaningful difference after
39,757 commands when source qids are normalized: native returns `unsat` in 223ms,
CLI `unknown`/`canceled` in 513ms on a 500ms query. Their preceding 3,052 responses
are identical. Source qids differ starting at raw command 7,116; therefore actual
solver inputs are not identical. Native ultimately hits the 600s whole limit;
CLI fails restoration after 492.42s. Logging has affected outcomes previously;
these runs cannot replace unlogged qualification.

Unlogged corrected candidate: native positive passed once in 190.36s, then failed
loop restoration in 282.86s; CLI positive timed out twice at 600s. Original frozen
H1 native passed in 201.28s after two earlier timeouts. Repeated exports match
without comments. Both exact negative controls reached their intended assertion:
Rust line 69 maps to `exhale false`, not the separate cleanup `assert false`.
The initial CLI classifier mistakes were corrected in an audit; raw results remain.

## Provenance and stopping boundary

- Integration `c2631cfb0` / `stdlib-restoration-baseline-20261102` is unqualified.
  No ownership repair integrated. Six pending main files and frozen H1 binaries,
  source and saved dependencies are hash-preserved. Shared Ferrum stays untouched.
- Compiler candidate: `/home/will/wk/exp/prusti-stdlib-parity-fix` with isolated
  Ferrum `/home/will/wk/exp/ferrum-stdlib-roundtrip-fix`, based on pinned `0e048a4`.
- Current solver is the recorded Z3 4.16.0 binary; no upgrade in this experiment.
- Resume restoration only after parity is repaired or adequately explained.
  Retain exact negatives, 180/600s qualification limits, final combined matrix,
  one verifier worker and Ferrum branch parallelism off. Silicon at major/final
  qualification only. Stop before D1–D6; no scheduled automation.

Key evidence directories: `parity-corrected-solver-trace-20261108/`,
`parity-entry-capture-20261106/`, `parity-controlled-repeat-20261106/`,
`parity-printer-fix-20261106/resume-pinned-lock/` (build and audited results).
Original H1 inputs: `prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`.
Acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`; plan: `STDLIB_EXTERN_SPEC.md`.
