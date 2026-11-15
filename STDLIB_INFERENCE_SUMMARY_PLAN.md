# Independent inference and ownership-summary experiments

User direction, 2026-11-15: start both experiments independently. Annotation
reduction and bounded ownership proof work are separate acceptance criteria.
Growing regression counts alone satisfies neither. This replaces the previous
R3-before-R4 ordering; retain all restoration assertions, negative controls and
the final 790-client / 2198-check inventory. Stop before D1–D6; no automation.

| Deliverable | Current status | Required result |
|---|---|---|
| I1: count-loop inference | Qualified b30c4156f: 568/568 Ferrum; 10/10 focused both backends | Contract-only generic count; overflow checked; wrong variants rejected |
| O1: bounded mutable-loop ownership | Resource-only native loop 3/3; prerequisite regression running; value summary pending | Manual value invariants; checked split/update/join/drop; bounded backedge state |
| C1: combine inference and summary | Not started | Infer value invariants with identical ownership obligations |
| H1: original difficult prefix | Unqualified | Original assertions and exact negative; repeated native/CLI measurements |
| Final restoration qualification | Unqualified | Complete same-source Ferrum/Silicon matrix and library verification |

## Frozen starting points and work allocation

Integration compiler `ef0752077eece23b6eaac737023839bcbc00dc7b`, report
`4f49648a2`: Ferrum 555/555, 143/143 focused; whole-state feature 13/13 on both
backends. These counts establish regressions, not inference or loop-summary gains.
The after-call experiment is preserved at `2b63ad337` on
`stdlib/ownership-prefix-r3`: Ferrum 571/571, 156/156 focused, no material timing
regression. Its prepared Silicon run was unstarted and is retired from the
immediate queue; it is **untested**, not passed. Integration has not adopted it.
Detailed provenance remains in `target/stdlib-ownership-prefix-r3/` in the
preserved main worktree. Main's six pending source files stay untouched.

Use `stdlib/count-inference` in its own worktree for I1 and the existing
`stdlib/ownership-prefix-r3` worktree for O1. One agent and one verifier worker;
independence means separate implementations, prerequisites and measurements,
not concurrent backend workers. Neither experiment waits for the other's feature
completion. Keep the integration working tree stable between qualified increments.

## I1: count-loop inference

The required Rust source has only `#[ensures(result == values.len())]`, a shared
slice iterator, a zero-initialized usize counter, successful-next increments and
exhaustion exit. No body invariants, extra length parameter, ghost sequence,
user-written helper contract or automatic source rewriting is required.
The existing annotated `shared_loop_direct_pass` stays as a reference/control.

Extract one loop representation from MIR plus the existing selected iterator
contracts. It records immutable logical versions, entry relations, guarded
transitions, exits, source locations and the actual VIR observations for each
logical variable. For the first supported shape, these are counter `seen`,
remaining length `r`, and entry collection length `n`. Do not give arbitrary
Iterator implementations finite-sequence semantics; those relations must come
from the existing sealed model's selected contracts.

Generate a bounded set of affine equalities and inequalities using constants,
branch guards, contracts and update deltas. The paired +1/-1 updates suggest
`seen + r = n`; bounds include `0 <= seen <= n` and `0 <= r <= n`.
Use a local SMT session over the extracted entry/transition relations to eliminate
non-inductive candidates. Record candidate count, queries, results, elapsed time
and why each fact was retained. No whole-Rust rebuild/reverification per candidate.
Inference timeout/unknown grants no fact. Keep candidate generation and solver
budgets explicit and deterministic.

Translate survivors back to ordinary loop invariants. Native Rust verification
must independently check entry, preservation, permission well-definedness, overflow
and use at exit. Any inferred frame for an immutable input must itself be proved;
an immutable logical length is not a permission or an assumed heap frame.
Unsupported shapes must keep manual invariants available and report the missing
fact/unsupported extraction, rather than inserting an unproved summary.

Fixed initial controls: original contract-only positive; wrong result; omitted or
doubled increment; extra next; early exit; false assertion. Add counter-width and
boundary controls as supported arithmetic warrants. Rename locals and change
irrelevant source layout to show recognition is structural, not name matching.
Freeze exact expected diagnostics before qualification. Measure annotation count,
inference cost, generated proof size and native verifier work separately.

## O1: one parameterized ownership summary

First scope: scalar prefix update through a mutable traversal, with every yield
expired before the backedge. Keep explicit manual value invariants. Start with
the smallest native positive/negative pair and only its demonstrated prerequisites;
do not resume a broad prerequisite migration ahead of the experiment.

At the loop head, `Summary(owner, original, processed, current)` accounts for the
processed region, remaining tail, original-owner restoration obligation and **no
outstanding yield**. Record physical permissions and entry/post-call/current
snapshots explicitly. The value model is immutable data; it grants no owner read.

Checked symbolic rules split the tail, expose the selected resource, preserve the
rest, update its logical value, discharge the yield, and join the prefix. The
backedge consumes the previous summary and establishes exactly one replacement
summary. Iterator drop consumes that summary and restores the source. No list of
completed-yield histories may grow at the loop head. Do not weaken permissions,
restoration obligations, assertions or negative controls to obtain a smaller proof.

Reuse the checked inductive segment rules where applicable, but demonstrate actual
native Rust calls and loop obligations. Raw Viper models are diagnostic evidence
only. Distinct indices do not imply distinct physical addresses. Retained yields,
reference replacement and generic/reference-containing elements are later controlled
extensions; the original hard example remains the unchanged final target.

