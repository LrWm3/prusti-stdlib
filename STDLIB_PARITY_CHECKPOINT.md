# CLI/native parity checkpoint — 2026-11-08

Read this first; detailed evidence stays in artifacts.

## Completed bounded solver comparison

- Runner: `reconciliation-20261102/parity-corrected-solver-trace-20261108/run.py`.
- Started 2026-11-08 15:18:49 UTC, PID 1990403; overall limit 1,800 seconds.
- Completed in 1,135.60s; no active job. See `review.json` and `comparison.json`.
- Native hit the 600s whole-program limit (618.91s command); CLI failed restoration
  after 492.42s with prover incompleteness `canceled`. These logged outcomes are diagnostic.
- First 39,757 commands match after only source-position qid normalization. At the
  first later divergence, the same 500ms query returned native `unsat` in 223ms
  versus CLI `unknown`/`canceled` in 513ms. Subsequent command order then differs.
- Quantifier labels already differ at raw command 7,116, so input equality is not
  established. Next isolate qid effects versus solver/resource variability by
  replaying preserved prefixes; no encoding change is justified by this evidence.
- Use frozen corrected native/CLI binaries; no rebuild or source repair. Run the
  original full Rust positive, then CLI on its exact just-generated export.
- Add identical `proverLogFile`/`reportReasonUnknown` diagnostic settings on both
  paths; preserve 600/30,000/100 limits, outer 720s, one worker and branch false.
- Preserve SMT files/hashes and compare actual command order/context: raw first,
  then source-position qids normalized. No expression reassociation or reordering.
- Logging has changed prior outcomes. This batch is diagnostic and cannot replace
  unlogged qualification; previous exact negatives remain recorded. No successor queue.

## Reviewed verifier-entry differences

After removing empty Seqn records and unwrapping single-statement Seqn with no
local declarations **for comparison only**, all four captures contain 73,012
retained node records. Each path is stable across repeats. Native and CLI differ
at just two expression kinds in the `ReadPerm` axiom: native `LtCmp`, CLI
`PermLtCmp`. Both lower through the evaluator to `TermFactory::less`; this source
review does not establish identical final solver contexts. Native expression
metadata also matches across both captures; callback internals remain opaque.
Rendered prepared text matches excluding comments. No verifier AST was normalized
or weakened. Evidence: `parity-entry-capture-20261106/structure-review-canonical.json`
and `metadata-repeat-review.json`; preliminary normalization reports are superseded.

## Completed capture — reviewed 2026-11-08

- Runner: `reconciliation-20261102/parity-entry-capture-20261106/run.py`.
- Started 2026-11-06 05:04:26 UTC, PID 1815812; overall limit 3,600 seconds.
- Completed all four captures in 706.36s. See `review.json`; new trace job above.
- Prepared Viper text matches between native repeats and native/CLI after removing
  comments. Captured solver settings match; configuration differences are only the
  frontend filename and `ignore_file`. Reviewed structural differences are described above.
- This diagnostic did not run proofs; parity and prior CLI timeouts remain unresolved.
- Separate Ferrum/Prusti worktrees rebuild diagnostic binaries. Instrument only
  actual `Session::verify` entry after each real frontend pipeline; record compact
  AST shape, metadata, prepared text and effective backend configuration.
- Capture native and CLI twice each, then deliberately return an error before
  backend verification. No acceptance proof or new ownership patch in this job.
- Compare within-path stability before cross-path differences. Callbacks and parser
  back-references remain opaque; hashes alone do not prove semantic equivalence.
- One worker, branch parallelism off. Build limits 900/1,800s, each capture 180s.
  Previous 180/600s qualification limits remain unchanged. No successor queue.

## Controlled repeat result

The uninstrumented repeat completed in 1,096.16s with all frozen inputs preserved:

| Positive run | Outcome |
|---|---|
| Original frozen H1 native | Passed, 201.28s; previously timed out twice |
| Corrected frozen native | Failed loop restoration at Rust line 24, 282.86s; previously passed |
| Corrected CLI, exact same saved export | Whole-program timeout again, 605.31s command time |

