# R2: checked local ownership transfers

Baseline: `56716a5ef`, whose compiler source is the qualified `2c5484c00`.
Worktree/branch: `prusti-stdlib-m3-repair`, `stdlib/ownership-core-r2`.
Integration: `prusti-stdlib-ownership-inference`, `stdlib/ownership-inference-m3`.
Artifacts: `../prusti-stdlib/target/stdlib-ownership-core-r2/`.
The six pending main-tree files and all R1/H1 artifacts remain preserved.

Current checkpoint: `1ef39f2c29364eab505773a0f9a4f5f9f75fc419`. Its own integration
rebuild matches **422/422 Ferrum suite checks**, **48/48 focused controls**, and
all **36 specification tests**. Escaped `mem::replace` references now restore both
pointees, including reversed use order, a still-live displaced reference, and an
untouched replacement target. Exact wrong-target, value and frame controls reject.
Both original escaped-reference sources are unchanged. Access preparation exposes
an available view; PCG-directed transitions preserve resource multiplicity and
check value correspondence even when other destination units are already held.
See [resource-view qualification](STDLIB_RESOURCE_VIEWS_EVIDENCE.json).

The experimental and own integration builds independently match 422/422; their
counts are not combined. Integration suite time is **684.01s**, versus 601.54s for
the smaller previous gate. No integration case crosses the fixed >2x/+10s slowdown
threshold. An experimental range-proof outlier at 19.74s is retained: controlled
native replay with identical exported Viper takes 9.35s, and integration takes
9.76s, versus an 8.80s baseline repeat. Proof text grows, and no speedup is claimed.
No timeout, wrong diagnostic, skip or unresolved failure remains in these gates;
17 intentionally unsupported clients match their required diagnostics.

This completes the escaped-call increment, **not R2**. Selected-element rules,
continuation/summary semantics and the R2 Silicon feature gate remain open.
The original difficult prefix proof and inference remain R3/R4, and the complete
790-client/2,198-check restoration target remains unqualified. Stop before D1–D6.

Previous local-slot checkpoint `7814eccc4` matched 411/411 suite checks and 38/38
focused controls; its saved exploratory 412/414 target exposed the two escaped
failures repaired here. See [its evidence](STDLIB_REFERENCE_SLOT_EVIDENCE.json).

Previous identity checkpoint: `d0b8b0ac95babfaea4fc618e67bb46fe606a4f2d`, with PCG
`77e2a4a` (only the existing reservation-location getter was made public).
Both the isolated experiment and own integration rebuild independently match
**408/408 Ferrum suite checks**, **36/36 focused controls**, and **36 specification
unit tests**. No timeout, wrong diagnostic, skipped or untested required case
remains in these gates. The baseline matches 13/14 new identity controls: its
branch-selected-owner positive fails with a missing-permission diagnostic. The
new compiler matches 14/14, preserving the exact wrong-frame control. Saved
creation targets replace historical-slot rereads during local restoration.

The native programs exercise three-level reborrowing, guarded parent links,
successive children with intervening mutation, slot retargeting and early exits.
Deferred values are assigned after checked restoration; they are never assumed
while pending. That checkpoint supports acyclic scalar loans and direct PCG parent
edges. Loops, package/escaping continuations and non-scalar transfers retain the
earlier encoder. See [identity qualification](STDLIB_OWNERSHIP_IDENTITIES_EVIDENCE.json)
for exact compiler/source provenance, every suite result and preserved programs.
The integration suites took 639.68s, versus 551.88s for the smaller preceding gate.
Extra state checks increase proof size and cost; no speedup is claimed. No case
crossed the fixed material-slowdown threshold (>2x and >10s).

## First increment and falsifiable hypothesis

