# R3: original resource-prefix proof

Baseline: R2 compiler `ecec87c3fe84379b607cb30e81951fa544ce7979`, report
`b24809d11`. Integration remains `stdlib/ownership-inference-m3`; experiments use
`stdlib/ownership-prefix-r3` in the existing isolated repair worktree.
Artifacts: `../prusti-stdlib/target/stdlib-ownership-prefix-r3/`.

The original 29 `mut_native_resource_*` clients are frozen byte-for-byte from
`c2631cfb0`, checked against `STDLIB_RESTORATION_ACCEPTANCE.json`. This family
supplements, and does not replace, the unchanged 790-client restoration target.
The original early-break positive and exact false control both have a recorded
600-second qualification limit; preserve it with a separate 720-second outer
watchdog. Other cases retain their recorded 180/600-second schedule.

| Current evidence | Result |
|---|---|
| Frozen R2 Ferrum / Silicon gates | Each 454/454; each 70/70 focused |
| Current direct-state integration Ferrum gate | 490/490; 96/96 focused; 37 specification + 1 binder test |
| Original prefix positive on R2 | Compilation/specification failure |
| Original exact false control on R2 | Wrong diagnostic |
| Prefix proof, inference, full restoration | Unqualified |

`original-pair-baseline-01` cannot reach verification: the quantifier parser rejects
`move` capture and the branch lacks `ResourceState`, resource observations,
mutable-iterator observations and `body_restore_prefix!`. These are prerequisites,
not a new solver timeout. Neither result counts as a passing negative control.

## Ownership invariant before restoration repairs

The physical owners are the slice's reference slots and the separate scalar
referents. A reference-slot snapshot identifies its historical target; it does
not describe or grant permission to that target's current value. The slice's
entry sequence fixes the original slot values and ordering. Entry resource states
capture the initial referent values under their permissions.

After creating the iterator, its saved post-call history identifies the original
owner and the remaining tail. Selecting one slot transfers its resource and the
corresponding referent resources to the yield, retaining the remainder's frame.
After `**value = 7`, expiry must restore the slot and its referent resources before
that yield joins the processed prefix. The prefix stores immutable slot and
resource-state values; these are not permissions. The remaining tail preserves
its original suffix states. At the loop head a bounded checked summary must
account for prefix, any outstanding yield, and tail without accumulating an
unbounded list of call histories.

On early exit, discharging the iterator obligation rejoins the processed prefix
and untouched suffix into the original physical owner. Only then may the original
owner be read to establish unchanged slots, updated referents equal to 7, and the
unchanged suffix. Reference replacement must retain obligations for displaced
and replacement targets. Distinct logical indices do not imply distinct physical
addresses, including for generic or zero-sized elements.

## Ordered experiments

1. Restore `move` quantifier capture from the later parser, preserving it on both
   body and trigger closures. A small native positive/negative pair isolates this
   syntax needed by the original proof. Preserve the existing trigger validation
   and quantifier well-definedness checks; add no ownership or solver assumptions.
2. Specify and qualify permission-checked resource observations and the minimum
   indexed-resource representation. Use concrete scalar/reference/sequence cases
   first, with wrong-state and missing-permission controls, then generic and ZST
   controls. An observation must never manufacture a permission.
3. Restore mutable iterator operations and manually supplied bounded prefix
   summaries one checked rule at a time. Keep original callee/caller expiry
   obligations and exact value/frame controls. Do not import the newer restoration
   encoder wholesale or claim inference repairs missing ownership resources.
4. Run the unchanged full Rust early-break pair, targeted prefix/suffix/replacement
   controls and three cold native/CLI repeats. Compare proof structure and costs
   under matching inputs; under 60 seconds remains a stretch target.
5. Proceed to bounded value inference and complete restoration qualification only
   after the required proof and controls pass. Silicon is reserved for major
   feature completion and final qualification. Stop before D1–D6; no automation.

Keep one integration candidate, one worker and Ferrum branch parallelism disabled.
After two failed hypotheses for a symptom, review the model before any successor
patch. Raw Viper edits are diagnostic only. Preserve exact programs, source and
binary hashes, settings, logs and every unsuccessful result. No timeouts, skipped
cases or wrong diagnostics satisfy acceptance.

## Quantifier prerequisite evidence

