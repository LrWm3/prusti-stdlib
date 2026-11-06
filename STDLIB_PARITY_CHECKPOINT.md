# CLI/native parity checkpoint — 2026-11-06

Read this first when resuming; consult the full plan only as needed.

## Scope and state

- Current task: explain or repair Ferrum CLI/native parity before further ownership repairs.
- One bounded experiment per turn; save results, avoid repeated polling, then return.
- Initial investigation budget: 90 minutes of active investigation, excluding the interrupted days. This iteration used approximately 10 minutes.
- Integration baseline: `c2631cfb0` / `stdlib-restoration-baseline-20261102`, still unqualified.
- Six pending main source files match their saved hashes. Frozen H1 driver also matches its manifest.
- Initial AST diagnostic built successfully, then stopped before capture because its `PRUSTI_PARITY_*` environment names failed Prusti's strict configuration validation. Original failure preserved; no solver ran.
- Corrected environment capture then hit its 420s outer diagnostic limit while recursively formatting the CLI AST (2.7GB partial dump). No solver ran; this was instrumentation overhead, not a proof timeout.
- Compact capture completed in 72.38s. No background process or solver is active. Frozen binaries and pending main files remain unchanged. Current evidence: `parity-ast-20261106/compact/review.json`.
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

## Confirmed AST comparison (2026-11-06)

The instrumented export matches the frozen program excluding comments. Native,
CLI-translated and CLI-prepared captures contain 60,626, 57,664 and 73,397 nodes;
these counts include representation differences and do not measure obligations.

In the first `m_visit` ownership package, native groups a pure suffix as
`(A && (B && C)) && R`; the CLI groups it as `A && ((B && C) && R)`.
A is owner-address equality, B owner-length equality, C tail-address self-equality.
This corresponds to the saved short-circuit scope divergence. Mixed permission
conjunctions are regrouped as well. All 685 non-parenthesis tokens in the saved
first-package renderings match after removing comments. The package's Boolean
structure is unchanged between CLI translation and preparation, locating this
difference before the preparation phase.

H2's pure-only global right association neither reproduced the CLI's retained
subgroups nor aligned permission conjunctions. Its failure does not isolate this
specific roundtrip difference. ADT representation and metadata still differ too;
full semantic parity and the cause of the whole-program timeout remain unresolved.

## Next bounded experiment

Test a minimal native conjunction AST through printing and parsing. Then use
structure-preserving parentheses in a diagnostic export and check whether the
first package scope divergence disappears. Preserve the original H1 driver,
positive, exact negative and limits. No ownership patch or qualified repair yet.

**Completed runner:** `reconciliation-20261102/parity-ast-20261106/compact/run.py`.
It reuses the successful initial build and frozen-rendering comparison, and writes
compact node records instead of recursive Debug output. It retains tree structure,
purity boundaries, function names, leaves, triggers, positions and metadata tags.
Parser back-references and callback internals are opaque: complete metadata parity
is not claimed. Earlier failure artifacts remain intact.
The first runner, `parity-ast-20261106/run.py`, restores exact H1
source/dependencies and checks a fresh encode-only rendering with the frozen driver.
It builds an instrumented copy in `/home/will/wk/exp/prusti-stdlib-parity-h1`, captures
the native AST and the saved export after CLI translation/preparation, then stops.
It records AST shape/metadata files, configuration and source/build provenance.
The instrumentation never creates a solver session. A failed build, timeout or
rendered-program mismatch stops the job and records a failure; there is no repair queue.
See that directory's `plan.json`, `launch.json`, `status.json`, and `result.json`.

The comparison focuses on the package at SMT trace line 27693 and divergence at
native 30236 / CLI 30243. Further inspection must account for permissions,
obligations, triggers, metadata and verification scope as well as grouping.

The exact H1 worktree and saved dependencies have been restored at the original
paths; frozen H1 binaries survive. The instrumented cached build is isolated.
The old standalone CLI executable is gone; rebuilding it requires explicit new
provenance. Do not substitute a newer binary.

Evidence root: `target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.
Current audit: `parity-iteration-20261106/result.json` and adjacent trace excerpts.
Inputs: `prefix-h1-build.json`, `prefix-h1-repeat-native-inputs.json`,
`prefix-native-trace/`, `prefix-cli-trace/`, `prefix-cli-repeat-unlogged/`.
Full acceptance: `STDLIB_RESTORATION_ACCEPTANCE.json`; plan: `STDLIB_EXTERN_SPEC.md`.