Introduce a small transfer IR between MIR/PCG and VIR for scalar mutable loans.
Freeze the physical target and value before changing representation, consume the
existing permission conversion, and check the destination permission and value.
Hypothesis: native scalar mutation and nested reborrowing preserve exactly that
target/value correspondence; the existing conversion contracts suffice to prove
it without any new assumptions. Wrong restored values, stale entry values and
wrong frames must still fail at their original assertions.

This first increment establishes a checked local rule and its immutable witnesses.
It does not implement an unresolved prophecy variable, eliminate the heap encoding,
claim faster verification, or complete R2. Keep it behind
`PRUSTI_CHECKED_SCALAR_LOANS=true` while qualifying the supported scope.

## First increment qualified (2026-11-15)

Implementation `f559ae610dcad2f71338b773d79b2f03af4d8d48` is integrated and rebuilt
from the integration branch. Ferrum matches **399/399 suite checks** and **30/30
focused controls**; all 36 specification unit tests pass. Exact matrix coverage is
293 native clients (88 positive, 188 intended rejections and 17 expected unsupported
diagnostics), eight library verifications, eight compilation checks, 88 runtime
checks and two structural trigger checks. The focused controls are a subset of
those clients, not extra unique coverage. No timeout, skipped case, unexpected
acceptance, wrong diagnostic or unresolved verification failure occurred.

The eight new native controls also match separately on the frozen qualified R1
compiler and the experimental compiler with the option both disabled and enabled.
Generated programs confirm acquire and resolve obligations actually execute,
including all three levels of nested reborrowing. Each target/value witness has
one assignment per dynamic transfer, and the new rules emit no inhales. The
retargeting control also exercises the existing historical-endpoint fallback;
it does not qualify a new rule for reference-containing replacement.

The experiment and own integration rebuild each independently match 399/399;
their counts are not combined. Source manifests cover 2,373 Prusti files and
1,497 PCG files. Six compiler binaries and the source remain unchanged during
each gate; the main tree's six pending files remain intact. PCG stays at
`e54bf675468d`, Ferrum at `931a2d5`, and the R1 toolchain/settings remain pinned.
Detailed expectations, diagnostics, per-case timings, source/binary hashes and
artifact references are in [the evidence record](STDLIB_OWNERSHIP_CORE_EVIDENCE.json).

The integration suites take **551.88 seconds** with one worker, branch parallelism
off, cache disabled, overflow/panic checks enabled, and the original 90/300-second
M3 limits. No case exceeds the predeclared material-slowdown threshold (both 2x
and +10s) against the experimental gate; the experimental gate has none against
R1. Different coverage and ordinary timing variability prevent treating this as
a demonstrated speedup. Silicon is reserved for the completed R2 feature sign-off.