The small native controls reproduce the capture syntax failure on R2. The narrow
parser backport from `d4d4bc72bb` passes all 37 specification tests and repairs the
universal positive and exact negative. Its initial existential positive still
fails. `quantifier-witness-diagnostic-01` distinguishes the causes: removing
`move` has the same failure, while adding the checked `marker(bound) == 4`
assertion proves the unchanged existential with its explicit trigger.

The positive control therefore includes that checked ground instance; its two
original quantified assertions and the negative are unchanged. This is a proof
annotation, not an encoder inference result or weakened obligation. The original
unannotated source/results and both diagnostic variants remain preserved. The
original prefix sources and 29-case target are untouched. Automatic witness
selection remains outside this parser repair and may be considered in R4.

The parser prerequisite is integrated at `e3db7a3185ca5fa2f4bef2f262aaaf647a17e36b`.
Its own integration rebuild matches 457/457 Ferrum suite checks and 72/72 focused
controls, plus 37 specification and one binder-scope test. Suite time is 696.54s;
no case crosses the fixed material-slowdown threshold. Source, binaries and the
six pending main files are preserved. See
[parser qualification](STDLIB_PREFIX_QUANTIFIER_EVIDENCE.json). R3 remains open;
next restore permission-checked resource observations and iterator summaries.

## Slice observations: invariant and hypothesis

The next fixed target is the original ten length/contents observation clients,
including current/entry/expiry positives, wrong value/history controls, and
missing-permission controls in specifications and ordinary pure functions.
`original-pair-baseline-02` confirms the quantifier syntax error is gone; resource
APIs and the prefix macro still prevent the unchanged pair from verifying.

The reference snapshot owns address and metadata observations, not its referent.
Length comes from that metadata and is available without element permissions.
Contents reads the slice predicate at the reference's physical target; that read
requires its existing permission. The entry, current and expiry heaps must be
selected for the contents read itself, not only for the reference snapshot.
`old` uses the saved entry/call label, and `before_expiry` uses the bound wand LHS
heap. A saved contents sequence never grants permissions to reference-containing
elements' referents. No loan is created or discharged by either observation.

Hypothesis: import only `mut_slice_len` from `ec0081c6c` and the permission-checked
`mut_slice_contents` translation and temporal selection from `aa520ad34`. Keep
existing slice predicates, generic casts and old/before-expiry restrictions.
Reject unsupported relational/nested temporal uses rather than choosing a heap
implicitly. Qualify all ten original native controls and affected families before
integration. Referent/resource-state and iterator-prefix support remain separate.

Slice observations are integrated at `ce448169f9488de9252b3f8d0ca325334ac065e6`.
All ten unchanged original controls match, including both missing-permission
rejections. The experimental and own integration gates independently match
471/471 Ferrum checks and 82/82 focused controls, plus 37 specification and one
binder-scope test. Integration suite time is 703.00s with no material per-case
slowdown. See [slice observation evidence](STDLIB_SLICE_OBSERVATION_EVIDENCE.json).
The original prefix pair remains unqualified; next specify and qualify direct
referent-state observations before indexed states and iterator summaries.

## Direct referent-state observations: model before implementation

Freeze the five original `mut_resource_state_*` clients: equal/different scalar
states, a false unchanged-state postcondition, missing permission for an opaque
value, and executable misuse. The initial rule observes values only; it neither
creates nor transfers a loan or resource.

For a mutable-reference snapshot `r`, its address/metadata identify the physical
target. Read the target's current generic snapshot only through its existing
predicate, then construct `Reference(stored_value, nested_state)`. Equal target
values produce equal states even at different addresses. Replacing a reference
slot does not redirect a previously captured state. Entry and expiry observations
use the same explicit heap selection already qualified for slice contents.
Known resource-free primitive/shared-reference/raw-pointer values have empty
indirect state. Their own stored value is included only when reached through an
owning mutable reference. Do not infer emptiness for generic or opaque values,
arrays, recursive structures or unknown ownership wrappers.

For those unsupported structural groups, observation requires a separate opaque
resource predicate indexed by snapshot, type and metadata. This increment emits
no operation that grants that predicate. It therefore rejects the original
generic missing-permission control conservatively; it does not claim generic
resource transfer support. Finite field/enum composition and indexed collection
states will require their own checked rules and native controls. The state domain
contains immutable logical values and grants no heap access. Preserve the actual
physical predicate reads, all original negative diagnostics and executable-use
rejection. No physical-address injectivity or future-value assumption is added.