Repeated native exports match their respective prior programs excluding comments.
Thus native outcomes vary with unchanged binaries and rendered proof text. This
rules out attributing the observed native improvement to the printer correction
alone. It does not distinguish internal AST/context changes from solver/resource
variability. The failure diagnostic alone does not identify a solver timeout.
Machine load was recorded; causation is not established. The next discriminating
experiment captures actual verifier-entry inputs rather than trying another patch.
Evidence: `parity-controlled-repeat-20261106/results.json` and its command/log files.

## Completed candidate comparison

Printer fix `931a2d55831ca3dffc34444e8ff0f88750f20bbc` is pushed on Ferrum branch
`fix/stdlib-ast-roundtrip`. All 18 focused tests passed (5 roundtrip, 8 printer,
5 frontend); the roundtrip regression also failed on the original printer.
CLI and isolated H1-based Prusti builds succeeded. The uninstrumented batch finished
in 1,696.87s; main changes, frozen H1 and saved dependencies remain preserved.

| Original full Rust case | Native | CLI of its exact export |
|---|---|---|
| `mut_native_resource_early_pass` | Passed, 190.36s | Whole-program timeout at 600s; command 605.55s |
| `mut_native_resource_early_false` | Intended assertion at Rust line 69, 345.69s | Intended exhale at Viper line 11546, 294.63s |

**Parity remains unresolved.** Native passing once does not establish why earlier
H1 runs timed out, or qualify the complete restoration implementation.
The positive export preserves the previous non-parenthesis program tokens.

The raw runner misclassified the CLI timeout and negative. Audit corrected them
without changing original logs/results: `[timeout.error]` identifies the 600s limit;
`SpecBlockKind::Assert` emits `exhale(spec_expr)` and maps
`exhale.failed:assertion.false` to the Rust assertion diagnostic. CLI reached that
final false assertion after the value/suffix checks. The separate `assert false`
at line 11742 belongs to a cleanup block, not the intended negative control.
Evidence: `parity-printer-fix-20261106/resume-pinned-lock/audited-results.json`.

## Repair scope and preparation finding

The parser associates `&&` and `||` to the right, while printer metadata marked
them left-associative. The fix makes printing preserve the original expression
tree when parsed. Tests cover all five four-operand trees, mixed permissions,
guarded division, precedence and let scope. Parser and verifier rules are unchanged.

Prusti uses `NativeFrontendApi(minimal=false)`. Both paths already call
`SilFrontendPhases.prepare_verification` and share Session/backend preprocessing.
Earlier native AST captures preceded frontend preparation; their ADT node counts
cannot establish missing native preprocessing. No duplicate preparation was added.

Ferrum candidate: `/home/will/wk/exp/ferrum-stdlib-roundtrip-fix`, based on `0e048a4`.
Prusti candidate: `/home/will/wk/exp/prusti-stdlib-parity-fix`, exact H1 plus that
Ferrum path patch. Unrelated lockfile entries remain identical. Initial Cargo
invocation/resolution failures were rejected and preserved before proofs.
The shared Ferrum checkout remains untouched.

## Preservation and stopping boundary

Integration baseline `c2631cfb0` / `stdlib-restoration-baseline-20261102` remains
unqualified; no ownership repair integrated. Six pending main files and original
H1 source/binaries are preserved. Resume restoration only after parity is repaired
or adequately explained; retain complete combined matrix qualification. Stop before
D1–D6; no scheduled automation.

Evidence root: `target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.
Completed build/proofs: `parity-printer-fix-20261106/resume-pinned-lock/`.
Earlier evidence: `parity-ast-20261106/compact/`, `parity-ast-20261106/roundtrip-import-fix/`,
`prefix-native-trace/`, `prefix-cli-trace/`, `prefix-cli-repeat-unlogged/`.
Original native H1 timed out twice; its saved CLI export passed in 297.49s.
Logging altered outcomes and remains diagnostic only. Original build/inputs:
`prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`.
Acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`; plan: `STDLIB_EXTERN_SPEC.md`.