The implementation is opt-in. For example, after building this branch:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite borrow --backend Ferrum --checked-scalar-loans
```

**R2 remains incomplete.** Persistent identities and direct child/parent deferred
resolution are now implemented in the later checkpoint above. Next qualify
reference-containing replacement and one selected element, with exact value and
frame negatives, and specify the remaining continuation boundary before R2's
Silicon gate. Pending values stay unavailable to ordinary assertions. The
difficult prefix proof and inference remain R3/R4; no D1–D6 work or automation is
started.

## Ownership invariant, written before implementation

### Third increment: reference-containing slots

Hypothesis: the same checked representation transfer applies when the borrowed
value is itself a reference. Its saved/deferred snapshot describes the reference
slot's address and metadata, not an observation of the pointee's current value.
The pointee is a separate physical owner with a separate loan identity and
permission obligation. Replacing the slot must not change the old loan's target.
No facts may be read from the arbitrary referent-value field of an impure reference
snapshot. Preserve the existing indirect predicates, call postconditions and
expiry wands; they still account for both the displaced and replacement pointees.

First extend only to reference-typed values (including mutable references, which
are non-Copy). Qualify local slot replacement and an escaped old reference returned
by `mem::replace`, preserving exact wrong-original-target controls. This does not
yet generalize the rule to arbitrary aggregates or select collection contracts.

The first native experiment (`reference-focused-01`) matches 14/18 controls;
both replacement positives and both negatives fail with missing permissions.
The qualified compiler also matches 14/18. Merely admitting reference snapshots
does not repair restoration. No failing negative counts as a successful control.

For the smallest local case, let `r` be the outer reference slot, `s` the stored
reference slot, and `a`/`b` the displaced/replacement scalar targets. Before the
last assignment, the encoder owns `C(r)`, `C(s)` and `C(b)`, with the old value 7
preserved by `G(a)`. After assigning 9 it folds `C(b)` into `G(b)`. The creation
record for the outer loan still names `s`; its entry reference names `a`, whereas
its current reference names `b`. These snapshots do not grant access to either
scalar. There is no call or post-call history in this smallest case.

The first missing obligation is `C(s) -> G(s)`: PCG schedules outer-loan removal
before removing the place expansion `r -> *r`. Resolution consequently requests
`G(s)` while `C(s)` is still available. The delayed fold also reads `r` after its
permission has been weakened. This precedes restoration of the scalar owners.

Second hypothesis: schedule that existing expansion fold before the corresponding
loan resolution, retaining both conversions and all value assertions. Match the
exact labelled reference place and identical edge guards; cross only encoder
no-ops and weakening of that reference slot, never another resource transfer.
The fold consumes `C(s)` and produces `G(s)`; resolution consumes it and publishes
the checked current `C(s)` snapshot. It cannot discharge `a` or `b`; their original
loan identities and generic scalar resources remain separate. A changed guard,
historical epoch or intervening resource operation prevents this local reorder.
Test the unchanged 18 native controls before broadening this rule. The escaped
`mem::replace` case has additional call histories and remains separately unresolved
unless its complete native proof and exact negative also pass.

The first scheduling implementation (`reference-focused-02`) also matches 14/18,
but emits zero reorders: it matched `BorrowPcgExpansion::Place`, whereas the
reference dereference is a `Deref` edge. After the two unsuccessful experiments,
the model review preserved a PCG visualization of the unchanged 12-line local
reproducer. All three operations are in `bb0[21] PreOperands`: borrow removal at
action 6, slot weakening at 9, and dereference folding at 12. This rejects a
cross-phase explanation and identifies the missed edge variant. The next test
matches that actual `Deref` edge under the same restrictions, requires a generated
reorder, and reruns the native controls without visualization. Detailed evidence
is in `reference-model-review.json`; no solver trace was enabled.

`reference-focused-03` emits the required reorder and matches **16/18**: the local
replacement positive passes and its exact wrong-target control rejects; all prior
14 controls still match. Both escaped-call controls remain internal errors. Freeze
this 18-case target in `reference-acceptance.json`. Qualify the local repair against
the previous 408-check gate plus its two native clients and one runtime check
(411), with 38 focused controls. Keep the two escaped-call sources in the frozen
target fixture until their separate repair; excluding them from this increment's
gate does not count them as passed or remove them from R2 acceptance.

### Fourth increment: escaped-call resource views (qualified above)

The unchanged escaped-reference positive still fails at the `make_concrete_u32`
needed by `*stored = 9`. A bounded native diagnostic identifies its argument as
the current target of `stored`: the original right-hand loan already restored
that scalar to its concrete predicate. Extending the old reference's liveness or
reversing the two uses instead fails while reading the still-generic `stored`
slot. Omitting the new-reference mutation passes. These three diagnostic variants
do not replace either original acceptance control; see `reference-escape-order-01`.
The temporary one-line error logger was removed after preserving its source,
binaries and result. No SMT query trace is enabled.

Let `s` own the stored reference slot and `a`, `b` own the old and new scalars.
At entry the slot names `a`; the call's pre-state captures that reference and the
replacement naming `b`. The existing `mem::replace` postconditions establish that
the returned reference names `a` and the slot now names `b`. They return `G(s)`;
the generic scalar resources `G(a)` and `G(b)` are framed through the call. Saved
loan targets identify the scalar obligations independently of the current slot.
No snapshot of a reference supplies the pointee's current value or permission.

The representation expected by a later place access need not be the representation
left by PCG's independent lifetime and place events. The proposed local rule is
`normalize(a, C)` or `normalize(a, G)`, where `C` and `G` are the exact concrete and
generic predicates for that value/type at the same captured physical address.
If the destination's full permission is already held, assert it and preserve it.
Otherwise assert the source permission, capture its current value, execute the
existing representation conversion, and assert destination permission and value
correspondence. A permission query selects an existing resource; it neither
creates one nor identifies distinct logical owners by address equality. Missing
resources and insufficient permission must fail. Every conversion frames all
other predicates. Existing generic caster contracts remain trusted translation.

First restore the already-implemented `PredicatePerm` expression interface from
the later tree, with matching Ferrum and JVM conversions and a focused adapter
check. Then test normalization only for the core's supported scalar/reference
values in acyclic executable code. Keep package scripts, loop summaries, opaque
generic values and indexed/zero-sized resources outside this first rule. Historical
addresses may be frozen, but resource selection and value observation occur in the
current heap. Do not normalize a saved heap observation as if it were current.

PCG loan expiry still creates the same discharge obligations. Normalize their
source representation before the original checked resolution and publish deferred
values only afterward. A physical-view conversion by itself neither discharges a
loan nor makes a pending deferred value available. Reacquisition and executable
place reads must likewise establish the concrete view they actually need.
Hypothesis: these checked identity/conversion alternatives repair both lifetime
orders without changing call specifications, permission quantities or negative
assertions. Require the unchanged 18 native controls, missing-resource/value/frame
rule controls and every affected existing family before integration. If this
hypothesis fails, inspect its first failed obligation before proposing a second.

The first implementation (`resource-view-focused-01`) matches only 7/18 and is
not integrated. Bounded native error logging identifies a scalar's initial borrow
as the first failure, before any call-restoration event: the selector tested
`perm(C) == write` and entered the generic branch despite a concrete resource
being available. Predicate permission can accumulate above `write`; different
local addresses have not necessarily been proved distinct at that point. The
correct availability condition is `write <= perm(C)`, preserving any extra
permission as a frame. Add an adapter control with two predicate permissions,
consume exactly one and prove that one remains; then rerun the unchanged native
target. No physical-address disequality is added to make the selector succeed.

`resource-view-focused-02` matches 15/18: all original 14 scalar/identity controls
and the escaped wrong-target control match. The model review after these failed
candidates identifies two specific remaining obligations. In local replacement,
PCG weakens/exhales the dead outer slot, then emits a capability-regain bookkeeping
event for it. Normalizing every such event incorrectly requests a resource that
has already been discarded. In escaped replacement, the final current assertion
on `right` requests `C(b)` while the last reference use left `G(b)`. Both diagnoses
are confirmed by bounded native errors in `resource-view-diagnostic-02` and the
unchanged generated programs. The next experiment normalizes actual current
dereferences, operands and accessible current specification captures, plus the
original checked loan discharges; bookkeeping regains remain no-ops. Keep saved
heap reads and packages unchanged. This distinguishes use-site resource exposure
from indiscriminate eager restoration; do not remove either final assertion.

The third candidate (`resource-view-focused-03`) matches the unchanged **18/18**
native targets without diagnostic logging. The extended `resource-view-controls-01`
matches **23/23**, adding both reference-use orders, a still-live escaped reference,
an untouched replacement target, and wrong-value/frame controls. The original
escaped positive and exact negative remain byte-for-byte unchanged. Adapter
tests also check zero, one and two predicate permissions, including rejection
of access without a resource and preservation of the extra permission.
These focused results precede full affected-family qualification; this candidate
is not yet accepted into the integration branch. No Silicon feature sign-off or
R2 completion is claimed.

The first full gate (`resource-view-gate-01`) is **418/422**, with **44/45** focused
controls and all 36 specification tests. All sources and binaries are preserved;
there are no material timing increases. The four regressions are `frame_pass`,
`index_pass`, `lookup_pass` and the exact negative `nested_footprint_wrong`.
Do not integrate this candidate. The latter's generated continuation requires two
generic resource units on a path where the returned reference can denote the
holder's existing target. Its preceding dereference fold sees an already-held
generic unit and skips converting the concrete unit. Destination availability is
therefore insufficient to replace a structural transfer: it loses the required
resource multiplicity. The collection failures report missing value relations;
the next experiment tests whether the same skipped-transfer explanation applies.
No distinct-address assumption repairs this model.

Next discriminating experiment: distinguish **exposing a view for an access**
from **performing a PCG-directed transfer**. Exposure keeps an already-available
destination. A structural transfer instead consumes one available source unit
and proves its current-value correspondence even when destination units also
exist; only an absent source permits the checked destination identity alternative.
This preserves framed destination units and the original transfer's quantity.
Keep access preparation unchanged, since converting additional framed source
units during every read would again steal continuation resources. Check the four
native regressions, unchanged 23-case restoration controls, and then all families.

`resource-view-controls-02` matches **27/27** without diagnostic instrumentation:
all four regressions recover, including the exact nested-footprint assertion
diagnostic, and all 23 restoration controls remain matched. This supports the
resource-count/value-transfer explanation. Full affected-family qualification
is required again on this revised source before integration.

### Fifth increment: one selected array element (planned, not qualified)

Start from `1ef39f2c2`, with its qualified local/escaped reference rules. First
freeze native scalar, generic/non-`Copy`, zero-sized and reference-containing
array controls with exact wrong-value/history/frame or non-vacuity negatives.
Run them unchanged on this compiler before extending the encoding. Keep failed
controls in the target; do not alter array contract selection to make them pass.

The physical owner is the array predicate `A(a,T,N)`. Selection at logical index
`i` transfers it into the element resource at `e = index_ref(a,i,T)` and the
opaque remainder predicate `F(a,i,T,N)`. `F` identifies the owner and selected
logical slot; it is not inferred from inequalities between element addresses.
The existing array split/join methods are trusted representation contracts, not
new proved lemmas. Keep their bounds obligations at the Rust access, and make
the resource/value correspondence around each call explicit and checked.

| Stage | Resource and immutable observations | Required check |
|---|---|---|
| Entry/split | Own `A`; freeze `a`, `i`, `e`, and array model `S0` before splitting | Owner permission, selected element equals `S0[i]`, and frame snapshot equals `S0` |
| Element access/loan | Own the selected element plus `F`; existing loans govern exclusive access | Saved loan target remains `e`; no owner read from a value snapshot alone |
| Post-call, if any | Existing callee summaries/wands track escaped references | Keep entry and post-call histories; never substitute the current reference slot for an old target |
| Pre-join | Own the current element and its matching `F`; capture current element value `v` and frame model `Sf` | Both resources must be present; a different owner/index frame cannot discharge this obligation |
| Join | Consume the element and matching frame through the existing fold | Recovered owner model has value `v` at `i` and `Sf[j]` for every other logical `j` |

Introduce a selected-element transfer record that freezes the address/index/value
inputs and emits these assertions. It creates a split obligation at expansion,
preserves the explicit frame through the element lifetime, and discharges it at
the checked join. The initial path is acyclic executable array expansion with
one selected element. Packages, historical heap reads, loops and multiple live
selected regions keep their existing rules until separately qualified. Capturing
an address/index once per transition does not by itself prove it matches another
transition: the same frame predicate and PCG expansion must supply that link.

Hypothesis: explicit capture and checking makes generic one-element transfers
compositional without address injectivity or new assumptions. The first experiment
compares the baseline native controls and generated obligations; only diagnosed
missing obligations justify further repairs. Require all affected existing families
before integration. This increment does not claim nested-collection support or
start D1–D6. Full prophecy/continuation summaries remain a separate R2 obligation.

The frozen `selected-baseline-01` target matches **8/10**. The generic replacement
positive fails its whole-sequence update postcondition; its exact negative also
gets that unwanted postcondition failure. Scalar, reborrow, reference-containing
and ZST pairs already match. `selected-model-01` isolates the boundary: the
selected-element postcondition and quantified untouched-frame postcondition each
pass, the fixed-length array's sequence update still fails, and the equivalent
slice function's sequence update passes. This points to the pure array-to-slice
value bridge, not missing element ownership or const-generic length semantics.

The emitted value axiom only triggers from an array lookup. A sequence consumer
queries the slice lookup, leaving the same element equality unavailable in that
direction. Later revision `98a29875f` contains the relevant alternate slice-lookup
trigger; its separate resource-marker change is unnecessary on this branch.
First test just that trigger backport, preserving the equality and every native
assertion. This is a demonstrated prerequisite for the required generic case,
not an array contract-selection experiment. Qualify affected families before
proceeding to the explicit selected-element transfer record.

The one-change candidate (`selected-trigger-focused-01`) matches **10/10**
unchanged native controls, including the generic positive and exact negative.
The generated generic program differs only in the added alternate trigger;
the value equality, permission obligations and user assertions are preserved.
Next run the complete affected M3 gate before integrating this prerequisite.

### Second increment: persistent identities and deferred resolution

Hypothesis: a loan keyed by its MIR reservation location can retain its target
across reference-slot reassignment, so restoration no longer has to rediscover
that target by rereading the slot or a historical reference snapshot. Expose PCG's
existing reservation location without changing its analysis. First qualify this
in acyclic bodies; loop bodies retain the first increment until a summary frames
the ledger across backedges.

Each registered creation has a saved target, entry value, a pending final-value
slot, and explicit live/resolved state. The final-value slot is **not** available
as a proposition while pending. It is assigned the checked resolved value exactly
once, after the generic-to-concrete transfer. There is no assumed future equality.
Creation and resolution assert their state preconditions. An unused final value
need not be invented when the existing PCG never restores that dead resource.
Escaping/package obligations continue to use the existing checked wand encoding;
the new ledger does not discharge them speculatively inside a package.

For a direct scalar reborrow, PCG's guarded parent edge identifies the parent
creation event, separately from address equality. Record that relation at child
creation, check both targets coincide and suspend the parent's local ledger.
Child resolution checks that it is the only outstanding scalar child, transfers
the resource, and proves the child's final value equals the parent's current
value at that boundary. Subsequent parent mutation does not rewrite this saved
child value. Branch guards select the parent; joins never merge identities merely
because targets coincide. Reassigning a slot creates a distinct identity.

Required new controls: reassignment after mutation with both owners restored,
wrong historical target/value, sibling reborrows with intervening mutation,
branch-selected loans and early exits. Preserve scalar/reborrow controls and all
affected M3 families. If a parent cannot be identified through a direct PCG borrow
edge, retain the baseline permission path rather than inventing a parent from an
address. This increment is a deferred-resolution representation, not permission
to use arbitrary future facts before resolution.

For one scalar loan, distinguish the reference slot `r`, its target address `a`,
and the scalar value `v`. `C(a)` is the concrete scalar predicate; `G(a,T)` is its
existing generic representation. Neither predicate represents the reference slot.
Slot ownership remains separate. No index or address disequality is introduced.

| Stage | Available resource and observation | Obligation |
|---|---|---|
| Before acquisition | `C(a)`; capture `a` and entry value `v0` in fresh locals | Check permission to read; snapshots are values, never permissions |
| Reference assignment | Slot `r` contains the captured address | Check `target(r) == a` independently of the scalar value |
| Acquisition | Consume `C(a)` using the existing concrete-to-generic conversion | Check `G(a,T)` and its value equals the generic image of `v0` |
| Mutation/reborrow | Existing PCG capabilities and assignment contracts govern access | A parent cannot read through an outstanding child loan; no new access is inferred from value equality |
| Before resolution | PCG supplies the expiring reference and blocked owner; `G(a,T)` is available | Capture the **current** generic value `vf`, not the entry value; check target correspondence |
| Resolution | Consume `G(a,T)` using the inverse conversion | Check `C(a)` and `snapshot(a) == concretize(vf)`; only now expose the resolved value |

Each transfer witness has fresh local names and one definition per dynamic
execution. It lives only across one conversion; a loop iteration cannot reuse a
previous iteration's witness. The Rust transfer object is consumed by lowering.
PCG retains responsibility for scheduling acquisition, child-before-parent expiry
and resource availability. Viper checks permissions and equalities at the actual
program point, including both sides of each conversion. Existing generic caster
contracts have no bodies: their representation-preserving interpretation remains
part of the baseline's trusted translation, not a newly proved lemma.

For local scalar operations there is no post-call history. The first scope excludes
package scripts and labelled/historical restoration endpoints; those use the
qualified encoder. Existing call summaries, pledges, saved heap labels and frames
are retained. Outside resources are not consumed by these local conversions;
native wrong-frame controls check that unrelated values do not change. There is
no assumption that a reference snapshot's arbitrary value field is the referent's
current value: capture it from the referent predicate instead.

## Prophecy and scope boundary

The eventual interface is `Loan(id, target, entry, current, deferred)`, with
immutable values distinct from mutable Rust locals. An unresolved `deferred` is
not an ordinary unconstrained Viper local that may later be constrained by inhale.
The first increment exports only resolved observations. A subsequent increment
must supply either a checked continuation/wand rule or a justified logical
translation before permitting future-value facts prior to resolution.

The [Thrust type system](https://doi.org/10.1145/3729333) motivates separating
immutable logical values from mutable program places and relating a loan's current
and eventual values at expiry. Its soundness theorem does not automatically apply
to this hybrid permission encoding.

Before enabling that subsequent increment, implement and validate these boundaries:

- Fresh loan identities belong to creation events, not reusable Rust reference
  slots. A child resolves into its parent's current version without changing the
  parent's deferred identity.
- Reference replacement preserves both the old target's outstanding obligation
  and the new target; it never redirects an existing obligation through the slot.
- At joins, merge values under path conditions and retain each path's resources.
  Early exits must discharge the obligations they own. A loop uses an explicit
  summary, never an unbounded list of live SSA witnesses.
- Escaping references retain their obligations in a checked callee/iterator
  summary. Arbitrarily many escaped yields remain outside the initial scope.
- Selected elements require a checked resource/frame correspondence. Distinct
  indices alone prove nothing about distinct physical addresses, including ZSTs.

## Acceptance sequence

1. Build the own-branch dual-backend compiler. Run native scalar and nested
   reborrow positives, wrong-final/wrong-entry/wrong-frame controls, and inspect
   generated VIR/Viper for both acquire and resolve obligations. Compare with the
   same controls using the qualified compiler and with the flag disabled.
2. Run every repaired M3 family with the option enabled on Ferrum, one worker,
   branch parallelism off. Preserve original diagnostics and 90/300-second M3
   limits. Any timeout remains unresolved. Record coverage/source/binary hashes.
3. Qualify replacement and selected-element rules as separate increments, then
   complete prophecy scope and manual summaries. Silicon belongs at R2 feature
   sign-off, not every small change. R3 retains its original 180/600-second limits.
4. Integrate only a qualified increment, commit/push coherent source and tests;
   keep the narrower R1 baseline available. Stop before D1–D6; no automation.

After two materially different failed hypotheses for a symptom, preserve both
and review the model before another patch. Raw Viper edits are diagnostic only.
