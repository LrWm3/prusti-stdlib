# CLI/native parity checkpoint — 2026-11-06

Read this first when resuming; consult the full plan only as needed.

## Scope and state

- Current task: explain or repair Ferrum CLI/native parity before further ownership repairs.
- One bounded experiment per turn; save results, avoid repeated polling, then return.
- Initial investigation budget: 90 minutes of active investigation, excluding the interrupted days. This iteration used approximately 10 minutes.
- Integration baseline: `c2631cfb0` / `stdlib-restoration-baseline-20261102`, still unqualified.
- Six pending main source files match their saved hashes. Frozen H1 driver also matches its manifest.
- Initial AST diagnostic built successfully, then stopped before capture because its `PRUSTI_PARITY_*` environment names failed Prusti's strict configuration validation. Original failure preserved; no solver ran.
- Corrected background capture launched, PID `3436732`, using `STDLIB_PARITY_*` names and the cached build. Overall limit: 1,200 seconds. Read `parity-ast-20261106/retry-env/status.json` and eventual `result.json` before further work.
- The last exemplar diagnostic completed: positive 168.13s, exact negative 265.10s; diagnostic only.
- Unstarted `trace-prefix-both-state.py` retired at the user's redirect.
- Retain all original acceptance criteria, one verifier worker, branch parallelism off, 180/600s limits. Stop before D1–D6; no scheduled automation.

## This iteration: saved-trace and pinned-source audit

The actual SMT `set-option` sequence matches through the first meaningful
divergence. Both runs use the recorded 600s whole-program, 30,000ms assertion and
100ms check limits. Outer watchdogs differ (720s native, 660s CLI); neither fired.
This is not yet a complete effective-configuration or plugin audit.

Inside the same ownership package, native evaluation enters a short-circuit
scope for address equality A within a pure conjunction. CLI evaluation evaluates
A separately and enters its next short-circuit scope for length equality B.
The enclosing package renderings also regroup conjunctions containing permissions.
Earlier H2 changed only pure conjunction association and still timed out.

**Next hypothesis:** boundaries between permission and pure conjunctions change
symbolic execution and incremental solver context. This is an observed structural
lead, not an established timeout cause or semantic defect. Logging affected prior
outcomes; these traces remain diagnostic.

## Next bounded experiment

**Current runner:** `reconciliation-20261102/parity-ast-20261106/retry-env/run.py`.
It reuses the successful initial build and frozen-rendering comparison, renames
only the diagnostic environment keys, and performs an incremental rebuild/capture.
The first runner, `parity-ast-20261106/run.py`, restores exact H1
source/dependencies and checks a fresh encode-only rendering with the frozen driver.
It builds an instrumented copy in `/home/will/wk/exp/prusti-stdlib-parity-h1`, captures
the native AST and the saved export after CLI translation/preparation, then stops.
It records full ASTs, shape/metadata files, configuration and source/build provenance.
The instrumentation never creates a solver session. A failed build, timeout or
rendered-program mismatch stops the job and records a failure; there is no repair queue.
See that directory's `plan.json`, `launch.json`, `status.json`, and `result.json`.

Compare ASTs at native Session entry and after CLI frontend processing, focusing
on the package at SMT trace line 27693 and divergence at native 30236 / CLI 30243.
Check permissions, obligations, triggers, conjunction boundaries, metadata and
verification scope. Use separate diagnostic instrumentation; preserve the frozen
H1 driver and original positive/exact negative. Do not repeat H2 or patch ownership.

The temporary H1 worktree and pinned CLI executable are gone. Restore isolated
H1 source from baseline plus `prefix-h1.patch`, and dependencies from the saved
manifest, before any native replay. The frozen H1 binaries survive. Reconstructing
CLI tooling needs explicit new build provenance; do not substitute a newer binary.

Evidence root: `target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.
Current audit: `parity-iteration-20261106/result.json` and adjacent trace excerpts.
Inputs: `prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`,
`prefix-native-trace/`, `prefix-cli-trace/`, `prefix-cli-repeat-unlogged/`.
Full acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`; plan: `STDLIB_EXTERN_SPEC.md`.