Measure before/after proof size, loop-head summary/history counts, generated
obligations, backend work and cold runtime on matching source/configuration. Show
that the representation is parameterized rather than unrolled and that equivalent
traversals of symbolic length retain bounded ownership state. If solver-work counters
are unavailable, report that limitation instead of substituting pass counts.

## C1 and H1: combine only demonstrated components

Value templates are unchanged regions, constant-filled prefixes and elementwise
relations. `*yield = 7` supplies a candidate template, not a fact. Prove empty-prefix
initialization, one-element extension, untouched-suffix framing and exit progress.
Compare manual values with O1, inference on the same ownership summary, and I1's
inference-only experiment so each source of improvement remains distinguishable.

For `&mut [&mut u32]`, reference slots and referent values have separate models.
Replacement requires a separate transfer rule. Escaping-borrow future values are
bound to specific loan identities and constrained by checked mutation/discharge;
they cannot be selected to satisfy a caller assertion. Do not expose the source
while conflicting loans remain live.

Run the original difficult positive and exact negative unchanged, then controlled
cold native/CLI repeats. Preserve effective configuration, frontend inputs, source
and binary hashes and solver evidence. Previous CLI/native timing discrepancies
are not declared fixed by passing these prerequisites.

## Validation and stopping rules

Ferrum is the development backend; Silicon runs at completed feature milestones
and final combined qualification, not on every small patch. Keep one verifier and
Ferrum branch parallelism disabled. Retain established 180/600-second qualification
limits; larger diagnostics are separate and never qualify a timeout as a negative.
Use five-minute sleeps between checks during long waits.

After two materially different hypotheses fail for one symptom, preserve the
reproducer and review the first missing/expensive obligation before another patch.
The next experiment must distinguish explanations. Integrate qualified repairs
incrementally, rerun affected passing families and investigate new failures or
material slowdowns. Commit/push coherent tested changes with explicit scope.

Complete I1 and O1 independently, then C1 and H1, then same-source complete
Ferrum/Silicon restoration qualification. Report implemented behavior, annotation
reduction, ownership-proof reduction, exact diagnostics, performance and restrictions.
If blocked, identify the unresolved obligation and the evidence/decision needed;
keep the task incomplete. Stop before D1–D6 and schedule no automation.

## Current experiment evidence (2026-11-15)

I1 prerequisite `c86eafa76` eliminates literal empty loop wands consistently.
The native manual positive and exact negative passed; all historical iterator
129/129 and borrow 208/208 checks passed without material per-case slowdowns.
Artifacts: `target/stdlib-inference-summary-20261115/count-empty-wand-regression-01`.
These counts are separate from inference results.

`count-inference-03` has 10/10 native focused results, including zero manual
invariants on the generic count, incorrect update/control-flow negatives, false
assertion, renamed locals, non-Copy elements and ZSTs. The first arithmetic
projection generated 39 candidates, made 88 local queries and retained a
conservation equality plus two bounds in about 27 ms. A checked shared-reference
frame preserves entry length metadata. Overflow and ordinary permission checks
remain enabled. Subsequent feature qualification on both the experiment and integration builds passed: 568/568 Ferrum suite checks, successful library verification, and 10/10 exact focused controls on each backend. See `STDLIB_COUNT_INFERENCE_EVIDENCE.json`. The full combined matrix remains pending.

Scope: one direct core shared-slice iterator, one Some/None loop, a returned usize
counter and a zero initialization. The finite-step lowering is specific to the
existing selected sealed model contract; it is not a general contract-to-CHC
translator. Unsupported constructors, nested loops and additional branches/calls
receive no inferred summary. Manual invariants remain supported. The configuration
`infer_loop_invariants` enables this bounded checked pass; disable it for A/B
measurements. Inference errors or unknown results grant no fact. Full Rust checks
independently discharge all inserted invariants.

O1 remains independent on `stdlib/ownership-prefix-r3`. Thirteen straight-line
native controls pass, including repeated yields, retained yields, early drop,
exhaustion and exact stale/false negatives. Three resource-only loop controls
also pass after retaining singleton restoration edges, framing the source's
reference slot, preserving the PCG projection's pre-call tail, and explicitly
importing a continuation's checked antecedent before applying it inside a package.
A minimal Viper positive/negative pair diagnoses that last transfer rule; native
Rust verification is the acceptance evidence. No permission is granted by the
immutable value model, and no required assertion is removed.

The PCG prerequisites resolve established region unifications after associated-
type normalization and preserve historical post-call projections when a moved
argument's storage expires. Empty shared-reference enum resources no longer
produce a conditional empty wand. The previously passing iterator library and
wrong-source diagnostic recovered after that representation repair.

Artifacts are under `target/stdlib-inference-summary-20261115/` in the preserved
main worktree: `o1-history-regression-01`, `o1-loop-history-native-01`, and the
small `o1-loop-history-model-01` reproducer. The complete affected Ferrum gate
`o1-native-gate-02` is in progress (596 required suite checks; 172 focused controls,
not an additive total). These changes are not integrated or qualified yet. The
manual bounded prefix/suffix value summary, inference combination, H1 and final
combined matrix remain pending. The next ownership milestone must demonstrate
bounded backedge state and functional guarantees, not merely more passing cases.

The integration worktree rebuilt compiler `b30c4156f` independently, retained the
same complete source manifest as the experiment, and repeated exact suite coverage
and the dual-backend focused gate. All historical cases passed without a material
per-case slowdown (>2x and +10s). I1 is accepted; O1, C1, H1 and final combined
qualification remain incomplete. No D1–D6 work or automation is scheduled.