Direct referent-state observations are integrated at
`8a82d279f136f2784841e009b50fc09aad43de6b`. The five original clients and nine
additional history/nesting/expiry/ZST/permission controls match. Both experimental
and own integration gates independently match 490/490 Ferrum checks and 96/96
focused controls, plus 37 specification and one binder-scope test. Integration
suite time is 713.42s with no material per-case slowdown. Source, binaries and
pending main files are preserved. See
[direct-state qualification](STDLIB_RESOURCE_OBSERVATION_EVIDENCE.json).
The first extended run exposed a reserved `outer` identifier in the new nested
controls; renaming it preserved all operations, assertions and expected diagnostics.
Original clients remain unchanged. R3 remains open: next establish permission-
checked indexed observations and the resource ownership needed by prefix summaries.

## Pointwise sequence observations: invariant before implementation

The selected failure family is observing a single element of a saved logical
sequence. Freeze the original bounds, negative-index, missing-permission and
executable-use controls. Add native value/history, repeated-reference, zero-sized
and exact wrong-state controls. The full original slice-reference positives still
need indexed ownership and remain in the restoration inventory.

A sequence is immutable data and grants no resource. At an index proved in range,
cast its element snapshot into the actual element type and apply the qualified
direct-state observation. Every target read uses an existing physical predicate.
Saved observations remain values, while a new observation reads the current heap;
entry and expiry selection use the existing temporal wrapper. Duplicate element
snapshots may use the same physical permission for repeated reads; no quantified
permission or address-distinctness axiom is introduced. No loan or obligation is
created or discharged by this operation.

Hypothesis: a pointwise lowering plus a checked bounds identity function suffices
for this prerequisite. Bounds must remain explicit even for resource-free element
types whose state is constant. Unknown structural elements retain the existing
opaque permission requirement. Whole-sequence capture, ownership transfers and
loop-prefix summaries are separate work and receive no assumed permissions here.

The first lowering matches 7/13 controls. The first missing obligation is a saved
sequence's address-to-restored-owner correspondence: its original generic
predicate can have been converted back to concrete storage. Controlled native
variants show that the same assertions pass while an executable reference use
keeps the loan live. A later ghost observation merely moves the failure to its
own last use. Direct current-reference observations also pass.

Next hypothesis: at acyclic current specification sites, resolve a resource read
against PCG-owned scalar/unit roots that were actually borrowed. Require explicit
physical address and type equality before choosing that root's existing concrete
predicate snapshot, cast back to the common logical value type. Otherwise retain
the original generic predicate read. Both branches check their actual permissions;
there is no mask-dependent logical value, new permission or stale final-value
assumption. Skip historical observations, wand binders and loop invariant contexts.
The original thirteen controls remain unchanged. Detailed comparison and rejected
alternatives are in `pointwise-state-model-review.json` in the artifact root.

The owner-read candidate repairs duplicate/ZST controls but initially selects a
concrete owner too early in the history control (11/13). Review identified an API
misuse: `computed_owned_capability` explicitly ignores borrowing. Require the
tracked `place_capability == Exclusive` instead; the live-loan read must retain its
generic predicate, and only the restored read may use concrete ownership. The
failed snapshot precondition and full candidate remain preserved.

All original thirteen controls now match. Expanded tests preserve a newly exposed
last-use ghost-call failure: apply the same current-owner resolution at heap-reading
builtin call sites, using the tracked pre-main PCG state after operand repacking.
The new successive-update test initially violated Rust borrowing by assigning the
owner while its saved reference remained live; update through the existing mutable
reference instead. Both observations and final assertions remain unchanged, and
the rejected sources stay in the artifacts.

The expanded native gate matches 117/117 focused controls and both binder/history
unit tests. Its ordinary Rust borrow-suite build exposes another existing
prerequisite: erased ghost blocks still type-check their bodies, but the M3 runtime
contracts lack `Ghost<T>: Deref`. Import only the non-verifying `Deref` stub from
`f16d6eafc`; it panics if evaluated and ghost bodies remain erased. The verified
implementation, observations, assertions and ownership rules are unchanged. The
failed gate and successful remaining suites are retained; complete qualification
requires a new gate on the combined source.
