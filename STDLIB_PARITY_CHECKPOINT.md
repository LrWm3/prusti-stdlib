# CLI/native parity checkpoint — 2026-11-06

Read this first; detailed evidence stays in the artifact directories below.

## Active bounded batch

- Runner: `reconciliation-20261102/parity-printer-fix-20261106/resume-pinned-lock/run.py`.
- Resumed 2026-11-06 02:57:43 UTC, PID 274726; overall limit 5,400 seconds.
- Read its `status.json`, then `result.json` and `results.json` when complete.
- One verifier worker, branch parallelism off. Each proof retains 600s whole-program,
  30,000ms assertion, 100ms solver-check limits; both outer watchdogs are 720s.
- No diagnostic instrumentation or solver logging in acceptance runs.
- No successor experiment queue. Let this batch finish; avoid repeated polling.

## Implemented correction and validation

The pinned parser associates `&&` and `||` to the right, while printer metadata
marked them left-associative. The isolated correction makes the printer match the
existing parser, preserving expression trees through export and reparse.
Only the pretty printer consumes this fixity metadata; parser and verifier rules
are unchanged. Native proof improvement is not established by this correction.

Ferrum checkout: `/home/will/wk/exp/ferrum-stdlib-roundtrip-fix`, branch
`fix/stdlib-ast-roundtrip`, based on `0e048a420c081f4c442dd04277bb016346de4e10`.
The shared Ferrum checkout remains untouched. Clean Prusti candidate:
`/home/will/wk/exp/prusti-stdlib-parity-fix`, exact H1 source plus this dependency.

The new structural regression failed on the old printer and passed with the fix:
5/5 roundtrip tests, 8/8 existing printer tests and 5/5 frontend tests passed.
Coverage includes all five four-operand conjunction/disjunction trees, mixed
permissions, guarded division, precedence and let scope. The printer correction
is committed and pushed as `931a2d55831ca3dffc34444e8ff0f88750f20bbc`.

The CLI build passed. The first batch then stopped after 321.36s because its Cargo
update supplied `--precise 0.1.0` for a Git dependency; Cargo interpreted this as a
Git revision. No native or CLI acceptance proofs ran. That failure is preserved.
Removing the invalid argument resolved the path patches but also changed an
unrelated Windows dependency edge; the preservation guard rejected that result
before any build or proofs. Both failures are retained. The current continuation
uses the original lockfile with only the three patched Ferrum Git source fields
removed. All unrelated packages and edges are identical; locked offline Cargo
metadata succeeds. It reuses completed tests/CLI build and rebuilds the same Prusti
candidate. It records source/binary provenance and runs full
Rust `mut_native_resource_early_pass` and
its exact `_false` control natively, followed by each generated export through CLI.
All four outcomes are recorded separately. Timeouts and wrong diagnostics fail.

## Corrected preparation finding

Prusti uses `NativeFrontendApi(minimal=false)`. Both paths already call
`SilFrontendPhases.prepare_verification` and share Session/backend preprocessing.
The earlier native AST capture was **before frontend preparation**, while CLI
captures included translated and prepared states. Different ADT node counts do
not establish missing native preprocessing. No duplicate preparation is added.

## Established evidence

- Frozen H1 native positive exceeded 600s twice; its exact negative passed.
- The identical saved export passed CLI without logging in 297.49s. Logging
  changed outcomes on both paths; logged runs are diagnostic evidence only.
- Compact AST capture completed in 72.38s. The first ownership package groups a
  pure suffix as `(A && (B && C)) && R` natively and `A && ((B && C) && R)` after
  parsing. Mixed permission conjunctions also regroup. All non-parenthesis tokens
  in the compared package match. The difference exists before CLI preparation.
- Minimal roundtrip completed in 21.55s: ordinary printing changes both left-grouped
  pure and permission controls; explicit parentheses preserve all three controls.
- These results establish a printer defect, not the full timeout's cause or full
  semantic parity. H2's pure-only reassociation still timed out.

## Preservation and stopping boundary

Integration baseline `c2631cfb0` / `stdlib-restoration-baseline-20261102` remains
unqualified; no ownership repair integrated. The six pending main files and all
frozen H1 binaries/source are hash-checked. Saved replay dependencies are retained.
Resume restoration only after parity is repaired or adequately explained; retain
the complete combined matrix requirement. Stop before D1–D6; no scheduled automation.

Evidence root: `target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.
Prior diagnostics: `parity-ast-20261106/compact/`, `roundtrip-import-fix/` under that
AST directory, `prefix-native-trace/`, `prefix-cli-trace/`, `prefix-cli-repeat-unlogged/`.
The initial environment-validation and recursive-Debug failures remain preserved.
Original inputs: `prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`.
Acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`; plan: `STDLIB_EXTERN_SPEC.md`.
