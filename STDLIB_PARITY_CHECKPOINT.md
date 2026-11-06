# CLI/native parity checkpoint — 2026-11-06

Read this first; detailed evidence stays in artifacts.

## Active bounded repeat

- Runner: `reconciliation-20261102/parity-controlled-repeat-20261106/run.py`.
- Started 2026-11-06 04:37:23 UTC, PID 1478042; overall limit 2,400 seconds.
- Read its `status.json`, then `result.json` and `results.json` when complete.
- Sequential positives: original frozen H1 native; corrected frozen native; corrected
  CLI on the exact saved export. No rebuild, source change or diagnostic logging.
- Each proof retains 600s whole-program, 30,000ms assertion, 100ms check and 720s
  outer limits; one verifier worker, Ferrum branch parallelism off.
- Hash-check source, binaries, solver, dependencies and exports. Record machine load
  and compare repeated exports with previous programs, excluding comments.
- Test reproducibility before attributing native improvement to the printer fix.
  If stable, next compare ASTs at actual shared verifier entry and solver divergence;
  if variable, investigate identical-run variability. No successor patches queued.

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
