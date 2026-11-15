# R2: checked local ownership transfers

Baseline: `56716a5ef`, whose compiler source is the qualified `2c5484c00`.
Worktree/branch: `prusti-stdlib-m3-repair`, `stdlib/ownership-core-r2`.
Artifacts: `../prusti-stdlib/target/stdlib-ownership-core-r2/`.
The six pending main-tree files and all R1/H1 artifacts remain preserved.

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

## Ownership invariant, written before implementation

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
