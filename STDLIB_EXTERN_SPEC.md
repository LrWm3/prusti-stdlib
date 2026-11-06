# Standard-library verification through `#[extern_spec]`

## Purpose and milestones

Build reusable contracts for real Rust standard-library operations, using
`#[extern_spec]` as the public specification mechanism and modifying Prusti where
the required contracts cannot yet be expressed or applied correctly.

The work proceeds in three phases:

1. Strengthen external-specification support and reach the first major milestone:
   **unbounded `Vec<T>` functional verification for owned, non-`Copy` elements**.
2. Expand Vec and slice contracts, including mutable access and basic shared and
   owned iteration.
3. Add HashMap and HashSet contracts with explicit logical key models.

The implementation tasks and proof gates below define the work. The evidence
sections record completed gates; the roadmap itself does not establish them.
The next extension is the [mutable-iterator follow-up plan](#mutable-iterator-follow-up-plan),
covering functional loop results and reference-containing elements.

“Unbounded” means verification for arbitrary symbolic finite lengths, with no
fixed collection-size bound and no loop unrolling. Rust representability limits
and operation preconditions still apply. Length-only contracts, three-element
examples, and successful compilation do not establish functional verification.

External implementations remain trusted. The verification target is client code
that calls standard-library operations, rather than the allocator, raw-pointer
implementation, or unsafe internals of the standard library.

## Current qualification checkpoint (2026-11-02)

**Current priority (user redirect; resumed 2026-11-06).** Investigate CLI/native
parity before further ownership repairs, using frozen H1 and its preserved inputs.
Work in bounded iterations with a short [parity checkpoint](STDLIB_PARITY_CHECKPOINT.md).
The last active exemplar diagnostic finished with both expected outcomes; the
unstarted reduced-model trace is retired. The bounded AST diagnostic completed:
printing/parsing regroups pure and permission conjunctions in the first divergent
package. The timeout cause remains unresolved; a minimal structural roundtrip
diagnostic is running without solver verification.
Detailed results and the next roundtrip experiment are in the parity checkpoint.

**Current stopping point (latest user instruction).** Finish restoration and
stabilize the existing generic collection and ownership proofs, including the
combined qualification below, then pause and report back. Do not start D1–D6
or schedule automation; the user wants to schedule automation work next.

**Current backend policy (updated by the user on 2026-11-01).** Use Ferrum
for routine implementation, specification changes, negative controls and regression
checks. Run Silicon at major feature sign-off points, such as completed nested
collections, interior mutability or general coroutine support, and for final
combined qualification. A small fix or intermediate candidate does not trigger a
Silicon rerun. This supersedes the earlier per-change paired-validation policy
recorded below. Keep one verifier worker and Ferrum branch parallelism off.

The single integration baseline is **`c2631cfb0`**, tagged
`stdlib-restoration-baseline-20261102`, in `/tmp/prusti-restoration-integration`.
Its complete git tree and SHA-256 manifest capture encoder, interface, VIR,
contracts, adapter and PCG. All 478 compiler-source files match frozen 793; all
six binary hashes match its build record. PCG is `791e4ca`, Ferrum is `0e048a4`.
The original six pending main-tree files are backed up byte-for-byte and remain
untouched. Experiments and future builds use isolated worktrees. This baseline
is not a qualified combined implementation.

The fixed [acceptance list](STDLIB_RESTORATION_ACCEPTANCE.json) contains **790
Rust client cases and 2,198 matrix checks in 24 jobs**, including every failure
from full matrix 339. Expected outcomes, exact negative locations, source hashes,
libraries, runtime checks and the existing 180/600-second matrix limits are
recorded. Per-case evidence and provenance are in
`target/stdlib-matrix-20261031-1236/reconciliation-20261102/`.
Historical 600-second passes do not qualify a 180-second matrix job. Counts from
different compilers are never added together.

| Area | Current evidence | Next gate |
|---|---|---|
| Whole matrix | Last complete matrix 339: 1,755/1,842; 87 failures, none waived | Rebuild final source and complete both backends with exact coverage |
| Ownership, enum, local history, associated output | 793 audited 124/124; historical 722 audited 129/129 on its own compiler | Preserve as separate reference gates; rerun affected families after each repair |
| CLI/native parity | Captured ASTs confirm conjunction regrouping before CLI preparation; it matches the first saved short-circuit scope divergence | Minimal render/parse roundtrip and structure-preserving export test; full timeout cause and semantic parity unresolved |
| Heavy prefix | H1/H2 full positives time out; exact negatives pass. Reduced both-state positive fails restoration; exemplar-only pair passes | Ownership repairs on hold for parity investigation; no experiment integrated |
| Region iterator | 794: 3/7; missing tail indexed resources in both positives and wrong early errors in both negatives | 809's checked tail-snapshot equality succeeds, but the original resource assertion still fails; model review before a repair |
| Nested calls | 794: 9/11; repeated positive missing wand, postcondition control times out | Full replay 810 times out at 600s with solver logging; previous failed hypotheses require model review |
| Extra `next` | 794: 3/6; positive timeout and two wrong diagnostics | Retain all six controls; check as an affected regression of prefix framing |
| Prefix/suffix expansion | 794 skipped 17 client proofs after the heavy failure | Skipped remains unresolved; historical 722 passes do not qualify this baseline |
| Array contract selection | 800: 3/6; positive and two exact negatives fail, out-of-bounds control passes | Separate prerequisite investigation for existing `mut_option_owner_*` RangeFull clients; no array changes in the baseline |
| Unstarted experiments | 806, 807, 812 and raw 813 retired before native execution | Source, binaries and structural evidence preserved; no automatic successor queue |

Detailed prior experiments below are historical evidence, not current status.
No accepted-negative error, wrong diagnostic, timeout, skipped group, encoding-only
success or raw Viper diagnostic counts as acceptance. The latest user workflow
requires one failure family and one falsifiable hypothesis at a time. After two
materially different hypotheses fail for one symptom, review the model and
produce a distinguishing reproducer before any successor patch.

### Selected family: heavy-prefix ownership invariant and experiment

The representative is `mut_native_resource_early_pass`, with the exact
`mut_native_resource_early_false` control. The generic-prefix positive and both
its exact controls belong to the affected family. All source assertions and
prefix/suffix value obligations remain required.

The physical owner is the original mutable slice of mutable references. Its
storage predicate describes the reference slots; indexed physical predicates
protect each referenced `u32`. An indexed key includes its logical index, path,
physical address and type. Distinct logical indices do not establish distinct
physical addresses, including for generic or zero-sized elements. This repair
introduces no address-distinctness premise or permission-query invariant.

At entry, `original` captures the slot/reference sequence and `initial_states`
captures each referent's resource state. At loop position `k`, the source is
partitioned into a consumed prefix and the live iterator tail. The prefix ghost
sequences record exactly `k` slots and updated states; the tail retains the
remaining permissions and its unchanged suffix observations. The saved
`body_restore_prefix!` history connects these pieces to the entry owner.

Each `next` call creates a restoration obligation from its captured post-call
result/tail resources to the pre-call owner resources. A `Some` result temporarily
exposes the selected physical resource; after writing 7 and recording its state,
that borrow expires and its existing history is consumed exactly once. Loop
restoration carries the composed history across the backedge. At an early break,
`position == limit`; dropping the iterator discharges the remaining tail history
and returns the original owner with unchanged slot identities, prefix values 7,
and the untouched suffix states. Current resources, entry resources and the
history connecting them cannot be substituted for one another.

**Hypothesis H1.** Variant-result frame generation added quantified cross-products
between every optional yield and a plain restored owner. They are derived
additional guarantees, not source pledges or ownership permissions. Their solver
cost caused the early-prefix regression. Restricting these added variant frames
to variant-containing restored owners should recover the earlier proof while
retaining the non-variant frames and the optional-owner guarantees that 793 needs.
The preserved 806 patch is the one proposed change; 812's separate fixed-path
pruning and every array/interface experiment remain excluded.

The falsifier is a failure or timeout of the complete Rust positive or an
incorrect diagnostic in its exact negative. Do not change the 600s whole-program,
30,000ms assertion or 100ms internal check limits. The 794 positive explicitly
hit the whole-program limit; this does not identify which internal query was
expensive. Historical 722 passes in 208.35s under matching settings. After removing
comments and generated crate identifiers only, 806's complete positive Viper
program matches 722's exactly; the 793-to-806 difference is retained for review.
This motivated H1 but did not qualify it. The isolated rebuilt H1 positive now
times out at 600s (615.08s command elapsed); the 720s outer watchdog did not fire.
Its native dump still matches 722 under the stated normalization. H1 is not
accepted. Its exact negative passes in 309.70s versus 722's 304.84s. The focused
gate is 3/4 with complete coverage and matching source/binary provenance.

Both unchanged complete programs pass a controlled replay through the pinned
Ferrum CLI with identical limits: 722 in 260.94s and H1 in 272.04s. These are
diagnostic results, not Rust acceptance. A controlled native repeat with the same
frozen H1 compiler, source and settings also hits the 600s whole-program limit
(614.86s command elapsed). Its dump has the same obligations under the stated
normalization, but different raw bytes. Both native runs have complete coverage
and matching source/binary provenance; neither is accepted.

The next diagnostic replays the saved full Rust compiler invocation and dependency
metadata, keeping crate identities fixed and adding only solver logging and
unknown-result reporting. It retains the 600/30,000/100 limits. Locate the first
expensive query and its restoration/value/frame obligation; compare proof
structure through the CLI if needed. Logging results remain diagnostic. Preserve
the discrepancy before any successor encoder patch or integration. The detailed
plan, exact inputs and results live under `reconciliation-20261102/`.

The logged native replay fails in 516.32s at `body_restore_prefix!`, after the
successful `Some` branch. Its first assertion-budget failure checks indexed
physical permission coverage when applying the saved loop history (Viper line
10081; SMT query 70710). Subsequent prefix-value and suffix-frame queries also
exhaust their assertion budgets. The logged CLI fails earlier while matching
the `next` restoration wand. The **exact same current program** passes the CLI
without diagnostic logging in 297.49s. Logging changes outcomes and these
diagnostic failures must not be substituted for the two standard native timeouts.

**Hypothesis H2 (one bounded experiment).** Native and parsed programs group
pure Boolean conjunctions differently. The traces first diverge in evaluation
of the pure address/length framing condition, after 19,413 matching SMT commands
when source-position query IDs and associative pure `and` structure are normalized.
Normalize only pure conjunctions to right association in the native adapter,
preserving operand order and every leaf's metadata. Do not reassociate permission
conjunctions, cross quantifier/let/wand scopes, drop facts, or alter any ownership
stage, snapshot, history or discharge obligation described above. The proposed
repair changes pure-query scheduling; it adds no premise about physical addresses.

H2 is an isolated experiment on H1's frozen source, not integration of H1. First
build and structurally check it, then replay the preserved full Rust invocation
with unchanged dependency metadata and crate identities to isolate this change.
The positive must pass within 600s, followed by the native positive/exact-negative
gate and all affected families before integration. A continued failure falsifies
H2 as a sufficient repair. With H1 and H2 both unsuccessful, stop successor patches
and return to the model and reproducer; do not create an H3 patch without new
distinguishing evidence.

H2's own build and three ownership unit checks pass. Its adapter controls preserve
the permission conjunction's node, short-circuit undefined division correctly and
reject the exact false assertion. The full generated program passes AST consistency
checking. Crate identifiers and every non-parenthesis token, in order, match H1;
the recorded parentheses-only diff reflects the reviewed pure-conjunction change.
The preserved-input full Rust positive **times out at 600s** (608.78s elapsed;
outer watchdog not fired; source unchanged). H2 is therefore also falsified as a
sufficient repair. Its exact native negative passes in 340.35s with complete
coverage, matching source/binary provenance and no accepted negative.
These structural and unit results do not qualify H2, and the immutable integration
baseline stays put. No successor patch is planned.

The required model review now distinguishes basic ownership composition from
the cost of additional observations. A separate diagnostic worktree uses H1's
frozen compiler and retains the mutable owner, per-yield writes, prefix slot/state
histories, length/position bounds, early break and iterator expiry. It removes
exemplar, original-slot and untouched-suffix observations only in the reduced
fixture, with a nonempty exact false-assertion companion. If this still fails,
localize its first obligation as a smaller reproducer. If both controls behave
as intended, add back one observation family at a time to locate the cost trigger.
A passing reduction does not establish the full case or isolate solver naming
effects by itself. The original acceptance source and all its obligations remain
unchanged. Evidence and the experiment's interpretation are recorded before
execution in `prefix-model-reduction-plan.json`.

The first ownership-only reduction is audited 3/4: its positive fails final
entry-indexed permission restoration in 155.97s, while its exact negative passes
in 107.63s. This is a different location from the full-case timeout, and removing
slot-identity facts can underconstrain the physical resource keys. It is not yet
a reproducer of the original failure. The next recorded reduction therefore
retains original slot identity, prefix/tail identity relationships and a checked
final slot-identity assertion, omitting only referent-value observations. Its
source, interpretation and fixed H1 provenance are recorded separately; the
required acceptance files remain unchanged.

The identity-preserving reduction passes its complete native positive in 118.76s
and exact negative in 216.62s, with an audited 4/4 gate. This establishes basic
ownership composition in that reduced context, not the original value proof.
The next comparison fixes the original H1 compiler arguments and dependency
metadata. Recheck the identity positive and negative first; only if both pass,
add the initial referent-state snapshot and untouched-suffix loop invariant.
Prefix-value observations and final value postconditions remain omitted only in
these diagnostic fixtures. This distinguishes the cost of suffix-state
observations while retaining physical ownership, slot identities and histories.
The plan and sources are preserved in `prefix-state-model-plan.json`; no compiler
patch or change to the fixed acceptance list is involved.

That fixed-context comparison passes all four native expected outcomes: identity
positive/negative in 106.74/178.86s, suffix-state positive/negative in
238.41/306.17s. All generated programs pass structural checks; exact diagnostics,
coverage and unchanged source/binaries are audited in `prefix-state-fixed-audit.json`.
These are diagnostic reductions, not four newly qualified acceptance cases.
The next recorded pair adds the concrete `u32` exemplar and updated-prefix state
invariant to the suffix model, while still omitting final referent-value assertions
and postconditions. It retains every existing permission, identity, history and
suffix-state fact. This distinguishes loop-summary cost from the remaining final
value proof; see `prefix-both-state-model-plan.json`.

The historical 722 capture and the frozen baseline have identical adapter source,
adapter manifest, workspace manifest and lockfile. The recorded 722 source patch
hash also matches. Its historical pass therefore cannot be explained by a source
change in those components; generated names and solver behavior remain relevant.

Any eventual repair must rebuild from its isolated source, check the complete
generated programs, and pass the native positive and exact negative. Then run all
affected previously passing families before integrating: the full heavy-prefix
group, extra-next controls, enum, optional/local histories, first-element and
replacement controls, owner histories, generic/prefix libraries and suffix
restoration. Inventory generated-program changes when deciding whether any other
family uses the changed path. A new failure or material slowdown stops integration.
Identical programs with different outcomes require backend investigation; repeat
an identical run only to distinguish variability. Larger diagnostic allowances,
if needed, are separate and never qualify a control.

### Historical repair evidence (2026-11-01, probes 408–449)

The nested observer repair now proves the original nested-reference Rust
positive on Ferrum. Probe 408 passed 9/9 Ferrum checks and 8/9 Silicon checks;
Silicon could not apply the temporary output reservation. Saving its resource
keys explicitly in probe 414 preserved the original positive and all exact
negative controls, but the new repeated-call positive failed: 12/13 Ferrum
checks passed. Silicon has not qualified that revision. Probe 424 now shares
the set of consumed, guarded call obligations across return packages; each
later package must still prove its consequent from its antecedent and frames.
Its seven encode-only clients pass. The minimum remains 12/13 on Ferrum:
the original positive and all exact negative controls pass, but the repeated
caller still fails. AST inspection confirms that the second package no longer
reuses the earlier callee obligation. Exact textual replay 438 timed out at
600 seconds, so it does not establish a new failure location. A method-filtered
diagnostic is queued; this candidate is not adopted.

The owned-loop candidate 382 passed 21/26 checks on each backend, with complete
coverage and provenance audits. Exact-source replay 418 locates duplicate
`into_iter` wand consumption in the second loop-entry package. A strengthened
PCG regression fails before `9dfaf88` and passes afterward under both NLL and
Polonius; the PCG corpus, shape, projection and opaque-body checks also pass.
That commit removes post-call outputs for moved owned arguments, whose storage
has been consumed, and is pushed on `stdlib/owned-iterator-resources`.
Prusti's own rebuilt candidate 422 passes 6/10 Ferrum checks: the duplicate
package is gone, but loop reconstruction now reads labels from the wrong copy
of the loop prefix. Main's PCG pin remains `269cb9a`; this is not an adoption.

Candidate 433 preserves actual call obligations, shares capture locals between
the duplicated prefix and repeated body, and captures historical nonreference
values at executable MIR points. Its Ferrum minimum improves to 8/10: both
ordinary loops and both exact wrong-invariant controls pass. The extra-`next`
positive and false control still fail resource restoration. Prefix early-return
fixtures encode, but their proofs are not yet qualified.

Checked-source diagnostics 437 and 440 isolate three lost facts: the current
second iterator equals its post-call snapshot, a discarded yield still owns
resources needed by the retained obligation, and independently captured copies
of one observation must remain equal. The added invariants pass entry and
backedge checks; the remaining diagnostic is the bound needed to observe the
first element of the pre-call tail. A separate checked invariant tests that
ordinary `next` postcondition in diagnostic 444. These edited Viper programs
are diagnostic evidence, not Rust feature qualification.

The rebuilt 449 candidate implements the resulting plan: deduplicate identical
call observations, retain ADT projections of a single capture, preserve stable
logical consequences of functional postconditions, carry dropped-result
resources that remain leaves of a live call obligation, and infer checked
snapshot equalities along post-call resource flows to available current owners.
All inferred invariants must be proved at entry and every backedge. Whole
postcondition retention in prototype 446 was retired after encoding showed
heap-dependent facts and old labels; none of its proofs is claimed.

Run the focused ordinary/extra-call positive and exact-negative gate first,
then early returns, empty traversal, nested loops and the broader owned suite.
The independent cross-crate rerun 360 passes all ten generic-resource checks
on Ferrum; Silicon passes all nine client/runtime checks but fails the
specification library's prefix restoration. Its prefix-resource pair remains
in progress. These backend differences must be resolved before qualification.
Logs, exact-source replays, frozen source/binary hashes and audits are under
`target/stdlib-matrix-20261031-1236`. A failed Ferrum minimum gate stops routine
qualification of that candidate; passing repairs still require Silicon before
adoption and the final combined matrix.

## Baseline and evidence

Baseline: checkout `87487f774`, inspected on 2026-10-26. Upstream observations are
dated to that inspection and must be rechecked when implementation begins.
The inventory below records the initial inspection; subsequent implementation
evidence is recorded separately below.

Use these evidence labels throughout implementation:

- **Inspected:** supported by source inspection, without a successful proof run.
- **Verified:** the named positive and negative verification cases ran and met
  their expected results on the recorded toolchain and revision.
- **Failing:** a reproducer ran and demonstrated the stated failure.
- **Untested:** the required experiment has not run.

| Area | Evidence | Finding |
|---|---|---|
| External-specification syntax and rewriting | Inspected | Existing support covers free functions, inherent methods, traits, and trait implementations in the [rewriter](prusti-contracts/prusti-specs/src/extern_spec_rewriter/). Preserve this syntax. |
| Target resolution | Inspected | The [resolver](prusti-interface/src/specs/external.rs) maps specifications to targets and checks several duplicate and generic-parameter errors. Its existence does not establish complete generic compatibility. |
| Cross-crate contracts | Inspected | Export/import and duplicate detection exist in [typed specifications](prusti-interface/src/specs/typed.rs). End-to-end reuse remains a required probe. |
| Vec and slice contracts | Inspected | [Vec contracts](prusti-contracts/prusti-contracts/src/alloc_spec/vec.rs) provide `len` and `as_slice`; [slice contracts](prusti-contracts/prusti-contracts/src/core_spec/slice.rs) provide `len` and a `SliceIndex::index` declaration. These do not establish full sequence behavior. |
| Logical collection types | Inspected | [`prusti-contracts`](prusti-contracts/prusti-contracts/src/lib.rs) already exposes `Seq`, `Map`, `Set`, `Ghost`, and `Int`. Reuse them before introducing new logical primitives. |
| Explicit quantifier triggers | Inspected | The [pure encoder](prusti-encoder/src/encoders/mir_pure.rs) contains `TODO: triggers` and emits quantifiers without the user-written trigger sets. |
| Default test coverage | Inspected | The [compiletest harness](prusti-tests/tests/harnesses/compiletest.rs) runs `verify/pass` with overflow checks disabled; failure/UI and overflow suites are commented out. The [main runner](prusti-tests/tests/main.rs) also disables the Cargo harness. |
| Local build prerequisites | Inspected | [`rust-toolchain`](rust-toolchain) pins `nightly-2026-08-15`, which is not installed locally. The `pcg` submodule directory is empty. |
| Functional collection and iterator milestones | Untested | No Vec, slice, iterator, or hash-collection milestone has been verified. |

### Relevant upstream work

| Reference | Observed limitation or contribution | How to use it |
|---|---|---|
| [PR #1582: basic Vec specs](https://github.com/prusti/prusti/pull/1582) | Draft contracts cover certain operations only up to length three and depend on snapshot work. | Use as a starting point; the length bound must be removed before the first milestone. |
| [PR #1567: quantifier triggers](https://github.com/prusti/prusti/pull/1567) | Forwards explicit triggers to Viper and validates trigger coverage. | Evaluate for Phase 1; test the emitted encoding and symbolic-index proofs. |
| [PR #1573: deep snapshots and two-state trait specs](https://github.com/prusti/prusti/pull/1573) | Adds pre-state snapshots and trait pledges; explicitly leaves generic deep snapshots and nested mutable-reference permission cases unsupported. | Evaluate the required parts, then reproduce and address remaining cases. Do not assume adopting it completes generic Vec or mutable access. |
| [PR #980: iterator tracking](https://github.com/prusti/prusti/pull/980) | Historical work addresses associated types, reference-valued items, refinement lifetimes, and unintended model `Copy` constraints. | Extract relevant tests and ideas for the current encoder rather than treating the old branch as current support. |

### First implementation milestone: unwrap and replace

**Verified locally on 2026-10-26:** a separate specification crate exports working
contracts for `Option::unwrap` and `std::mem::replace` to generic, untrusted
clients. A non-`Copy` token with no `Clone` or equality traits exercises the
payload, returned old value, and updated destination. See the
[fixture and test command](prusti-tests/tests/cargo_verify/stdlib_extern_spec/README.md)
and the [focused runner](scripts/test_stdlib_extern_spec.py).

The first run exposed a **failing** permission-encoding case in replacement.
The contract returned the destination's permission inside `true --* P`, then
attempted to read that destination in its functional postcondition. The
[wand encoder](prusti-encoder/src/encoders/impure/fn_wand.rs) now returns those
permissions directly when the left side has no resources and there are no
pledges. Real borrow and pledge obligations remain. This is a general encoding
fix, not a special case for `mem::replace`.

The focused suite checks ordinary compilation, runtime behavior, positive
verification, and intended rejection of wrong values, missing preconditions,
overflow, a false assertion, and an invalid shared-reference pledge. The positive
client also verifies restoration after a mutable borrow. All clients keep
overflow and panic checks enabled; only the external declarations are trusted.
The suite is wired into CI, with local logs and timings written to
`target/stdlib-extern-spec-logs/`.

The build uses `87487f774` plus the local encoder fix, PCG
`e90a8144af97582508ff29e238dabfaa5f01800b`, Rust
`1.99.0-nightly (d453bdd8f 2026-08-14)`, Viper release
`v-2026-09-01-0834`, and Z3 `4.16.0`. The pinned submodule, Rust components, and
Viper tools are now installed, and `./x.py build --all` succeeds locally.

Five existing positive borrow/pledge cases and the negative shared-reference
pledge case also passed. The older negative
[`assert_on_expiry` test](prusti-tests/tests/verify/fail/magic-wands/assert_on_expiry.rs)
still reports an internal backtranslation error for
`apply.failed:assertion.false`; the same result was reproduced with the original
encoder. It is an existing diagnostic limitation, not a completed negative-test
gate.

This completes the small unwrap/replace milestone and the local P1-A runner.
The P1-B audit has since been extended as recorded below.
The first major collection milestone M1 has not been attempted. At this stage the
prototype contracts lived in the fixture crate; their bundled integration is
recorded below.

### External resolution and generic-trait probes

**Verified locally on 2026-10-26:** this audit expanded the focused suite to 27 checks,
including ordinary Rust compilation, execution of both positive clients, and
positive/negative verification with overflow checking enabled. The unannotated
`target_api` fixture supplies generic traits and implementations; its contracts
are exported by `stdlib_specs` to a third crate. The probes cover trait type
arguments, owned non-`Copy` associated items, shared-reference items, explicit
borrow lifetimes, concrete implementation contracts, and local refinement of
an imported trait contract.

Negative cases reject duplicate local and imported declarations, mismatched
signatures, stronger implementation preconditions, weaker implementation
postconditions, and use of a concrete refinement in an unconstrained generic
caller. Diagnostics must match both the expected message and source line;
signature failures also require the explanatory note.

The signature probe initially **failed**: a declaration taking `&mut u32` was
accepted for a target taking `&u32`, because Rust coerced the generated call.
The [external-spec resolver](prusti-interface/src/specs/external.rs) now compares
normalized input/output types and checks that type and const parameters are
forwarded unchanged, preventing generic specialization from silently producing
an unconditional contract. The existing generic trait-implementation,
module-path, and shared-reference equality regression cases also pass.

The bound audit reproduced another **failing** case: an external declaration
could add `T: Copy` to `mem::replace`, whose target has no such requirement.
The resolver now checks each declaration bound against the instantiated target
environment. The suite rejects added `Copy`, associated-type, supertrait, and
`T: 'static` requirements. It accepts a redundant `Clone` bound implied by
`Copy`, an existing `'static` requirement, and a trait with a bounded type
parameter. Positive clients also exercise generic `?Sized` callers and by-value
receivers. All 33 `prusti-specs` unit tests pass.

This check exposed an implicit `Self: Sized` requirement in trait-spec rewriting.
The rewriter now permits unsized receivers, keeps type parameter bounds out of
type arguments, and preserves declared supertraits. A by-value `self` still
requires `Sized` at a Rust call. The bundled `PartialEq` declaration now matches
the pinned Rust API's `PointeeSized` bounds on both `Self` and `Rhs`.

At the end of this audit, lifetime-signature matching and bundled integration
were still open. The next milestone addresses both, as recorded below.

### P1-B complete: bundled contracts and lifetime-preserving resolution

**Verified locally on 2026-10-26:** all 43 focused checks pass. The contracts for
[`Option::unwrap`](prusti-contracts/prusti-contracts/src/core_spec/option.rs) and
[`core::mem::replace`](prusti-contracts/prusti-contracts/src/core_spec/mem.rs) are
now bundled, with a single authoritative declaration for each target.
`std::mem::replace` resolves to the same target. The contracts were checked
against the pinned Rust source: unwrap moves the `Some` payload, and replace
returns the previous destination after moving in the source without dropping
either value. Client proofs cover owned non-`Copy` values without `Clone` or
equality traits, both replacement effects, and missing-precondition/wrong-value
rejections.

The runner checks direct clients, explicitly verifies a separate library's
generic wrapper bodies, and verifies clients consuming those exported wrapper
contracts. A `#![no_std]` library with only the `core` feature also compiles and
verifies. Duplicate bundled and fixture-library contracts are rejected. Invalid
declarations are not registered, avoiding secondary duplicate diagnostics.
Legacy examples now use the bundled contracts instead of redeclaring them.

Three lifetime probes initially **failed**: the old resolver accepted merged
input lifetimes, a `'static` input restriction, and a shortened `'static` return.
The resolver now normalizes signatures without erasing regions and anonymizes
each binder independently. It checks early-bound generic positions, preserves
early lifetimes when checking bounds, and compares concrete trait specs against
the public trait call signature. Positive probes cover renamed lifetimes,
existing outlives bounds, type aliases, and nested higher-ranked callbacks;
negative probes cover changed relationships and added outlives requirements.

Making the bundled shared-reference equality impl lifetimes explicit exposed
another **failing** case: spec encoding removed lifetime argument slots and
shifted subsequent type parameters, causing a compiler panic. The
[spec encoder](prusti-encoder/src/encoders/pure/spec.rs) now retains those slots.
The shared-reference equality regression and an exported generic identity
contract with an early-bound lifetime verify with this fix.

Validation: `./x.py build --all`, all 43 focused checks, all 33 `prusti-specs`
unit tests, the three existing external-spec regression cases, and the
overflow-enabled Option regression pass. The environment is `38d7d383e` plus
this milestone's working-tree changes, PCG
`e90a8144af97582508ff29e238dabfaa5f01800b`, Rust
`1.99.0-nightly (d453bdd8f 2026-08-14)`, Viper `v-2026-09-01-0834`, and Z3
`4.16.0`. Logs remain in `target/stdlib-extern-spec-logs/` and
`target/stdlib-regressions/`.

This completes the P1-B foundation gate within the tested domain. Matching is
conservative: early-bound parameters must occupy the same generic positions,
and complex equivalent outlives constraints may be rejected. These checks do not
establish unrestricted lifetime equivalence, reference-containing collection
snapshots, or iterator models. P1-C completion is recorded below. M1 remains **untested**.

### P1-C complete: sequence observations and explicit triggers

**Verified locally on 2026-10-27:** the [sequence fixture](prusti-tests/tests/cargo_verify/stdlib_sequence_spec/README.md)
passes 21 focused checks, and the earlier external-specification suite still
passes all 43. The [runner](scripts/test_stdlib_extern_spec.py) selects the new
suite with `--suite sequence`; CI runs both suites with overflow and panic
checks enabled, verification caches disabled, and fresh Cargo targets.

`Seq::from_slice(&[T])` now exposes the existing slice element snapshots as a
logical sequence. The encoder relates its length to the slice reference's
metadata and every valid index to the slice snapshot's element accessor. The
length axiom is conditional on a nonnegative length; observation does not add an
inconsistent negative-length sequence. The indexing axiom has triggers for both
sequence indexing and the combination of a slice index with an observation.
This supports direct element proofs from equality of two observations.

The ordinary pure helper `alloc_spec::vec::vec_contents<T, A>` is defined through
`Vec::as_slice`, outside the external method declarations, with the allocator
parameter preserved. No element `Copy`, `Clone`, or equality bounds are added.
Proofs cover generic lengths and indices, quantified element agreement,
non-Copy structs that remain available after observation, and both effects of
moving generic Vec values through `mem::replace`. A separately compiled
`no_std` library, using the `core` and `alloc` contract features, exports verified
observation and replacement wrappers to the clients. False length, element,
replacement, and contradiction assertions are rejected at their expected lines.

Explicit trigger sets are forwarded for universal and existential quantifiers,
adapting [PR #1567](https://github.com/prusti/prusti/pull/1567). The parser validates
variable uses rather than raw identifier tokens: every term must mention a
bound variable, and each set must cover all binders. It rejects empty sets,
unnamed binders, local bindings, macros, and unsupported top-level expression
forms. Field and method names cannot masquerade as binder uses. Positive cases
exercise captures and multi-triggers. The runner inspects emitted Viper for an
`explicit_marker` function appearing inside both kinds of trigger; that function
never occurs in a quantified body, ruling out automatic trigger inference as
the explanation. Generated programs remain under
`target/stdlib-sequence-spec-logs/verifier/`.

**Inspected:** [PR #1573](https://github.com/prusti/prusti/pull/1573) separates
shallow and deep snapshots and adds two-state trait specifications. It has not
been imported. Existing owned-value snapshots and pre-state heap reads suffice
for this gate's replacement proofs. Separate probes on this branch verify
precise unsupported-feature diagnostics for mutable references in generic fields
and pure observation of nested mutable references. These are not proofs of
support for reference-containing elements, interior mutability, or nested
collection models. An exploratory nested-reference function body also produces
an insufficient-permission backtranslation error; its
[reproducer](prusti-tests/tests/cargo_verify/stdlib_sequence_spec/known_limits/nested_mutref_body.rs)
is retained outside the passing suite and remains **failing**.

Validation: the full build, 21 sequence checks, 43 earlier milestone checks,
34 `prusti-specs` unit tests, and seven existing sequence/array/trigger regressions
pass. The trigger UI diagnostics and typechecking snapshot were refreshed.
The toolchain, PCG, Viper, and Z3 versions remain those recorded for P1-B;
the checkout is `38d7d383e` plus the P1-B and P1-C working-tree changes.
Regression logs are in `target/stdlib-p1c-regressions/`.

### P1-D completion evidence: minimal functional Vec / M1

**Verified locally on 2026-10-27:** the
[Vec gate](prusti-tests/tests/cargo_verify/stdlib_vec_spec/README.md) passes all
21 checks. Bundled contracts cover `new`, `len`, `is_empty`, `push`, `pop`,
`as_slice`, and shared scalar indexing. `Index<I>` retains the actual index and
allocator parameters; its contract uses `SliceIndex::get`, with a supporting
`usize` implementation contract. Range indexing remains in P2-A.

The explicitly verified `no_std` specification library exports its sequence
observation and generic append/round-trip wrappers to separately compiled
clients. Generic proofs establish arbitrary-index append preservation, the
appended argument snapshot, new length, pop's returned last element and retained
prefix, and push/pop restoration. A loop constructs a Vec of symbolic length
with an invariant describing every element of the prefix. Empty, five-element,
and eight-element executions instantiate owned structs without `Copy`, `Clone`,
`PartialEq`, or `Eq`. No collection-size bound, unrolling, or known-failure
exemption is used.

The retained negative cases reject wrong elements, prefixes, lengths, pop
results, out-of-bounds indexing, `usize` length overflow, and duplicate bundled
contracts. Two encoder blockers exposed by these proofs have dedicated probes:

- Concrete trait calls now select the implementation and its substituted
  arguments consistently for purity and call encoding. A generic trait-bound
  caller cannot inherit a particular implementation's purity. Unsupported pure
  calls produce a diagnostic instead of a compiler panic.
- `old(n)` in a body assertion or invariant now reads the entry state. Previously
  it was treated as a current captured snapshot, making `n == old(n)` a tautology.
  Body-spec temporaries are inlined before applying the state label, and nested
  closure captures are resolved in their enclosing context rather than cast
  unsafely between expression representations. The scalar positive/negative
  probes and the symbolic Vec loop exercise this fix.

The runner enables overflow and panic checking, disables verification caching,
and uses fresh Cargo output directories. Ordinary compilation and positive
runtime execution pass as well. The full build, the earlier 43 external-contract
and 21 sequence checks, and the seven existing sequence/array regressions pass.
Legacy Vec and trait-dispatch regression checks also pass; the existing default
trait-body test retains its usual overflow-disabled configuration. Logs are in
`target/stdlib-vec-spec-logs/` and `target/stdlib-p1d-regressions/`. The pinned
toolchain, PCG, Viper, and Z3 versions remain those recorded for P1-B; the checkout
is `38d7d383e` plus the P1-B through P1-D working-tree changes. CI includes the new
suite, but a hosted CI run is not claimed.

**M1 is complete for owned values with stable value snapshots.** These are
normal-return contracts for trusted library implementations. `push` requires
length below `usize::MAX`; it does not guarantee allocation success or exclude
capacity-overflow failure. Allocation, unwinding, and arbitrary destructors
remain execution assumptions. Reference-containing elements, interior mutability,
nested collection models, mutable views, and iterators remain outside this gate.
The P2-A expansion is recorded below.

### P2-A complete: expanded Vec and shared slice contracts

**Verified locally on 2026-10-27:** the
[collection gate](prusti-tests/tests/cargo_verify/stdlib_collections_spec/README.md)
passes all 27 checks. Bundled contracts now cover `Vec::clear`, `truncate`,
`append`, `insert`, `remove`, and `swap_remove`. They describe the complete
resulting sequences, including both entry states and both final states for
`append`, shifted suffixes for insertion/removal, and replacement by the old
last element for `swap_remove`. The allocator parameter and generic element
type retain the actual API's bounds.

Shared slice contracts cover checked lookup, indexing, `split_at`, and
`is_empty`; `Vec::deref` connects implicit shared coercions to the same sequence
observation. Supported indices include `usize` and the standard `core::ops`
range forms `start..end`, `start..`, `..end`, `..`, `..=end`, and freshly
constructed `start..=end`. Checked lookup describes both success and failure,
and indexing requires success. The `Index` contracts preserve the entire
reference snapshot, including slice length metadata. `slice::swap` exchanges
two element snapshots while preserving every other element, including the
case where both indices coincide.

The inclusive-range contract accounts for Rust's hidden iterator state. The
snapshot of `RangeInclusive::new(start, end)` distinguishes a fresh range from
an exhausted one; an exhausted range selects an empty slice. Fresh literal
ranges satisfy the freshness condition, but equal endpoints alone do not
establish it. Tests verify singleton ranges, valid empty `4..=3`, invalid
`5..=3`, and rejection of an inclusive `usize::MAX` end. A negative probe rejects
inferring a singleton from equal endpoints of an arbitrary inclusive range.
General iterator state remains outside this gate.

The explicitly verified `no_std` library exports generic append,
insertion/removal round-trip, and split wrappers. Its symbolic-length builder
constructs owned values without `Copy`, `Clone`, or equality traits. Clients
prove arbitrary-index append observations, quantified insertion/removal
prefixes and suffixes, swap-removal framing, truncation, range lookup branches,
complementary splits, and restoration after swapping twice. Concrete tests
cover empty/end cases and lengths five and eight. Some concrete observations
are asserted before mutation to instantiate the builder's quantified contract;
these are proved assertions, not assumptions. Boundary checks are separated
into small functions to avoid accumulating unrelated range facts in one proof.
No collection-size bound, loop unrolling, or known-failure exemption is used.

The 19 negative probes reject incorrect values, lengths, ordering, source state,
framing, bounds, and length overflow at their expected source lines. The runner
performs ordinary compilation and positive runtime execution, enables overflow
and panic checking, disables verification caching, and uses fresh Cargo output
directories. CI includes `--suite collections`; a hosted run is not claimed.

Validation: `./x.py build --all`, all 27 new checks, and all 85 earlier focused
checks pass, for **112 passing focused checks**. The two affected legacy Vec
tests and the duplicate-specification regression also pass; the latter now
uses `reserve` because `clear` has an authoritative bundled contract. Logs are
in `target/stdlib-collections-spec-logs/` and `target/stdlib-p2a-regressions/`.
The pinned toolchain, PCG, Viper, and Z3 versions remain those recorded for P1-B;
the checkout is `38d7d383e` plus the P1-B through P2-A working-tree changes.
This gate required additional external contracts and clients, with no further
encoder changes beyond the earlier milestones.

**P2-A is complete within the owned, stable-snapshot domain.** These remain
normal-return contracts with allocation, capacity failure, unwinding, and
arbitrary destructors outside the proof. Mutable slice swaps are exercised
through direct slice arguments and arrays; restoration through mutable Vec
views is not established. The next gate is **P2-B**, mutable views and
borrow-expiry contracts. At that point, P2-C iteration and the overall M2 gate
were still open.

### P2-B progress: mutable Vec slice restoration

**Verified locally on 2026-10-27:** the initial
[borrow fixture](prusti-tests/tests/cargo_verify/stdlib_borrow_spec/README.md)
passes four checks. `Vec::as_mut_slice` exposes the entry sequence and carries
an expiry pledge restoring the Vec's model from the final slice observation.
A separately compiled `no_std` library proves a generic swap through that view;
its client proves length preservation, exchanged values, and restoration after
two swaps, with a non-`Copy` owned runtime instance. The runner uses the same
pinned environment and proof settings as P2-A, via `--suite borrow`.

This establishes one mutable-view dependency. **P2-B remains incomplete**:
mutable indexing, optional mutable lookup, disjoint splitting, reborrowing, and
negative framing/value checks are still required.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 24 checks. `slice::split_at_mut` describes both entry subsequences and
restores their concatenated final contents at expiry. Generic clients change
both halves and prove restoration through slices and Vecs, including separately
expiring halves. Its invalid-bound, wrong-value, and wrong-frame probes reject
the expected claims. `slice::get_mut` now describes both result branches: an
absent lookup preserves the sequence; a successful lookup exposes and restores
the selected value and preserves length. Matching and reborrowing are exercised.

Three infrastructure failures were isolated and fixed. PCG matched call-site
regions against a deduplicated signature list, losing a second result region
when both had the same signature lifetime. Its structural region mapping now
handles the two separately; the fix is pinned at `e54bf675468d` on the
`LrWm3/pcg` fork. Pledge expressions now cast concrete call arguments to the
callee's generic snapshot representation, and `before_expiry` preserves the
expiry-state heap read through a temporary obtained by matching an enum. The
cross-crate generic and `Option<&mut T>` probes exercise these changes, including
rejection of incorrect owner states. The full build, 62 PCG unit tests, the 24
borrow checks, and the earlier 43 external-contract and 21 Vec checks pass.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 32 checks. Mutable slice and Vec indexing now describe the entry and
final selected referent and preserve length. The encoder resolves the same
trait implementation when creating and applying expiry wands; abstract trait
calls also carry the indirect permissions and wands required by borrowed
results. Cross-crate tests exercise concrete and abstract trait calls and reject
wrong selected values and out-of-bounds indexing.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 36 checks. Scalar `get_mut` and slice and Vec `IndexMut` describe the
whole final sequence as the old sequence updated at the selected position.
Symbolic clients prove both selected values and preservation at any distinct
index, including generic owned elements and a separately verified cross-crate
wrapper. This uses a trusted ghost-only `scalar_index_model`: its intended
interpretation is `Some(i)` for a `usize` index `i` and `None` for other sealed
`SliceIndex` types. The `usize` `SliceIndex::get` contract exposes its
classification. Clients cannot infer that a range has a scalar classification,
as tested by a negative probe. A sequence-update pledge avoids the timeout seen
with an earlier quantified framing condition.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 40 checks. Mutable lookup and slice and Vec indexing of bounded
`start..end` ranges expose the returned slice's length and preserve both outside
regions. Symbolic clients verify mutations and the resulting owner state. The
entry contract equates pointer metadata as well as referent snapshots, because
slice length is stored in pointer metadata. A trusted ghost-only
`range_index_model` maps `Range<usize>` to its endpoints and other index types
to `None`; a negative probe prevents treating a scalar index as a range. The
range fixture took 85 seconds overall in the full run, so its solver
performance remains a concern; the verifier uses a 90-second limit for the
whole verification, not for each method. The checked, slice-indexing, and Vec
indexing examples are now separate fixtures, preserving the same proofs while
giving each an independent whole-verification budget.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 56 checks. The range classifier also handles `start..`, `..end`, and `..`.
Open ends use `usize::MAX`, with independent guards for the prefix and
suffix; this is sound for invalid ranges because checked lookup restores the
entire owner on `None`, and indexing requires a valid range. Symbolic clients
verify exact updates, outside framing, empty views, and generic owned Vec
elements. Splitting the bounded fixtures also avoids requiring all their proofs
to complete within one whole-verification timeout.

**Further progress, verified locally on 2026-10-27:** the expanded borrow suite
passes 65 checks. Inclusive ranges with representable exclusive endpoints
are classified using their actual state. Fresh ranges use their start and
incremented end; exhausted ranges use an empty range at the incremented end.
The shared lookup contract also describes the exhausted case. Symbolic slice
and generic owned Vec clients prove updates and outside framing, and checked
lookup covers valid empty ranges and `usize::MAX` failures. A negative mutable
lookup probe rejects inferring a singleton from equal endpoints without a
freshness guarantee.

**P2-B complete, verified locally on 2026-10-27:** the expanded borrow suite
passes 74 checks. `Vec::DerefMut` now carries the same entry and expiry relation
as `as_mut_slice`, matching the pinned standard-library implementation. Direct
`Vec::get_mut` clients prove scalar, bounded, and inclusive updates, the missing
branch, reborrowing, and restoration of whole sequences; a separately verified
generic library and client exercise owned non-`Copy` elements. An additional
exhausted-inclusive indexing proof and negative framing checks pass. These
results satisfy the P2-B mutable-view gate; at that stage P2-C and M2 remained
open.

### P2-C: owned and shared iterator models

**Verified locally on 2026-10-27:** the focused iterator suite passes 20
checks, including its separately verified `no_std` library and negative cases.
Trusted free impure functions, directly called external trait methods, and
external inherent methods can provide type-conditional postconditions when
rustc proves their bounds at the call site. The bundled sealed `ModeledIterator`
capability observes the
remaining sequence of a `Vec::IntoIter`. Trusted contracts connect
`Vec::into_iter` to the original vector and specify the head and tail of
`Iterator::next`, including an empty iterator that stays exhausted. A generic
cross-crate client uses the capability, while negative cases reject claims
without the required evidence. The [owned iterator fixture](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/README.md)
also exercises non-`Copy` elements and a symbolic counting loop.

The separately verified generic `advance` wrapper exports the head-and-tail
contract for an existing owned iterator of borrowed items. This exercises
`Item = &T` across crates without establishing general reference-containing
collection snapshots. Direct `next` calls on this iterator type are also
verified, including head, tail, and exhaustion properties.

The sealed capability also models `slice::Iter`. Its trusted constructor
connects each remaining reference to the corresponding source element, and
the verified generic wrapper advances it. A separately verified `no_std` helper
returns the first source reference, with non-`Copy`, empty, exhausted, and
negative clients across crate boundaries. A symbolic shared traversal checks
each yielded reference against the corresponding source position, using a
ghost snapshot to retain that relationship in its loop invariant. The pinned
Rust implementation was reviewed for constructor state and forward advancement.
Clients can also call `slice::Iter::next` directly when its model evidence is
valid for every lifetime. The trusted `IntoIterator` contract for `&[T]`
connects the resulting iterator to the same source references; a cross-crate
`prusti_for!` client verifies symbolic shared counting. Its initializer can
capture the original iterator sequence, enabling a traversal that verifies
each yielded reference against its source position. The pinned Rust
implementation delegates this conversion to `slice::iter`.
The conditional `ExactSizeIterator::len` contract relates the returned length
to the remaining sequence. A verified generic `&I` helper exports the relation,
and clients check owned and shared iterators before and after `next`. A
[reproducer](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/exact_length_mut_ref.rs)
records that method syntax on `&mut I` selects an unmodeled forwarding
implementation; a verified generic helper uses `ExactSizeIterator::len(&*it)`
to retain the relationship while advancing.

Selection checks a region-bearing bound by independently quantifying its
lifetimes, selecting a concrete implementation or builtin, checking its
region-free nested obligations, and requiring rustc's leak check to succeed.
The always-sized reference case is also accepted without lifetime evidence.
It conservatively rejects unproved region requirements, unresolved inference
variables, refinements with conditional preconditions, associated functions
other than directly called external trait or inherent methods, external free
functions and trait impl methods, and pure calls. A
[reproducer](prusti-tests/tests/cargo_verify/stdlib_borrow_spec/known_limits/)
retains the conditional-precondition limitation. Conditional `after_expiry`
pledges on trusted free functions and external inherent methods are selected
only with proven bounds and no conditional preconditions. They require exactly
one borrow wand; conditional trait-method pledges and `assert_on_expiry` remain
unsupported. Trait implementation refinements now preserve impl and method
lifetimes and const-generic parameters across generated specification functions;
cross-crate tests check borrowed results and reject strengthened preconditions
and unsupported generic claims. Generic calls to impure trait methods now use
the trait's two-state contract; behavioral-subtyping checks account for the
mutable arguments' post-state. A concrete owned finite iterator refines its
sequence contract and a generic cross-crate helper consumes it. Negative cases
reject a weaker implementation postcondition and an incorrect state claim.
The same helper consumes verified concrete refinements for borrowed items and
owned non-`Copy` items. Restoring unblocked input permissions at function return
allows the borrowed implementation to return its reference inside `Option`;
regressions cover a partial move and an unrelated mutable reference. Some
lifetime-sensitive calls remain open. Ordinary owned `for` loops can use the specification
marker `for_iterator::<alloc::vec::IntoIter<T>>()` to refer to the actual
compiler-generated iterator in a body invariant. The symbolic-count proof is
verified across crates; nested loops, another same-typed iterator, incorrect
invariants, and invalid marker uses are covered. The marker currently excludes
iterator types with explicit lifetimes and old-state or nested specification
closures. The opt-in `prusti_for!` macro expands to Rust's iterator
construction and an explicit loop, exposing the iterator by name to the body
invariant. A generic,
cross-crate client proves symbolic counting through this form; runtime checks
cover labels, early exit, non-`Copy` items, and single evaluation. A named
iterator cannot be read in a `for`-body invariant because the loop holds its mutable borrow;
the [probes](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/)
record both this Rust error and the hidden-iterator overflow failure. Shared
traversal can refer directly to the source in its loop invariant.

The final encoder passed all 58 iterator checks and all 79 external-spec checks.
The added symbolic early-break helper and its positive, negative, and runtime
clients were checked separately: it processes exactly the smaller of the input
length and a positive stopping point, including for non-`Copy` elements. The
return-permission change passed 76 borrow checks, plus a separate negative test
for a stale owner after a returned mutable borrow. These results satisfy P2-C's
iterator gate.

An attempted universal-lifetime check using rustc's region-insensitive solver
was rejected: it accepted an implementation whose nested type argument had a
hidden `'static` requirement. A negative cross-crate test preserves that case;
the stricter universal check retains a conservative boundary.

### M2 integration

**Verified locally on 2026-10-27:** the final encoder passed 21 sequence, 21
Vec, 27 collection, 79 external-specification, and 58 iterator checks. The
symbolic early-break library and its positive, negative, and runtime clients
passed separately. The return-permission prototype passed 76 borrow checks;
the final encoder also rejected a separate stale-owner claim after a returned
mutable borrow. P2-A, P2-B, and P2-C satisfy their stated gates, so M2 passes.
The documented boundaries on conditional preconditions, shared loop markers,
and iterator forwarding remain outside that gate.

## Phase 1 — External-specification support and unbounded Vec

**Exit gate M1:** verified generic clients use real Vec operations to prove exact
element and length behavior for arbitrary symbolic lengths, without extra
`Copy`, `Clone`, or `Eq` bounds on elements.

### P1-A: Restore a reproducible verification environment

**Prerequisite:** the inspected baseline and repository setup instructions.

Initialize the pinned submodules and install the pinned Rust toolchain and
required components. Follow the [setup guide](docs/dev-guide/src/development/setup.md)
and [build guide](docs/dev-guide/src/development/build.md), then build Prusti with
the repository's `x.py` workflow. Confirm that the invoked compiler matches the
pin, including any local rustup override. Record the Prusti, PCG, Rust, Viper, and
solver revisions used for every milestone result.

Add a focused standard-library test entry point that runs positive, negative,
overflow-enabled, and cross-crate cases. Reuse the existing test infrastructure,
but make these selections explicit so disabled legacy suites cannot hide missing
coverage. Preserve diagnostic checking for negative cases.

**Likely changes:** test harness and CI wiring; environment setup. No collection
encoding changes are needed for this task itself.

**Completion evidence:** the runner executes a known true assertion and rejects a
known false assertion for the intended reason; a cross-crate fixture runs; the
recorded test configuration has overflow checking enabled. This gate must pass
before reporting any collection milestone as verified.

### P1-B: Exercise and strengthen the external-specification path

**Prerequisite:** P1-A.

Audit macro rewriting, target resolution, generic substitution, specification
export/import, and call-site encoding together. Build small probes for:

- `Option::unwrap()`: require `Some` and relate the returned value to its payload.
- `mem::replace()`: relate the return value to the old destination and the new
  destination to the moved source, including a non-`Copy` value.
- Inherent methods, trait methods, concrete trait implementations, associated-type
  projections, lifetimes, and calls through generic bounds.
- A library, a separate specification crate, and a client consuming the contracts
  across crate boundaries.
- Duplicate contracts, mismatched signatures, and invalid trait refinement.

Preserve `#[extern_spec]`, `#[trusted]`, preconditions, postconditions, and existing
refinement syntax. Extend the bundled `core_spec`, `alloc_spec`, and `std_spec`
modules, keeping one authoritative contract for each target. Cross-crate probes
must avoid redeclaring bundled contracts and must check duplicate rejection.

**Likely changes:** targeted rewriter, resolver, generic-substitution, metadata, or
call-site fixes where probes expose failures. Existing machinery should be
extended rather than replaced.

**Completion evidence:** both free-function probes preserve non-`Copy` values;
generic and cross-crate callers receive the expected contracts; invalid inputs
produce useful errors instead of internal failures or silently missing contracts.

### P1-C: Connect collection observations to a sequence model

**Prerequisite:** P1-B and the existing ghost collection types.

Use `Seq<T>` as the logical sequence of element snapshots. Add specification-only
Vec and slice observation helpers, relating their results to length and element
access. Keep these helpers separate from external declarations of real methods;
do not invent standard-library methods inside an `#[extern_spec]` block.

Use snapshot equality for element preservation. Observing a moved argument or an
element must not copy, clone, consume, or invoke equality on its runtime value.
Connect Vec and slice observations so converting to a slice does not lose the
sequence facts. Keep the original allocator and method generic parameters when
matching standard-library signatures; begin client coverage with the default
allocator.

Forward explicit quantifier triggers through the encoder and validate their bound
variable coverage. Inspect generated Viper to establish that trigger annotations
survive encoding. Evaluate PR #1573 for the snapshot and pre-state machinery
needed by mutation contracts, and separately test its documented generic gaps.

**Likely changes:** model observation helpers, their connection to snapshots and
element access, trigger encoding/validation, and any required generic two-state
snapshot fixes. `Seq`, `Ghost`, and `Int` are existing interfaces to reuse.

**Completion evidence:** a generic observation and an arbitrary-index preservation
lemma verify without element trait bounds; explicit triggers appear in Viper;
malformed triggers are rejected with useful diagnostics; the same observation is
available through a separate specification crate.

The first supported element domain is owned values with stable value snapshots.
Reference-containing elements, interior mutability, and nested collection models
are separate extensions. Their support must not be inferred from a scalar test
or from an unconstrained-looking Rust signature.

### P1-D: Specify and prove the minimal Vec surface

**Prerequisites:** P1-A through P1-C.

Cover `new`, `len`, `is_empty`, `push`, `pop`, `as_slice`, and shared indexing.
Resolve the actual method or trait target, including slice and indexing dispatch.
Do not specify a nonexistent inherent Vec method for an operation supplied by
slice dereferencing or a trait.

The following is mathematical notation, not executable Prusti syntax. Let `S`
denote the entry sequence, `S'` the normal-return sequence, `len(S)` its length,
`S[a..b]` its half-open subsequence, and `snapshot(x)` the entry value of an
argument. `++` denotes sequence concatenation. Sequence equality compares element
snapshots, not Rust `PartialEq` calls.

| Operation | Required functional contract |
|---|---|
| `new()` | The result sequence is empty. |
| `len()` | The result, interpreted mathematically, equals `len(S)`. |
| `is_empty()` | The result is true exactly when `len(S) = 0`. |
| `push(x)` | `S' = S ++ [snapshot(x)]`; equivalently, the length grows by one, the appended element has the argument's entry value, and every earlier element is preserved. |
| `pop()` on an empty Vec | Return `None` and retain the empty sequence. |
| `pop()` on a nonempty Vec | Return the last element and set `S' = S[0..len(S)-1]`. |
| `as_slice()` and shared indexing | Vec, slice, and indexing observations agree on length and every valid indexed element. Indexing requires a valid index. |

Express length arithmetic through the mathematical model while preserving the
runtime `usize` relationship and applicable preconditions. Do not remove overflow
checking to make these laws verify.

**Likely changes:** trusted external contracts and small supporting core contracts
after P1-C; any newly exposed encoder blocker gets its own reproducer and fix.

**M1 acceptance cases:**

- Append to a Vec with an arbitrary entry sequence and prove preservation at an
  arbitrary valid index, as well as the appended value and new length.
- Prove a generic push/pop round trip returns the moved value and restores the
  original sequence.
- Verify a loop that constructs a Vec of symbolic length using an invariant
  describing the entire constructed prefix.
- Exercise a non-`Copy` struct with no `Clone` or equality implementations; use
  snapshot comparisons in its generic client proofs.
- Include empty cases and concrete lengths greater than three as regression
  checks, alongside the symbolic proofs.
- Reject wrong-element, wrong-prefix, and wrong-length assertions, and invalid
  indexing. The positive clients must also work across crate boundaries.

M1 is complete only when all these cases run successfully with no fixed length
bound, no unrolling, and no `known-failure` exemptions. Only label a Rust example
as working after it has passed verification on the recorded baseline.

## Phase 2 — Vec, slices, and basic iteration

**Entry gate:** M1. **Exit gate M2:** common mutations, mutable views, and finite
shared/owned iteration preserve the same sequence model in verified clients.

### P2-A: Expand Vec and shared slice contracts

**Prerequisite:** M1 and the common Vec/slice observation model.

| API group | Required behavior |
|---|---|
| `clear`, `truncate` | Produce the empty sequence or the retained prefix; truncating above the length preserves all elements. |
| `append` | Concatenate the two entry sequences and empty the source Vec; relate both post-states to both entry states. |
| `insert`, `remove` | Describe the inserted/returned value and the shifted prefix and suffix, with the appropriate bounds. |
| `swap_remove` | Return the removed element and describe replacement by the former last element; do not promise stable order. |
| Shared lookup and range indexing | Return the selected element/subsequence or `None` where the API uses checked lookup; preserve bounds and ordering facts. |
| `split_at` and element swapping | Describe the two complementary subsequences or exchanged elements and preserve everything else. |

**Likely changes:** external contracts, common sequence lemmas, and resolution of
slice/trait targets. Isolate any snapshot or slice-encoding failures separately.

**Completion evidence:** symbolic-length append, insertion, removal, truncation,
and slice examples verify; tests distinguish `remove` from `swap_remove`, cover
empty/end boundaries, and reject incorrect framing or ordering claims.

### P2-B: Mutable views and borrow-expiry contracts

**Prerequisite:** P2-A and validated two-state snapshot support.

Specify `as_mut_slice`, `get_mut`, `IndexMut`, and `split_at_mut`. Relate the
initial referent to the selected entry value or subsequence. Use borrow-expiry
contracts to restore the owner's model from the referent's final state, preserving
length and all unaffected elements.

Handle `Option<&mut T>` explicitly: the `None` branch preserves the collection;
the `Some` branch carries the borrow's obligations through matching and reborrows.
For split slices, restore both disjoint regions without duplicating permissions.
Use the existing [pledge interface](docs/user-guide/src/verify/pledge.md), extending
encoding or diagnostics where the required reference shape is unsupported.

**Likely changes:** deep/pre-state snapshot handling, trait pledges, borrow-expiry
encoding, and generic reference or slice framing. A postcondition about a returned
reference alone is insufficient for this task.

**Completion evidence:** verify one-element updates, missing lookup, reborrowing,
and changes through both halves of a disjoint split. After expiry, prove the
owner's updated sequence and reject claims that unrelated elements changed or
that the modified element kept its old value.

### P2-C: Shared and owned iteration

**Prerequisite:** the sequence model and generic trait/associated-type probes;
complete P2-A and P2-B before declaring the whole phase finished.

Require contracts for `slice::Iter` and `vec::IntoIter`: construction,
`Iterator::next`, exact remaining length, and the `IntoIterator` operations needed
by basic `for` loops. Model remaining elements as a sequence. A successful `next`
exposes its head and leaves its tail. Shared results remain linked to the source
elements; owned results transfer values without copying.

Introduce an opt-in iterator model capability for generic helpers and test model
refinement for `Item = T` and `Item = &T`. The generic helper must have the
required model evidence; a bare `I: Iterator` cannot assume a concrete
implementation's sequence contract.

Before expanding the library, test whether the capability can be exposed across
crates, refined by concrete implementations, connected to implementation-specific
state, and consumed through generic calls. Include lifetimes, associated types,
and non-`Copy` elements in these probes. Treat a failing probe as an infrastructure
blocker rather than substituting an unverified concrete-only demonstration.

Apply finite-sequence and permanent-exhaustion laws to the supported concrete
iterators. Do not impose them on every `Iterator`: the trait allows implementations
to resume after returning `None`. ([Iterator contract](https://doc.rust-lang.org/std/iter/trait.Iterator.html#tymethod.next))

**Likely changes:** iterator observation/model interfaces, trait specification
refinement, associated-type and lifetime handling, cross-crate model reuse, and
any necessary support for loop invariants after `for` desugaring. Higher-order
callback contracts are not prerequisites for these concrete iterators.

**Completion evidence:** verify shared traversal, owned traversal of non-`Copy`
elements, exact remaining lengths, empty and exhausted cases, early exit, and
generic modeled-iterator clients. Include symbolic-length `for` loop proofs and
negative cases for incorrect yielded values or state advancement.

**M2 acceptance:** P2-A, P2-B, and P2-C all pass their gates. Investigate `IterMut`
after mutable-access contracts work, but keep it outside M2. Defer adapters such
as `map`, `filter`, and `fold`, callback contracts, closure captures, and general
`collect` support.

## Phase 3 — HashMap and HashSet

**Entry gate:** M2. **Exit gate M3:** finite-map and finite-set client proofs work
with explicit key models, including mutable access to a selected map value.

### P3-A: Establish key semantics and collection observations

**Prerequisites:** reusable external specifications, logical `Map`/`Set`, and the
model and snapshot infrastructure from the earlier phases.

Use a logical key projection whose equality corresponds to Rust key equality.
The map model associates projected keys with value snapshots; the set model
records projected-key membership. Start with integers and a custom key whose
equality ignores a non-key field. Do not identify snapshot equality of the whole
key with Rust `Eq`.

Require verified evidence or explicitly recorded trusted assumptions for key
equality laws, equal-key hash compatibility, and key-model stability while stored.
Unequal keys may have equal hashes. Keep the hashing algorithm abstract and do not
assume collision freedom. These requirements reflect the standard library's
[HashMap key contract](https://doc.rust-lang.org/std/collections/struct.HashMap.html).

Preserve original generic method signatures, including lookup's borrowed key
parameter. Activate stronger behavioral contracts only when the relevant model
evidence is available. Begin lookup proofs with the stored key type itself;
heterogeneous borrowed keys require a later model correspondence proving that
their equality and hashing agree with those of the stored keys.

**Likely changes:** specification-only key and collection model interfaces,
conditional contract/refinement support, and generic normalization where exposed
by the probes. Arbitrary user `Eq`/`Hash` implementations are not silently assumed
to satisfy the stronger modeled contract.

**Completion evidence:** model probes distinguish equal keys with different
snapshots from unequal colliding keys, and clients without the required evidence
cannot use the stronger guarantees.

**Verified locally:** the [key-model fixture](prusti-tests/tests/cargo_verify/stdlib_hash_spec/README.md)
passes 20 checks. It verifies generic projections, logical-map insertion,
integer keys, and a custom key whose equality ignores a field. The fixture
proves the custom equality laws and distinguishes equal keys with different
snapshots from unequal keys with the same hash observation. The custom `Hash`
implementation is explicitly trusted and reviewed as writing the same byte
for every key.

The bundled `KeyProjection` describes a key's logical identity without
asserting any Rust equality or hash laws. The separate unsafe `HashKeyModel`
opt-in explicitly assumes equality/projection agreement, equality laws,
equal-key hash compatibility, and stability while stored; the verifier does
not prove those assumptions merely from an implementation. The bundled `u32`
and fixture custom-key implementations are reviewed. Trusted map and set
observations use projected keys and do not expose stored representatives.
Conditional `HashMap::new` and `HashSet::new` contracts establish empty models;
negative clients without the key-law evidence cannot infer those facts. No
other hash-collection operations are specified by this milestone.

### P3-B: Core map/set operations and mutable map values

**Prerequisite:** P3-A; the `get_mut` work also depends on P2-B.

**Verified locally on 2026-10-27:** conditional contracts for map/set `len` and `insert` connect
the returned length, insertion state, previous map value, and set insertion
boolean to the logical observations. Cross-crate clients cover generic keys,
integer keys, replacement by equal custom keys with different snapshots, and
separate colliding keys. The original borrowed-key signatures for map
`contains_key`/`get` and set `contains` have conditional contracts when a
sealed same-type capability proves the query type is the stored key type.
The same capability supports conditional map/set removal, including old-value
or boolean results and preservation of unrelated entries. Conditional clear
and emptiness contracts complete the other basic operations listed below.
For `get_mut`, the conditional postcondition establishes presence and the
returned reference's initial value. Its conditional expiry pledge updates only
that projected key, preserving other entries and cardinality; an absent lookup
preserves the complete observation. Generic, custom-key, and non-`Copy` value
clients exercise these contracts. Heterogeneous lookup remains deferred.

The complete hash suite passed all 51 checks at `b15a36e4a`, including
cross-crate generic clients, runtime checks, and negative cases with the
intended diagnostics. The suite took 958.96 seconds in total; its longest
individual check, `collection_remove_pass`, took 59.22 seconds. Run it with
`./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite hash` after
building. The focused runner enables overflow and panic checks, disables the
verification cache, and sets a 90-second verifier timeout. The environment
uses PCG `e54bf675468d44c5e3c0d0e2eca6b7e837e11270`, Rust
`1.99.0-nightly (d453bdd8f 2026-08-14)`, Viper `v-2026-09-01-0834`, Z3
`4.16.0`, and Java `21.0.2`. The full external-spec suite passed 97 checks
and the existing iterator suite passed 61 checks at the same encoder revision;
the later test-only `IterMut` additions were verified separately. The full
borrow suite passed 77 checks at `b15a36e4a`, taking 1074.21 seconds in total;
its longest individual check was `frame_pass` at 69.26 seconds. An earlier
parallel run timed out on `inclusive_missing_pass`; that case passed both a
separate retry and this complete run. The 36 `prusti-specs` unit tests also
passed. These results satisfy M3 within the
explicit key-model and same-key lookup domain described above; they do not
establish the deferred cases below.

| Collection | Required operations and laws |
|---|---|
| HashMap | Construction, `len`, `is_empty`, `contains_key`, `get`, `insert`, `remove`, and `clear`. Relate observations to the finite map, distinguish insertion from replacement, return the previous value where required, and preserve unrelated entries. |
| HashSet | Construction, `len`, `is_empty`, `contains`, `insert`, `remove`, and `clear`. Specify membership, exact cardinality changes, and the booleans distinguishing new/present or removed/absent elements. |
| HashMap mutable lookup | `get_mut` initially exposes the selected value and, after borrow expiry, updates only that value. Preserve keys, cardinality, and unrelated values; an absent lookup preserves the entire model. |

HashMap replacement retains the existing stored key even when the incoming equal
key has a different snapshot. Do not assert replacement of the stored key in an
insertion contract. APIs that expose stored representatives will need that
identity represented explicitly before they are added. ([HashMap insertion](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.insert))

**Likely changes:** external contracts and model helpers, reusing borrow-expiry
infrastructure for `get_mut`. Generic map/set or lookup-reference encoding failures
must be isolated and fixed rather than hidden behind trusted client wrappers.

**M3 acceptance cases:**

- New and duplicate insertion, with exact cardinality and old-value results.
- Removal and lookup for both present and absent keys.
- Distinct keys with colliding hashes and equal custom keys with different
  snapshots; preserve unrelated entries in both cases.
- Mutation through `get_mut`, followed by observation of the updated owner.
- Empty/clear cases and incorrect membership, cardinality, old-value, and framing
  assertions that must fail.

Defer entry APIs, collection iterators, heterogeneous borrowed-key lookup,
arbitrary custom hashing behavior without model evidence, and nested mutable
collection models. Do not promise deterministic iteration order.

## Mutable-iterator follow-up plan

**Current status (2026-10-31):** IM0–IM4 implementation and focused proof gates
are complete, and the combined IM5 proofs and matching controls pass on both
Ferrum and Silicon. Native loops now prove whole-prefix values and exact
early-break suffix framing for reference elements, including separate-crate
slice/Vec clients. Generic state preservation, non-`Copy` referents, two-reference
wrappers, empty/zero-sized cases, and the previously archived controls have
paired focused evidence. The user authorized the full stdlib matrix on
2026-10-31; it is now running, with failures to be fixed before qualification.
This remains focused qualification until that gate passes. Ferrum stays pinned to `afdf10c`, with
one worker and branch parallelism off. The sections below preserve the work's
chronological evidence, beginning at Prusti `22ea24a6b` and PCG `a4eb802`.

The execution order is IM0, IM1, IM2, IM3, IM4, then IM5. Functional loops
first use owned elements with stable snapshots. Reference support then adds
the separate permissions needed to observe and update nested referents.
The final gate combines both extensions.

| Step | Deliverable | Depends on | Exit gate |
|---|---|---|---|
| IM0 | Minimal reproducers and resource/state inventory | Native core gate | Complete: [17 probes and the resource ledger](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/README.md#im0-native-loop-values-and-references-inside-elements) record the observed failures and controls. |
| IM1 | Proven loop restoration summary | IM0 | Complete: the standalone summary, negative checks, and untrusted Rust expiry-pledge controls pass. |
| IM2 | Functional native iterator loops | IM1 | Focused gate complete: Rust clients prove whole-slice updates and exact early-break prefix/suffix behavior for symbolic lengths. |
| IM3 | Concrete reference-containing slice elements | IM0; implemented after IM2 | Focused gate complete: reads, writes, framing, and expiry work for shared references, mutable references, and reference-bearing structs/enums. |
| IM4 | Generic and cross-crate reference resources | IM3 | Focused gate complete: generic definitions retain the same referent permissions under substitution and through Vec/slice views. |
| IM5 | Combined acceptance and support update | IM2, IM4 | Focused paired proofs complete: symbolic reference-element loops prove final values and framing across crates. Full stdlib matrix and final-pin merge qualification are in progress. |

### Full matrix qualification (2026-10-31; baseline complete, repairs in progress)

The complete `final339` baseline finishes with **1,755/1,842 checks passing**:
875/921 on Ferrum and 880/921 on Silicon. All 24 jobs have complete coverage,
matching configuration and source hashes, and no unexpected or missing cases.
This is the frozen `225eb7714` encoder, PCG `269cb9a`, and Ferrum `afdf10c`;
it is not qualification of the pending Ferrum upgrade or repair candidates.

| Baseline job group | Ferrum | Silicon |
|---|---:|---:|
| Extern, sequence, Vec, collections, resources, hash | 249/249 | 249/249 |
| Borrow | 155/164 | 156/164 |
| Ordinary iterator | 389/421 | 391/421 |
| Heavy iterator | 51/54 | 52/54 |
| Cross-crate iterator resources | 11/11 | 11/11 |
| Cross-crate generic resources | 9/10 | 9/10 |
| Cross-crate prefix resources | 11/12 | 12/12 |

The 87 failed checks comprise 52 positive checks, 28 negative-diagnostic
mismatches, and seven library checks. No negative client is unexpectedly
accepted. Twelve failed checks report a verifier timeout; these and negatives
that fail before their intended assertion remain failures. Full per-case
diagnostics and timings are in `summary339.json` and `final339/` under the
artifact directory below.

The final cross-crate jobs expose two additional library findings. Generic
preservation fails the `body_restore_prefix` obligation at `specs/src/lib.rs:24`
on both backends (425.90 seconds on Ferrum, 220.67 on Silicon), while all its
clients and controls pass. The prefix-update library reaches Ferrum's
600-second verifier bound, while Silicon proves it and both backends pass its
clients and controls. Follow-up 360 runs both complete suites with rebuilt
baseline 352 and Ferrum `0e048a4`/Silicon at unchanged bounds, serially after
the already queued independent repair gates. The earlier focused IM proofs
remain historical evidence; they do not replace these current library failures.

`scripts/test_stdlib_matrix.py` runs all eleven fixture suites on Ferrum and
Silicon, serially, with one verifier and Ferrum branch parallelism disabled.
The initial plan contains 663 clients (1,326 client/backend pairs), plus 22
library runs, Rust checks, runtime positives and the existing extra controls.
The runner checks exact coverage, source and driver hashes, bounds and backend
settings before accepting a completed job for resume. Completed failures
retain their coverage but never count as successful qualification.

The baseline uses a detached source snapshot at `d749dd359` and executables
copied after rebuilding this branch with `./x.py build --all --locked
--features prusti/ferrum`. Build provenance, the coverage plan, per-case logs
and results are under `target/stdlib-matrix-20261031-1236/`. Ordinary jobs retain
the 180-second program bound; the resource suites and previously qualified
heavy iterator prefix/early-break cases retain 600 seconds. Both use 30-second
assertion and 100-millisecond solver-check bounds. No helpers are filtered.

The background supervisor monitors initially at 10, 30 and 60 seconds, then
backs off from five minutes to at most an hour while healthy. Child completion
wakes it immediately; newly observed failures reset the interval. It continues
through failed jobs to preserve a complete baseline. The snapshot keeps fixes
on `main` from changing an in-flight suite.

The first extern suite had seven matching failures on each backend. Investigation
found two encoder defects: an unused consumed generic marker created a spurious
expiry stage, and a resource-free lifetime anchor on a generic result hid the
mutable destination's retained resources. The latter also affected the sequence
library's `Vec<T>` replacement. Explicit input selectors were initially suspected
to be necessary, but the existing conditional specifications are valid and have
been retained unchanged. No generic type bounds or negative diagnostics changed.

The initial dispatcher was stopped between suites after preserving both extern
results (90/97 each) and sequence/Ferrum (21/22). Fix validation uses a new build
and copied executables in `/tmp/prusti-stdlib-matrix319-bin`. The extern suite now
has two additional reference-resource regressions, with all 100 checks passing
on each backend under `fix319b/`. The full plan therefore has 665 clients and
1,330 client/backend pairs. Sequence passed 22/22 on Ferrum at 180 seconds
(library command: 184.52 seconds including startup), while Silicon exhausted
the program bound in the generated resource-conversion helpers (187.82-second
command). The matrix now assigns this generic Vec resource suite the existing
600-second heavy-proof budget on both backends. Assertion/check bounds remain
unchanged; the timed-out attempt is preserved and does not count as a pass.
A runner manifest
bug exposed by these newly added files has also been fixed and unit-tested.
Completed evidence from `fix319/` is retained and validated before reuse.

After paired validation, commit the fixes and run the complete matrix against
the final source snapshot and build. Preserve earlier failed runs; do not merge
old-build results into a claim of final-build matrix qualification.

The first corrected run is preserved under `final321/`, using the frozen
`b238ad198` checkout and the rebuilt `f25f9616f` encoder (the intervening change
only adjusts the matrix budget and this document). `build321.json` records the
six executable hashes; `adoption321.json` checks the identical fixture sources
and build before reusing the 200 paired extern checks. All remaining jobs run
serially from that snapshot. The sequence/Ferrum rerun at the 600-second bound
has passed 22/22. This incomplete run is superseded by a complete rerun of all
24 jobs after the encoder repairs below; it is not final-build qualification.

The first twelve corrected jobs (extern, sequence, vec, collections, resources,
and hash on both backends) passed all 498 checks. Borrow/Ferrum completed with
146/164 checks passing and Borrow/Silicon with 147/164. Their failures include an assignment graph cycle, implicit
pledges made ambiguous by opaque payload stages, premature restoration of
generic collection resources, and an aggregate conversion-helper proof whose
failures vary with client context and backend. Silicon accepts the inclusive
positive rejected by Ferrum, but reports the same helper failure in
`range_wrong_frame`. A wrong-assertion client that also reports a helper failure
is counted as failed, despite reporting its intended error. Both baseline
results are retained; the dispatcher is paused at the suite boundary while
focused fixes use the single verifier.

The assignment cycle has a small PCG reproducer: replace an indexed generic
element, then request the input's return dependencies. A historical expansion
source was redirected to the replacement value, making an earlier aggregate
snapshot depend on its later expansion. The repair must distinguish historical
target places from live future targets; testing also caught and rejected an
overbroad version that broke the existing nested-reborrow corpus case 216.
Validation includes both NLL and Polonius, the numbered corpus, and paired Rust
positives/negatives. No failure family is closed until its Rust proofs pass.

PCG `269cb9a` implements that repair and passes all 250 numbered cases, nine
function-shape cases, four call-projection-map cases, the loop-coupling case,
and seven opaque-body cases, including the new indexed-replacement regression
under both borrow checkers (`pcg329.log`). The rebuilt Rust clients now encode
past the cycle, exposing permission-proof failures that remain separately open.
The root pin is committed independently of the ongoing encoder proof repairs.

Build 330 tests two further repairs: attaching an implicit pledge to a unique
concrete-lifetime stage despite an additional opaque payload stage, and proving
the resource-conversion observation equality before changing its permissions.
Build 332 additionally reads the current array directly after its physical fold
and distinguishes saved element resources by the expansion's lifetime label.
Previously, opening the same slot again replaced the first opening's capture,
so both historical closures used the second resource identity. Frozen builds,
patches and paired results are retained under `fix330/` and `fix332/`; these
focused jobs do not replace the complete matrix qualification.

Trials 331–335 simplified the current snapshot, added an explicit equality to
the canonical sequence element, changed permission-query order, and checked a
fresh indexed opening directly. They did not resolve the positive failures and
have been removed. The positionless error reports only the failing resource,
which also occurs in the return-wand footprint: the initial opening was not the
failing assertion. Two historical expansions can refer to the same unchanged
element, and conjoining their exposed footprints demands its permission twice.
Build 336 retains the distinct opening captures and checks footprint parts with
separate assertions. It still checks every restoration action and the complete
wand consequent. This does not establish complete qualification; a genuine
missing resource must continue to fail.

Build 336 passes `frame_pass` on Silicon, while Ferrum advances past the
footprint assertion and fails the current-resource postcondition. Both still
reject the generic Vec-view positives; the three framing negatives pass on
both. Build 337 tests a checked temporary `true --* immediate_resources` wand
around return-wand packaging. Its purpose is to reserve immediate output
resources while the restoration script uses its antecedent, then return them
by applying the temporary wand. This is a pending experiment, not an assumption
that those resources already exist: packaging the temporary wand must prove
them. The broader generic call-expiry handoff remains a separate open problem.

The paired borrow rerun also exhausted the 180-second whole-program limit in
`inclusive_slice_pass` on Silicon (185.64 seconds including startup); Ferrum
had exhausted it in `vec_lookup_inclusive_pass`. The next complete matrix puts
the borrow suite under the existing 600-second generic-resource program budget,
like sequence and resource suites. Assertion and solver-check limits remain
30 seconds and 100 milliseconds. Those timed-out attempts remain failures;
only a completed proof under the recorded new bounds can pass.

Build 337 passes `frame_pass` on both backends (41.10/51.71 seconds). Its
concrete iterator-framing positive exceeded the focused 180-second bound.
Older evidence already used 600 seconds for that case, with successful runs
at 168.56, 228.47 and 298.77 seconds; this does not establish a performance
regression from the reservation. Build 338 narrows the reservation to opaque
generic resource outputs and validates the iterator controls at 600 seconds.
The full runner now classifies all 39 resource/generic iterator clients as
heavy, and verifies the iterator library once per backend with that group.
Ordinary clients retain 180 seconds. The partition test covers the previously
misclassified framing case and library coverage when a suite has no heavy cases.

Build 338's opaque-only reservation passes `frame_pass` on Ferrum and Silicon
(40.34/44.54 seconds). The implementation is retained: both the reservation and
the original restoration wands are proved, and applying the temporary wand
returns the immediate resources after packaging. The selector and iterator framing controls pass all 21 checks on each backend
at 600 seconds, including exact negative diagnostics. Together with the paired
three-check frame runs, this is 48/48 focused checks. No specification, type bound, or negative
expectation is weakened.

The complete rerun is now active under `final339/`: 24 jobs, 665 clients
(1,330 client/backend pairs), and 1,842 checks including libraries, runtime
positives and auxiliary controls. The frozen source is `225eb7714`, with PCG
`269cb9a` and Ferrum `afdf10c`. Rebuilding the clean committed branch reproduced
all six build-338 executable hashes exactly; `build339.json` records provenance.
No earlier-build results are adopted. The stopped `final321/` dispatcher was
retired after preserving its 791/826 completed checks in `superseded321.json`.

The first fourteen `final339` jobs are complete: the initial twelve pass
498/498 checks, borrow/Ferrum passes 155/164, and borrow/Silicon passes 156/164.
All borrow negative clients produce exactly their expected diagnostics. Both
backends time out on `vec_lookup_inclusive_pass` at 600 seconds. Ferrum alone
adds an aggregate-helper failure in `vec_lookup_range_pass`; Silicon accepts
that client. These results do not close the generic handoff or helper-stability
issues below.

The fifteenth job, ordinary iterator/Ferrum, completes with 389/421 checks
passing: 19 positive failures and 13 negative-diagnostic mismatches. None of
those negative clients is accepted; eleven report extra conversion/precondition
errors, and the two wrong-loop-invariant clients fail earlier while restoring
owned Vec resources. Silicon has started and accepts both concrete replacement
positives rejected by Ferrum. The optional-element positive fails on both,
while the nested-element positive exhausts the ordinary 180-second bound on
both. Final totals and remaining failures still require the complete run.

The ordinary iterator job exposes another budget-classification omission:
reference-element, generic-array and sequence-state tests do not necessarily
contain `resource` in their names. Earlier accepted runs include generic-array
proofs of 237–568 seconds and a reborrow control of 280 seconds. The next matrix
plan puts these resource families, native reference/holder clients and Vec
reference views under the existing 600-second resource budget. It also retains
that budget for the historically long continue and wrong-suffix restoration
controls. Ordinary storage/value clients keep 180 seconds. The partition is now
193 ordinary and 152 heavy iterator clients per backend; total coverage remains
665 clients, 1,330 client/backend pairs and 1,842 checks. Five runner/watchdog
tests pass, including coverage, library selection and these newly classified
families. `historical-heavy-iterator-evidence.json` records the timing evidence.
The running frozen `final339` plan is unchanged, and its short-budget failures
remain failures.

The concrete replacement failures have a relevant upstream backend repair:
Ferrum `85fc9d8` preserves independent permission-map definitions when a wand
package closes. Its upstream review also rechecks the Pair wrong-second
negative that blocked the earlier performance upgrade. Build 349 rebuilds
this branch with that pin and no encoder changes; current-branch adapter and
paired Rust qualification are queued after the baseline and handoff probes.
The broader concrete-resource reservation proposed in build 348 is preserved
but deferred without verification, so the backend change can be assessed
independently. See [the upgrade record](prusti-ferrum/VALIDATION.md) for provenance
and the acceptance gate. The committed pin remains `afdf10c` until that gate
has been reviewed.

The optional-tail family has a separate, concrete conversion gap. The existing
minimal `mut_take_slice_pass` takes an `Option<&mut [u32]>` through
`mem::replace`. Its generated whole-slice resource pack requires element type
facts, but the nested-reference conversion does not invoke the checked typing
traversal used for incoming collection storage. Candidate 350 adds that
traversal, with the slice's actual extent, after converting the referent's
physical predicate and before packing its resources. It restores the original
physical predicate afterward. No precondition or resource obligation is removed.
The candidate builds, and the minimal positive, negative and generic region
positive encode successfully with the additional checked calls. These are
encode-only results, not verified passes. Paired minimal tests against builds
349 and 350 are queued after the upgrade gate; broader array/reference
regressions run only if the candidate passes the minimal gate on both backends.

The owned-loop failures expose a different graph issue: one opaque input flows
to several loop outputs, but the graph leaves separate restoration edges that
each demand that same input resource. PCG `8fce1e6` couples those outputs into
one obligation when they share a single opaque input. The new regression fails
before the change and passes afterward with NLL and Polonius. The 250-input
corpus, nine shape tests, four projection-map tests, both loop tests and seven
opaque-body tests pass. The change is pushed to the PCG fork; the main project
still pins `269cb9a` until paired Rust validation succeeds. Build 351 isolates
this graph repair from the pending typing change in build 350. Its gate checks
the owned-loop positives, both wrong-invariant negatives, an added explicit
`false` after iteration, and existing mutable-iterator controls on both backends.

Following the user's Ferrum update, the upstream checkpoint-349 review confirms
that all three Ferrum-only failures verify with `85fc9d8`, including the range
helper positive; its exact inclusive-frame negative also succeeds in two fresh
runs. The subsequent `09a28fb`/`0e048a4` changes fix raw Viper frontend overhead,
not the injected-AST Rust path. Build 352 rebuilds this branch with `0e048a4`
and no encoder changes. Builds 353 and 354 independently rebuild candidates 350
and 351 with that same pin. Their unstarted older dispatchers were retired;
the new paired gates follow the complete matrix and alias probes using one
worker. The handoff models also had a duplicate local/let-bound name, now
renamed before their first verification attempt. Original models and all
earlier artifacts remain preserved. No parser rejection, incomplete replay,
or upstream benchmark exit status is counted as a local verified pass. See
[the current upgrade record](prusti-ferrum/VALIDATION.md) for provenance and
the targeted upstream evidence.

The local upgrade gate is now reviewed and `0e048a4` is adopted for development.
All 18 adapter tests and six scalar-handoff model expectations pass. The four
paired Rust runs account for 108 checks: 86 pass, and all 22 failures were
already failed baseline entries. No negative is unexpectedly accepted. Both
concrete replacement positives and Vec range lookup now pass on rebuilt Ferrum;
the exact Pair wrong-second rejection also passes on both backends. Coverage,
source/build provenance, comparisons and retained failures are recorded in
`qualification352.json` and `audit352.json`. Inclusive Vec lookup still times
out in Ferrum's aggregate helper and fails its functional postcondition on
Silicon. The final combined full matrix remains required; this pin adoption
does not accept any pending encoder or PCG candidate.

The heavy Ferrum baseline has also exposed a 600-second timeout in
`mut_native_resource_early_pass` and a wrong-obligation rejection in
`mut_native_resource_generic_prefix_wrong_state`: the latter fails at the loop
restoration declaration, before its intended final assertion. These remain
failed entries. Follow-up 355 repeats each affected family on build 352 and
Silicon with the same bounds; the complete baseline continues unchanged.

Follow-up 355 is complete with unchanged bounds: Ferrum passes 7/8 checks
and Silicon passes 8/8. Both backends reject all three negative clients at
their intended final assertions, including the former wrong-obligation
failures. The early-break positive still times out after 600 seconds on
Ferrum, while Silicon verifies it in 143.49 seconds. The generic-prefix
positive verifies on both backends. Source/build provenance and complete
selected coverage pass the audit (`qualification355.json`, `audit355.json`);
this closes the diagnostic follow-up, not the remaining positive timeout or
the final combined matrix. Raw replay 359 will skip these now-passing negatives.

Direct owned-reference controls now exercise `Vec<&mut u32>::into_iter`, a
write through its yielded reference, owner restoration, and an explicit false
assertion. Generated contracts reveal a missing ownership model: `IntoIter`
has raw pointers but no logical remaining-element resources. Conversion returns
the Vec's entry resources immediately, while `next` can produce an element
resource. No false assertion has yet been verified in these new controls;
their paired baseline and candidate checks are required before drawing a
consistency conclusion.

Candidate 358 adds an owned logical slice and length to `IntoIter`, and ties
`remaining` to that same slice through an ordinary pure definition. This also
requires resolving concrete associated return types before constructing their
resource interface. PCG `967a323` normalizes those definition signatures without
erasing regions, retaining the original interface when any associated type
remains opaque. The new missing-flow regression fails before normalization;
the 250-input corpus, ten shape tests, six projection-map tests, loop test and
seven opaque-body tests pass afterward. Distinct lifetime origins are checked
with NLL and Polonius. The PCG changes are pushed to the fork, but are not yet
the main pin. Build 356's logical-tail-only encoding still omitted the
associated output resources, so its waiting verifier dispatcher was retired
without running. Candidate 358 first gates the owned-reference positive and
exact false control on both backends, then runs the library, owned-loop and
existing mutable-iterator controls only if that minimum succeeds. This remains
an unqualified candidate, separate from the optional-slice typing and opaque
loop-coupling experiments.

A closer reading separates a second generic Vec issue from the current-owner
handoff. In `open_vec_pass`, the first indexed opening fails at `_tmp14`, before
any element is replaced. The generated `IndexMut::index_mut` contract returns
the result's physical storage but no opaque resource group for its associated
output type. Recursing into the referent normalizes `Index::Output` to
`SliceIndex::Output`, which may lose the original projection identity. An
isolated alias-normalization experiment is being inspected; it is not part of
`final339/` or an accepted repair. Returning an additional group also requires
checking its restoration dependencies and negative permission controls.

The inspection confirms two normalization sites: the top-level signature
projection lookup and recursive descent into its referent. Build 342 preserves
the opaque group through both sites, but that alone does not establish a sound
contract. The signature graph also needs to retain dependencies from the
associated type's arguments to its output resources. PCG `19327b5` adds that
conservative capture relation without changing Rust's outlives relation. New
`IndexMut` and `Iterator::Item` regressions fail before the change and pass
afterward; the 250 numbered inputs, eleven shape cases, four projection-map
cases, loop-coupling case and seven opaque-body cases pass (`pcg344-after.log`).
The PCG commit is pushed to the fork, but the root pin remains `269cb9a` pending
integration validation.

Rebuilt candidate 345 combines those graph edges with the normalization repair.
Encoding `open_vec_pass` and a new mutable-reference-element control exposes
`empty dependent resource restoration` before any solver runs. The new graph
has two independent target sets and a source that depends on their union;
stratification leaves that source with restored frames but no original target.
Removing the rejection is not yet justified: consuming an opaque output to
restore one source can discard permissions needed by the later source.
The candidate remains an experiment. Its frozen binaries, source patch and
failed encode-only attempts are retained under `build345.json`,
`source345.patch` and `encode345/`.

The paired associated-output probes now confirm these limitations. Candidate
342 passes 7/13 checks per backend, still failing the original generic range
positives. Candidate 345 passes 9/22 selected borrow checks and 21/21 iterator
controls per backend; its additional dependency edges also prevent the two
existing Vec-index negative clients from reaching their intended assertions.
Neither candidate is adopted. The new reference-element fixtures initially
used an unsupported nested dereference in a pure assertion. Corrected copies
read the values into scalar locals first; all four then expose the same
`empty dependent resource restoration` during encoding (`controls361.json`
and `encode361/`). The original invalid fixtures and results remain recorded.

The next handoff experiment checks a smaller scalar projection first. Its
outer wand must return current-owner resources together with a continuation
from those resources to the entry-owner resources. Both the immediate resource
reservation and the continuation must be packaged with checked proofs. The
positive, false-assertion and missing-current-resource models are queued after
the matrix and associated-output probes, using the same single worker on both
backends. A successful model would still need caller-side continuation
tracking, conditional and loop composition, and a separate indexed-replacement
proof before it could repair the broader generic collection failures.

The scalar handoff gate now passes on Ferrum `0e048a4` and Silicon `6b540aa4`.
Both backends verify the positive and reject exactly the false assertion and
the application of a continuation after its required current resource has
been removed. The three checked models are retained as `owner_handoff*.vpr`
under the iterator fixture's `known_limits/` directory; they are byte-identical
to the qualified inputs. `handoff352-qualification.json` records hashes,
commands, exact error positions and timings. These are scalar permission and
scalar-value-frame proofs only. They do not yet establish a resource-state
frame, indexed replacement, current-owner PCG nodes, or continuation tracking
through branches and loops.

The next checked models separate two additional obligations. Probe 365 adds a
referent-state frame to the delayed continuation and checks that a later write
through the current group survives restoration of the entry group. Probe 366
composes that continuation through an arbitrary number of scalar projection
calls in a loop, retaining one current-to-entry obligation. It is gated on
365 and includes false-value and missing-continuation controls. Both remain
unverified; neither is a Rust encoder integration or an indexed-replacement
proof.

Probe 371 separately models a two-element reference-bearing owner. Closing
the selected element returns the current owner and a checked nested
continuation: opening that owner returns its current selected resource, then
accepts the original selected resource to restore the entry owner. This avoids
requiring duplicate permissions when the selected element was not replaced.
The model exercises both unchanged projection and replacement while the old
reference remains usable across a write through the current owner. Its false,
wrong-old-value and missing-current-resource controls are queued. This is an
unverified permission experiment, not a generic indexed encoding or a
resource-state frame across expiry.

The state, loop-composition and indexed-replacement model gates are now
qualified: all 24 expectations match on Ferrum `0e048a4` and Silicon `6b540aa4`.
The paired negatives reject exactly the false/wrong-value assertion or missing
permission/continuation. Probe 371's initial textual matcher expected the source
field syntax, while both backends print its ADT getter; the exact line-76
assertion was reviewed and the original mismatch is retained. Byte-identical
models are committed as `owner_handoff_{state,loop,indexed}*.vpr`, with hashes,
commands and results in `handoff{365,366,371}-qualification.json`. Rust/PCG
continuation integration, arbitrary indexed sequences and the indexed expiry
state frame remain open.

The indexed `Option<&mut U>` failure has a different cause. Trace build 363
confirms that all historical array-capture lookups succeed. The failed read
occurs when constructing an expiry wand at a heap where the enum is unfolded
into its discriminant and active fields. Build 364 extends the existing
checked struct reconstruction to enum snapshots, preserving the original heap
label and testing actual field permissions. The original positive and wrong
value control, a false control, and replacement with an escaped old reference
all encode. Its paired verifier gate is still pending; encoding alone does
not qualify the repair.

The new-pin inclusive Vec lookup still reaches the 600-second timeout inside
`resource_pack_aggregate_f569555111cf551b`. For its generic element, the helper
recursively unfolds and refolds identical indexed permissions while comparing
two recursive sequence observers. Candidate 367 keeps those canonical
permissions in place and uses the existing canonical sequence observer
directly for type parameters. It does not treat an unknown type as having no
resources. Own-branch rebuilding and three encode-only controls pass; the
generated program no longer contains that identity conversion or duplicate
observer. The paired positive/frame gate, followed by iterator and cross-crate
regressions, remains queued and unqualified. Concrete representation-changing
conversions retain their checked helpers.

The optional-slice typing candidate 353 fails its minimum gate on both
backends, so its broader tests were skipped. It replaces the original
precondition error with an untranslated `fold.failed:insufficient.permission`.
Exact raw replay 368 locates the same enclosing-reference fold at Viper lines
1310–1313 on both backends: the whole-slice group has been packed using a
concrete slice length, while the generic reference still selects its metadata
through an unresolved associated type. The
generated trait domain has the `Sized` pointee's unit-metadata equation and
omits the slice equation. The pinned Rust source
(`nightly-2026-08-15`, `library/core/src/ptr/metadata.rs`) specifies `usize`
metadata for slices and `str`. Candidate 369 adds those compiler-provided
associated-type equations alongside the typing repair. Own-branch rebuilding
and four slice/str encode-only controls pass; paired verification remains
pending. The minimum gate runs before its broader regression continuation 370.

Candidate 369 finishes at 5/7 checks on each backend. Its optional-slice
positive and exact negative now pass. The added `str` controls expose a separate
missing resource body: Rust strings contain bytes, but the generic conversion
currently treats them as unknown owned referents. Candidate 373 retains the
opaque physical storage predicate and adds a checked, typed empty indirect
resource body only for the compiler's `str` type. Unknown opaque types remain
opaque. The isolated own-branch build and all four encode-only controls pass;
paired proof and broader regression gates are separate.

The owned-tail minimum 358 fails on both backends at the final `*value == 7`
assertion, including the negative's preceding positive assertion. Its broader
checks are skipped. The generated `next`, `into_iter` and `push` restoration
wands return permissions without preserving referent state. Candidate 375 adds
explicit state transfer and expiry frames, based on the pinned Rust method
bodies, and lets an owned iterator's `[_]` selector identify its element group
rather than its allocator. It builds, but three encode controls reject a nested
`after_call` inside `before_expiry`; the replacement captures that observation
in a separate binding before reading its expiry state. No proof has run on this
specification change, and the original failed build/inputs remain recorded.

The focused follow-ups now pass for optional slice/str transfer (373: 7/7
checks per backend) and historical indexed-enum snapshots (364: 5/5 per
backend). Every selected negative reaches its exact assertion, and coverage
and provenance audits pass. The broader 374 and 364 regression continuations
remain required before adoption. Candidate 377 rebuilds the owned-tail and
referent-state contracts with the temporal capture fixed; all four controls
encode, and its paired minimum is running. Candidate 375 never entered the
verifier queue.

Canonical-sequence minimum 367 fails on both backends (3/4 each). Ferrum
reports the current-owner permission failure in the final wand package and
two array-helper observation postconditions; Silicon reaches the 600-second
timeout. Both exact frame negatives pass. The canonical recursive observer
specifies only length, whereas the removed duplicate observer also specified
its pointwise relation to element observations. Candidate 379 adds that
relation as a checked recursive function postcondition. It builds in isolation
and all three controls encode; its paired gate is queued. This change cannot
be assumed to repair the separate current-owner permission failure.

Raw diagnostic 372 locates the nested-reference failure in the second
`m_visit` restoration wand at Viper lines 839–945. Its indexed inner-pointer
reads remain inside the contract's quantifier because `array_indirect`
disables snapshot bindings below that binder; packaging freezes those reads
at `package_post0`. A checked sequence capture can freeze the pointer
identities outside the wand without capturing permissions. Model 381 tests
that proposal with a recursive sequence observer, later parent-pointer
replacement, and false/state/permission controls. It is queued and unverified;
no Rust encoder change for this diagnosis is accepted yet.

The separate loop-coupling candidate 354 finishes at 20/25 checks on each
backend. The library, extra-next positive/false and both wrong-invariant cases
still fail on missing indexed Vec entry resources. Its direct owned-reference
controls pass, as do the paired baseline controls, but the PCG change is not
adopted. The logical-tail candidate 358 remains separate and unqualified.

Helper stability remains unresolved: `inclusive_wrong_frame` emitted exactly
the same Viper bytes in builds 336 and 337, but Ferrum accepted only the intended
negative in 43.56 seconds in the first and added an aggregate-helper failure in
139.03 seconds in the second, under identical bounds. The program hash and
results are recorded in `inclusive-helper-stability337.json`. The failed run
is not replaced by the earlier pass, and the cause is not yet established.

The remaining generic borrow failures need a separate resource-handoff repair.
In `swap_view`, the body applies the swap's nested-resource wand and then the
`as_mut_slice` wands before return. This restores resources indexed by the
entry sequence, while the function's postcondition needs resources indexed by
the current sequence. `replace_index`, `replace_lookup`, and the abstract
`Borrowed` caller expose the same distinction between the current owner's
resources and the obligation to restore entry resources. Both backends report
missing permissions, so changing solver budgets cannot repair this failure.

An NLL experiment keeping opaque projections live until explicit destruction
only moved the early applications to `StorageDead`; it was discarded. A sound
handoff must retain the continuation from current resources to entry resources
while restoring ownership of the current collection. Simply returning both
groups, suppressing expiry applications, or keeping the returned element's
resources alone is insufficient: an indexed borrow also encapsulates the
untouched collection elements. Work on this repair must first establish that
composition in a small checked encoding, then cover native and cross-crate
swap/replacement, generic trait reborrows, and the existing iterator early-drop
and wrong-referent controls. Until that succeeds, these positives remain failed
matrix entries; they must not be trusted or restricted to reference-free types.

Relevant design background is Gorse's [Extended Support for Borrowing and
Lifetimes in Prusti](https://ethz.ch/content/dam/ethz/special-interest/infk/chair-program-method/pm/documents/Education/Theses/Lorenz_Gorse_MS_Report.pdf),
sections 3.6.7 and 4.1. It describes nested expiration tools and capturing parent
wand observations with let-bindings, and discusses the additional permission
enumeration problem for structures containing references. These are design
inputs for the handoff investigation, not evidence that the current encoding
or a proposed extension is correct.

### IM0: Reproduce and locate the missing information

Keep the existing positive and negative native iterator cases as controls.
Adapt [region_full_loop.rs](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/region_full_loop.rs)
to the bundled native iterator and compare its generated Viper with an
equivalent unrolled client. Use a scalar assignment first, then a non-`Copy`
owned element. Record exactly where the restoration equations stop surviving
the loop boundary; a failure with only a length invariant does not by itself
establish an encoder defect.

Add small, separate probes for slices containing `&U`, `&mut U`,
`Holder<'b, U> { value: &'b mut U }`, and `Option<&'b mut U>`. Start with
indexing and reborrowing without an iterator, then add one and two `next`
calls. Distinguish a missing contract, missing referent permission, incorrect
heap selection, and incorrect lifetime dependencies before changing code.
These probes should be valid Rust; the negative proof cases must reach
verification rather than fail Rust borrow checking.

Two inspected implementation boundaries guide the investigation:

- [coupled_wand](prusti-encoder/src/encoders/impure/wand.rs) constructs loop
  abstraction wands from resource predicates. This is the first candidate
  boundary for retaining a value relation between the processed prefix and
  eventual owner restoration; the reproducer must confirm the required change.
- [IndirectPredicatesEnc](prusti-encoder/src/encoders/ty/indirect.rs) recurses
  through concrete mutable references and fields, but currently returns no
  indirect resources for array-like, parameter, and opaque types. Audit these
  paths together with [array indexing and framing](prusti-encoder/src/encoders/ty/kinds/arraylike.rs).
  Absence of a modeled resource must not be interpreted as proof that a type
  contains no borrows.

**Completion evidence:** retain the minimal probes, emitted proof fragments,
expected failures, and a short resource ledger identifying which permission
and heap state each operation requires. These are diagnostics, not new support
claims.

### IM1: Prove a summary for the processed prefix

Use ghost state `S0` for the entry sequence, `N = |S0|`, a processed count `k`,
and a sequence `P` of the final snapshots of completed elements. Begin with
loops whose current yield expires before the next loop head. The desired
invariant has four parts:

1. `0 <= k <= N`, `|P| = k`, and the remaining tail has length `N - k`.
2. The untouched tail still has the entry snapshots `S0[k..]`.
3. `P` satisfies the required transformation, initially every element equals
   `7`, then a pointwise relation to its entry value with overflow preconditions.
4. A restoration obligation says that returning the tail's resources restores
   the source with contents `P ++ tail_at_expiry`.

The fourth part is a permission-bearing implication, or magic wand. Its value
equation is interpreted when the source becomes readable again. It must not
permit reading the borrowed source during the loop or equate mutable contents
across unrelated heap states. `P` initially contains only completed owned
values, not snapshots pretending to contain the values behind mutable references.

Reuse [spatial_iter.vpr](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/spatial_iter.vpr)
as existing proof evidence, then adapt the summary to the current whole-slice
owner and borrowed-tail representation. Establish the empty-prefix base case,
composition with one `next` expiry pledge, and both exits. Exhaustion gives
`source_final = P`; early exit after `k` updates gives
`source_final = P ++ S0[k..]`. Include an untrusted slice-backed Rust iterator
as a control for composing the existing expiry pledges.

**Completion evidence:** arbitrary symbolic lengths verify without unrolling
or assumed update laws. Wrong prefix values, an altered untouched suffix,
and `false` at exit are rejected. Record whether existing ghost state and
pledges can express the summary before choosing any new annotation syntax.

**Standalone evidence:** [prefix_wand.vpr](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/prefix_wand.vpr)
proves full and early-exit updates, unchanged suffixes, and pointwise bounded
increments using an abstract, permission-checked snapshot observation. The
snapshot axioms are stated explicitly; no update law is assumed. The
accompanying runner checks wrong-prefix, wrong-suffix, wrong-increment,
overflow, and consistency clients. The existing untrusted Rust iterator's
positive control verified in an isolated 129.10-second run; its wrong-value
control was rejected in 128.62 seconds. A prior concurrent positive run timed
out and is not counted as proof evidence. This gate does not establish that
the summary can yet be written or preserved as a Rust loop invariant.

### IM2: Carry the summary through Rust loop encoding

If existing specifications can express the IM1 summary, preserve those facts
through loop abstraction. If they cannot, add a narrowly scoped loop
restoration specification whose clauses become obligations when packaging
the loop wand. Settle its syntax only after the IM1 proof model works; do not
expose an unrestricted future-heap observation or simply assume the desired
postcondition.

Extend [loop encoding](prusti-encoder/src/encoders/impure/loop.rs), wand
construction, and package scripts to retain both resources and the summary's
value clauses. Check initialization on entry edges, composition on backedges,
and application on every exit. Preserve source identity and entry snapshots
without recapturing them on backedges. Bind tail and yielded-value observations
to the appropriate call/expiry heaps, including captures inside quantifiers.
Maintain the existing omission of empty resource obligations.

Initial expressibility probes found that an ordinary loop invariant emits an
invalid `old[lhs]` when it uses `before_expiry`; invalid contexts now receive
an explicit diagnostic. A saved source-reference identity also exposes an
unreached loop-head label and overlapping restoration resources. A separate
raw-pointer ghost identity now survives the loop and matches the restored
owner, without supplying permission to read its contents. The same loop
framing preserves unchanged ghost scalar and sequence snapshots. Finally, a
valid quantified heap-reading pledge with a direct dereference capture fails
permission checking, and its generated closure omits the requested `old[lhs]`
heap selection. For a closure that captures the mutable reference itself,
expiry heap selection is propagated into the quantified body; explicit-trigger
positive and negative clients exercise that narrower case. These reproducers
and their current diagnostics are recorded under
[IM2 known limits](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/README.md#im2-expressing-a-restoration-summary-in-a-loop).
Address them without granting access to a borrowed source or treating a
failed positive proof as evidence for a negative case.

The native iterator now has symbolic-length Rust controls for full traversal
and early break: at each loop head the remaining tail equals the corresponding
suffix of the entry sequence, and the exit count is checked. These controls
include empty inputs and reject incorrect exit-count claims. A separate
nonempty early-break case proves a particular untouched tail value and rejects
a wrong value. Those controls observe the live tail; the restoration tests
below separately prove the restored owner's prefix and suffix.

The constructor's initial source–tail value equality has also been checked
inside the actual emitted Viper package, using the generated slice predicates
and the existing constructor pledge. A deliberately opposite assertion fails.
This historical experiment checks only the base case; the loop annotation
below now carries the relation on backedges and exits.

A further experiment strengthens the actual emitted loop wand with a
processed-prefix and tail-at-expiry relation. The existing constructor and
`next` expiry pledges then verify full traversal for symbolic lengths, including
empty input; an incorrect final prefix and `false` are rejected. A second
variant explicitly maintains a ghost sequence of completed values and proves
`source_at_restoration = prefix ++ tail_at_expiry` with the same controls.
These were manual transformations of generated Viper. The Rust annotation
now emits the sequence relation on the selected loop wand. Rust clients prove
full traversal and early break for symbolic lengths; a generic unchanged-value
client uses element snapshots without a `Copy` bound. Rust loops also prove
quantified prefix values across append with either `usize` or `Int` indices.
Untrusted library functions export both full-traversal and early-break
contracts to separate clients, including the exact untouched suffix. Wrong-value,
wrong-suffix, and consistency clients are rejected. The remaining IM2 cases and
IM3–IM5 are still open.

Implement the Rust connection in these steps:

1. Introduce a narrowly scoped loop restoration annotation carrying a saved
   source identity, the live native iterator, and a ghost sequence of completed
   element snapshots. Its meaning is `source_at_restoration = prefix ++
   tail_at_expiry`; the annotation itself must never read the borrowed source.
   The syntax `body_restore_prefix!(&iterator, *owner, *prefix)` is recognized
   by loop-spec analysis. Its typed source–prefix–tail equation is encoded,
   and the iterator must have one matching singleton slice-restoration edge;
   duplicate annotations, missing edges, and overlapping edges are rejected.
   The equation binds source and tail addresses and slice lengths to that edge,
   with checks ordered before the slice observations.
2. Identify the unique PCG restoration edge at the loop head. Reject
   ambiguous or mismatched resources instead of guessing an edge or adding a
   second overlapping wand.
3. Attach the value relation to that edge's existing wand at entry, backedges,
   and exits. Capture source, tail, and prefix snapshots at the loop head, and
   use the saved pre-`next` iterator when applying the previous wand. The
   existing expiry pledges prove full traversal and symbolic early break in
   explicit Rust loops, including empty inputs. Resource-only loops retain
   their existing behavior. The first supported shape reaches the annotation
   before any iterator `next` call on the loop entry path. The named `for`
   macro supports this through `before_next { ... }`, where full-traversal
   restoration, quantified final values, and symbolic early-break suffix
   framing verify. An annotation in the
   ordinary body runs after a yield and is rejected until the current yield can
   be included in the restoration wand.
4. Complete the remaining Rust acceptance cases: conditional updates and
   ordinary `for` loops and direct `Vec::iter_mut`. Named `prusti_for!` loops
   with a `before_next` annotation, conditional updates with a `continue`
   backedge, two simultaneous yields per iteration, and an explicit
   `Vec::as_mut_slice` view now verify; wrong-value and `false` controls are
   rejected. Direct `Vec::iter_mut` currently has a PCG source edge for the
   vector rather than a slice and receives an unsupported-feature diagnostic.
   Keep wrong-prefix, wrong-suffix, `false`, borrowed owner, mismatched identity,
   and mismatched length checks alongside them.

For a named mutable-slice traversal, place the restoration annotation and
loop invariants before advancement:

```rust
prusti_for!(iterator, for element in &mut *values => before_next {
    body_restore_prefix!(&iterator, *owner, *completed);
    // Other invariants for the processed prefix and remaining tail.
} {
    // Update `element` and append its final snapshot to `completed`.
});
```

Add explicit-loop clients first, followed by ordinary mutable-slice `for`
loops and `Vec::iter_mut()`. Cover full traversal, symbolic early break,
conditional updates with `continue`, empty input, non-`Copy` owned elements,
and two yields within an iteration that both expire before the next head.
An unchanged-value claim after mutation and a claim that early break updated
the entire slice must fail for the intended reason.

**Exit gate:** a verified library exports exact final-value and suffix-framing
contracts for symbolic-length clients; separate positive and negative clients
consume them. No fixed size, loop unrolling, trusted traversal helper, or new
`Copy`, `Clone`, or `Eq` requirement substitutes for this proof. Existing
ownership-only loops continue to work with their current annotations.

### IM3: Track referent permissions inside concrete elements

The shared-reference single- and two-yield repros now verify, with incorrect-value
and `false` controls, including reads of the restored owner. The two-yield
function permits repeated referents; see the
[resource ledger](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/README.md#im0-native-loop-values-and-references-inside-elements).
This is a narrow preliminary result, not the IM3 exit gate.

Call-wand selection now uses a checked correspondence from generalized
signature projections to concrete call projections. It preserves multiple
signature origins when lifetimes collapse, and multiple concrete incarnations
when a signature projection expands. Unknown mappings remain distinct from
known empty mappings. Labelled edge places are matched back to the original
MIR arguments/result only when that match is unique. This is a partial
structural map, not unrestricted normalization support.

Each retained wand has an ID within its callee's obligations. Application and
loop-invariant retention select the same IDs, and phase-local deduplication
also preserves the call location and path condition. Resource anchors include
both definition-site and instantiated predicates; pledge-only wands retain
their shape anchors. Unknown correspondence or partial coverage is rejected,
rather than treating an opaque generic value as borrow-free. The diagnosed
nested shared lifetime has no corresponding emitted obligation and therefore
does not reapply the constructor's outer wand.

The call-side predicate builder still encodes a single generalized projection
index. Selection checks that this denotes the same signature resource after
substitution; shifted, merged, or expanded resource indices that the builder
cannot represent are rejected explicitly. `call_wand_shifted_projection`
covers a lifetime-free generic argument preceding a mutable reference.
Extending the mapping alone does not establish generic resource support.
This checkpoint's unsupported expectation is superseded by the positive and
wrong-value controls in the 2026-10-30 adoption follow-up below.

Validation on 2026-10-28: the full build passed; PCG passed four projection-map
tests and eight existing function-shape tests. Focused borrow and iterator
runs passed 18 and 19 checks respectively, including both specification
libraries, runtime clients, and expected diagnostics. Results are in
`target/stdlib-call-projection-verified-{borrow,iterator}-logs/results.json`.
The iterator library was also re-encoded after the final projection-consistency
rejection check (two additional checks), which does not change emitted wands.
The shared pledge-only fixture's Viper contains both `apply true --*` branches.
The two loop limitations below were reproduced on that build in
`target/stdlib-call-projection-known-logs` before the subsequent lifecycle fix.

Regression coverage includes two live shared yields, repeated referents,
independent mutable borrows expiring in either order, shared pledge-only wands,
and incorrect-value/false controls. The conditional-call loop repros now verify
and have moved from `known_limits` to the borrow suite as
`call_wand_branch_loop_*` and `call_wand_retained_branch_*`. Their stronger
versions use symbolic loop lengths, including zero iterations, both initial
branch choices, and alternating branches. Additional cases exercise
`continue`, early `break`, and an outer-loop borrow retained by an inner loop.

The loop invariant preserves each retained call obligation under the
disjunction of its contributing edge guards. It also frames the shallow
snapshot of an unchanged reference local with a readable tracked capability.
The PCG accessor includes restrictions imposed by live loans; initialized
storage alone does not establish readability. Assignments, moves, and mutable
borrows of the reference slot exclude this automatic frame, while writes
through its referent remain allowed. Positive and incorrect-value controls
cover direct reference reassignment, a helper borrowing the reference slot,
and a slot already loaned before entering the loop.

At the invariant boundary, branch flags strictly internal to the executable
loop body are reset for the next iteration. The dominance check follows the
encoder's specification-branch elimination; Rust's unmodified CFG contains
dead spec arms that can bypass the invariant block. Choices supporting a live
edge or the incoming state, choices preceding the invariant, and loop-exit
choices retain their history. This avoids both untaken call-label reads and
the loss of restoration permissions caused by globally clearing branch flags.
The change establishes the covered lifecycle patterns, not arbitrary
conditional-loop resource support.

Lifecycle validation on 2026-10-28: the full build, 34 focused borrow checks,
and 23 focused iterator checks passed, including both specification libraries,
runtime clients, and exact negative diagnostics. PCG passed four projection-map
tests, eight function-shape tests, and all 248 numbered corpus inputs. The
iterator checks cover whole-prefix traversal, early exit, `continue`, generic
elements, two yields, Vec views, and shared referents. Consolidated results and
build/PCG logs are in `target/stdlib-conditional-summary-logs`; individual
iterator logs are in `target/stdlib-conditional-regressions-iterator-logs`.
The first nested-loop test omitted the outer counter bound from its inner
invariant and failed overflow checking; the corrected positive and false
control both passed. The consolidated borrow results retain that superseded
failure and link each current result to its original log.

Further reference shapes can still require mappings that the structural
correspondence cannot establish. Extend it from explicit evidence through
normalization, never by equating projection indices by position. General
reference-containing resource support remains the next implementation step.

Keep reference identity and referent value separate. `Seq<&mut U>` and the
iterator identity observations continue to describe reference snapshots;
reading `U` requires the corresponding heap permission. Shared references may
alias, including repeated references to the same referent, and must not gain
exclusive permissions. Logical ownership must remain independent of physical
address distinctness for zero-sized types.

Extend lifetime-indexed indirect resources through slices and arrays. A
collection must carry the resources behind its element references as well as
the shallow element slots. Use an indexed resource family with finite bounds,
or an equivalent collection predicate with verified extraction/reassembly
rules. Decide the representation with the IM0 probes and generated Viper;
specify how the whole owner, an indexed element, a yielded element, and the
remaining tail divide those resources so they are never counted twice.

The concrete encoder now carries finite indexed resources through slices and
arrays. Slice bounds come from pointer metadata and fixed-array bounds from
constant lengths. Quantified conjunctions and variant guards are distributed
into atomic permissions before sending the AST to Silicon; Prusti does not use
Viper's parser, which normally performs this transformation. Index-dependent
heap reads remain under their binder instead of becoming wand-local captures.

PCG indexed lifetime expansions open and close wrappers, and array aggregate
edges transfer the resources of separately borrowed operands. Restoration reads
the actual held representation of an element: its typed or generic predicate,
its expanded struct fields, or the folded array owner. Historical heap labels
freeze both the snapshot and index. Permission inspection selects an existing
representation; missing resources fail an explicit assertion. Repeated
historical expansions therefore do not consume a wrapper twice or manufacture
permissions. Nested quantified extraction currently receives an explicit
unsupported diagnostic.

PCG revision `3134385` delays automatic expiry of an outer borrow while its
referent remains expanded for a field borrow. This is an ordering dependency,
not another resource consumed by edge removal. It fixes optional-element
restoration and passed all 249 numbered inputs, four projection-map tests,
eight function-shape tests, and the annotated nested-field expiry regression.

The `mut_element_{index,array,option,pair}_pass` clients now prove executable
reads, mutation, framing, and local-array borrow expiry. The pair has two
independent inner lifetimes. Four runtime clients, nine verification clients
(including five exact negative diagnostics), and the Rust compilation check
passed on 2026-10-28; results are in
`target/stdlib-im3-verified-iterator-logs/results.json`. The full build passed;
the iterator specification library was also independently reverified (170.79s).
These tests read referent values through ordinary permission-checked Rust
expressions into scalar locals. Pure dereferencing of a shallow reference
snapshot remains unsupported. Native mutable-reference iteration and the generic resource interface still
require work. The subsequent concrete extension below adds replacement and
nested references.

The concrete extension captures referent identities while their permissions
are available, preserves inactive-variant guards, and delays historical
resource closure until the corresponding physical dereferences have been
folded. Replacing an indexed reference transfers the replacement's resources
at assignment, including when there is no later read. A restoration proof
opens the incoming collection and returns the captured original referents.
PCG revision `dbb94c8` keeps argument lifetime resources live until return;
return restoration now requests their original lifetime roots as well as
argument storage. Conditional capture locals are declared in their actual
method or package scope.

Indexed predicates now include a structural field/dereference path as well
as the collection index. Distinct reference fields must retain distinct
logical resource instances even when their lifetimes merge or their addresses
cannot be proved different. Permission inspection selects an existing
representation; every alternative remains a checked fold, unfold, or
assertion. No new permissions are inhaled for these transitions.

`mut_element_nested_*`, `mut_element_pair_merged_*`, and
`mut_element_replace_*` cover nested mutation and framing, fields sharing one
lifetime, symbolic-slot replacement, immediate return after replacement, and
recovery of all local-array owners. On 2026-10-28, the full build and 34 concrete
checks passed, including the iterator specification library, nine runtime
clients, 23 verification clients with exact negative diagnostics, and Rust
compilation. Ten focused borrow regression checks also passed. Logs are in
`target/stdlib-im3-paths-full-iterator-logs` and
`target/stdlib-im3-paths-regression-borrow-logs`.

Current-element resources are now closed when the physical array/slice slot
is folded. Historical lifetime expansions may stay live across a loop, but
the current collection must be reassembled at its head and exit. The operation
reads the current element identity and uses checked folds, preserving the
separate historical restoration path for reference replacement.
`mut_element_loop_pass` verifies symbolic traversal, each immediate updated
read, and unchanged slot identities, with empty/nonempty runtime cases. Its
wrong-value and false-after-traversal controls fail at their intended assertions.
The full concrete run passed the 34 earlier checks plus the new positive
verification/runtime checks; a focused five-check rerun passed after correcting
the two new negative annotations to the verifier's exact diagnostic text.
Logs are in `target/stdlib-im3-loop-full-iterator-logs` and
`target/stdlib-im3-loop-controls-iterator-logs`.

This still does not close IM3: native mutable-reference yields and the
untrusted explicit-tail control require generic resource transport. The indexed
loop establishes permission restoration and shallow identities. The anchored
whole-prefix value proof described under IM5 now passes for direct indexing;
its generic/native-iterator counterparts remain open.

The executable [element-resource model](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/element_resources.vpr)
now validates indexed resource wrappers. A direct quantifier over referent
addresses failed Viper's injectivity check; adding pointer inequalities would
exclude valid zero-sized cases. Instead, `element(index, reference)` owns the
referent resource, and the index distinguishes predicate instances. The model
does not assert physical address distinctness. Its empty-resource case permits
the same reference at two indices. Shared aliases use a set of referent
identities, so repeated sequence occurrences do not multiply permissions.

The model verifies two simultaneous yields, resource/value restoration with a
symbolic remaining tail, reference replacement, nested referent framing, and a
symbolic loop that renumbers wrappers for a slice view. Indexed wrappers carry
a structural path as well as the slot index; sibling empty-resource instances
may share the same physical address without merging their logical ownership.
Replacement transfers the previous referent out while transferring the
replacement in. Seventeen negative controls reject false/stale/wrong-target
claims, access to the borrowed owner, duplicated mutable resources, writes with
shared permission, missing opening/closing/future resources, invalid saved
referent reads, and false claims after repeated views or empty resources.
Run `./x.py exec python3 scripts/test_stdlib_element_resources.py`; individual
logs are in `target/stdlib-im3-resources-logs`. On 2026-10-28, all 18 checks
passed with their exact expected diagnostics; the positive model verified in
13.06s. The same validation run passed all 34 concrete checks, ten borrow
checks, and fifteen native iterator regression checks. These checks complement
the concrete Rust tests above; they do not establish generic resource transport.

The representation follows Viper's documented
[quantified-permission injectivity requirement](https://viper.ethz.ch/tutorial/quantified-permissions-injectivity.html)
and [wand restoration mechanism](https://viper.ethz.ch/tutorial/magic-wands.html).
The concrete encoder uses indexed wrappers around the existing indirect
predicate applications, with explicit finite bounds and guarded extraction at
PCG lifetime-projection expansions. Wrapper bodies retain the original
resources; opening a wrapper does not assume a new referent permission.

Preserve the relationship between the short container/iterator borrow and
each inner borrow. Outlives constraints must not collapse distinct lifetimes
into equality. Reuse the existing heap labels and guarded enum-field handling;
inactive variants must not require referent permissions. Test writing through
an inner mutable reference separately from replacing or moving that reference
out of its slot, since those operations transfer different resources.

**Exit gate:** concrete slice clients read shared referents, mutate inner
mutable referents, preserve unrelated slots/referents, and recover their
owners after expiry. Cover repeated shared aliases, optional references,
multiple independent inner lifetimes, two simultaneous yields, early iterator
drop, and both valid expiry orders. Negative clients reject stale values,
claims that the wrong target changed, and heap reads through saved snapshots
without permission.
Keep the untrusted explicit-tail iterator as a control before relying on native
contracts.

### IM4: Preserve those resources through generic calls and Vec views

Implementation direction after concrete extraction/reassembly (not yet an
implemented interface):

1. Preserve an opaque type's resource group in the PCG body, corresponding to
   the signature's `RegionsIn(T)`. It must survive moves, field/array expansion,
   calls, and loop boundaries. Do not map it to a numbered concrete region or
   treat it as a borrow-free type. Check the existing region-only consumers,
   particularly borrow-checker queries, before extending their domain.
2. Represent that whole group by a snapshot/type-indexed predicate. Generic
   definitions transfer this predicate without inspecting its body. Concrete
   type cases describe the actual referent permissions; an unknown case must
   remain opaque. Collection instances also carry the logical element index
   and finite bounds, including when element snapshots compare equal.
3. Verify conversions between a concrete type's complete set of lifetime
   resources and that abstract group. Reference cases include both the shallow
   referent predicate and its recursive indirect resources; structs and active
   enum variants compose their fields. Collection conversions require verified
   symbolic extraction/reassembly, not extra assumed postconditions. A partial
   lifetime split cannot consume or duplicate a whole opaque group.
4. Verify the generic explicit-tail helper before adapting native `IterMut`
   and Vec views. Then instantiate the separately verified definition with
   owned/shared/mutable-reference elements, two-lifetime wrappers, and associated
   types. Preserve negative tests for missing resources and incomplete maps.

The PCG body prerequisite is implemented on the isolated
`stdlib/opaque-resource-groups` branch at `f610569` (pushed to
`git@github.com:LrWm3/pcg.git`). Opaque groups survive body projections and
moves, use compiler-derived type outlives bounds, and exclude function-item
and raw-pointer type arguments from owned resources. A short container borrow
does not become a lifetime bound on its generic element. Relabeling collapses
identity borrow flows. Validation passed all 250 numbered corpus files, four
projection tests, eight shape tests, and six body tests. The stable work branch keeps the submodule
at `dbb94c8` until its encoder supports the same group interface;
these PCG results alone do not close IM4.

The [generic resource model](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/generic_resources.vpr)
verifies whole groups for mutable references and pairs, a nonempty opaque
fallback, generic identity with a heap-dependent state frame, nested reference
replacement, and symbolic collection conversion in both directions. Each
conversion captures a value before unfolding and proves it unchanged after
folding; its loop invariant preserves the converted prefix and remaining suffix.
Equal owned values with empty resources may coexist. Nine negative controls
reject forged unknown groups, missing/duplicated referents, incomplete pairs,
observations/calls without permissions, wrong conversion values, and false
claims. All ten checks passed (positive 5.24s):
`./x.py exec python3 scripts/test_stdlib_generic_resources.py`, with logs in
`target/stdlib-im4-groups-logs`. This is the proof pattern for the pending Rust
encoder integration, not an implemented cross-crate interface.

A second [checked model](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/dependent_resources.vpr)
covers a dependency exposed by integrating opaque groups: the outer storage
can expire before the resources inside its generic element. Restore the
storage first; a later resource wand consumes and returns that storage as a
frame. Its postcondition must preserve the storage snapshot relative to
`old[lhs]`, since returning the same permission alone loses the value fact.
The model verifies replacement of the yielded value while retaining the old
value's separate resource. Five negative controls reject missing outer/inner
permissions, wrong replacement values, false after restoration, and loss of
the value frame. All six checks passed:
`./x.py exec python3 scripts/test_stdlib_dependent_resources.py`, logs in
`target/stdlib-im4-dependent-logs` (positive 43.77s during concurrent builds;
the standalone positive previously passed in 9.45s).

The Rust integration is in progress on `stdlib/opaque-resource-encoder`.
The full build and five focused generic-body checks pass: generic identity,
reborrowing, returning a slice head, Rust/runtime checks, and exact rejection
of false after extraction and a wrong generic identity postcondition. Logs
are in `target/stdlib-im4-generic-controls-iterator-logs` (positive 43.58s).
Outer storage restoration uses its physical graph root; opaque group actions
are deferred to the later package, which frames the restored storage and its
snapshot. Input resources are not returned twice when their outer storage
remains borrowed. The generic method interface stays at its definition-site
shape under substitution.

The development branch now implements checked concrete/group conversions for finite
mutable-reference, nested-reference, struct/tuple, and enum shapes at whole
parameter boundaries. Calls pack their input resources, unpack their returned
resources, and repeat the inverse conversions around restoration-wand
application. The operations fold/unfold actual predicate bodies; they do not
assume conversion postconditions. Definition-site resource expressions are
instantiated before inserting caller snapshots, avoiding accidental repeated
substitution of identically named type parameters.

`resource_state(&value)` is a specification-only, permission-checked observation
with a dedicated logical state datatype. Known finite shapes observe current
referent snapshots recursively; an unknown group retains an abstract,
permission-dependent state. Generic functions can explicitly promise state
preservation after a call and at borrow expiry. This is separate from shallow
snapshot equality: returning the same outer pointer does not by itself frame
references stored behind it. A nested-reference client now proves the initial
value survives a generic call, writes 7, preserves the stored reference, and
exports the final value 7. Generic identity also composes through a second
separately verified generic body.
All 22 focused checks pass: six positive verifications and runtime executions,
nine exact negative diagnostics, and a Rust check. The controls cover false
claims, wrong values, a mutation violating a state frame, missing permissions,
and executable misuse of the observer. Logs are in
`target/stdlib-im4-finite-iterator-logs`; the full build also passed.

`ResourceState` also supports direct `==` and `!=` in specifications. The
encoder compares logical states, including observed referent values. A
focused positive client compares equal states, changes one referent, and
then proves the states unequal; a negative client rejects equality between
referents containing 1 and 2. Verification, runtime execution, and the Rust
check pass all four checks (`--case 'mut_resource_state_eq_*'`, logs in
`target/stdlib-iterator-spec-logs`). This does not by itself settle symbolic
collection expiry framing.

The separate `stdlib_resources_spec` fixture now verifies a `no_std`
`resource_specs` library containing generic identity, a twice-composed
identity, and identity over a trait's associated type. Clients use its exported snapshot, resource-state, and expiry
contracts for mutable references, nested mutable references, a public wrapper
with two independent lifetimes, a non-`Copy` owned struct, and shared references.
All five positive clients verify and run. Five negative controls reject a
wrong restored value, a wrong nested value, and `false` after returning to the
owner. Library verification, the Rust check, runtime executions, and these
clients make 17 checks. Run them with
`./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite resources`;
logs are in `target/stdlib-resources-spec-logs`. This establishes the finite
whole-parameter interface across crate boundaries, including an associated
type instantiated with a mutable reference. Parameters nested in argument
constructors and collection views remain open.

Generic calls also unpack whole-parameter input resource groups that their
contracts return immediately, rather than through an expiry wand. The inverse
conversion uses the input's pre-call snapshot because its owned storage has
been consumed. This repairs `drop` and generic discard helpers for reference
wrappers: the owner can use its referents again. A two-lifetime wrapper client,
its runtime execution, and a `false` control pass all four focused checks
(`--case 'mut_generic_discard*'`). The complete `after_call*` regression subset
also passes all ten checks, including captured values after a move or mutation
and wrong-value/consistency controls. This returns permissions; it does not
invent a value-preservation contract for arbitrary generic destructors.

Array literals now retain their complete generic element snapshots through
the existing sequence constructor. Equating only the concrete projection had
lost the element type tag needed to justify a collection resource group.
The concrete array/index/replacement regression subset passes all ten checks,
including wrong-value and consistency controls. The finite-reference subset
also continues to pass all 22 checks. Symbolic collection conversion and its
value framing remain separate acceptance work; literal construction alone
does not supply the typing facts for an arbitrary incoming array.

The array boundary now has checked recursive pack/unpack methods for finite
element shapes. A checked typing traversal establishes the concrete element
representation from an incoming physical array; the resource conversions
preserve an aggregate sequence of referent states and expose individual
observations for indexed clients. These helper proofs use a symbolic sequence
length. Three literal-array clients verify direct use, a generic identity call,
and state/value preservation through that call and expiry. Four negative
controls reject `false`, a wrong element value, and a wrong restored value at
the intended assertion. Together with runtime executions and the Rust check,
all 11 checks pass with the existing pledge encoding:
`--suite iterator --skip-library --case 'mut_generic_array*' --timeout 600`.
The slowest case took 237 seconds; these checks need the longer limit.
Logs are in `target/stdlib-array38-iterator-spec-logs` (the final baseline run
used build 41; the directory name is retained from the initial experiment).

A symbolic-length caller now proves its aggregate call and expiry frames for
`[&mut u32; N]` through generic identity. The recursive sequence observers are
opaque to client proofs and explicitly revealed inside the checked conversion
proofs. Their bodies and postconditions are still verified; this change adds no
assumed preservation facts. The raw observer's pointwise contract is triggered
by an index into its result, avoiding recursive heap-observation triggers.
The [symbolic client](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/src/bin/mut_generic_array_symbolic_pass.rs)
passes native verification (55.20s), and its `false` and unequal-expiry-frame
controls reject the intended assertion and pledge. The trusted executable
entry point is only a runtime exercise; the symbolic relay and generic identity
are verified without trust. The earlier pledge simplification experiments
remain removed. Arrays of reference-bearing structs still expose a conversion
state-proof gap; nested collections and Vec ownership also remain open.
The full array subset passes all 15 checks with a 300-second program limit
(`target/stdlib-im4-opacity-iterator-spec-logs`), including all earlier literal
array controls. The 17 cross-crate finite-resource checks also pass with this
observer encoding (`target/stdlib-im4-opacity-resources-spec-logs`).

Explicit pledge placement is now implemented. A function
can restore its outer input storage before references inside its element expire;
placing every pledge on the last stage loses the earlier storage relation.
Use `after_expiry(input => condition)` and
`assert_on_expiry(input => obligation, condition)` to select the stage that
restores the named input's outer borrowed lifetime. Dereferencing the selector
chooses a nested type, for example `*self` for the borrowed tail inside a
temporary borrow of `IterMut`. A non-reference selected type must identify a
unique contained borrowed lifetime (or a single opaque resource group).
The encoder resolves it in the definition's signature,
then match the original input projection by lifetime, since repeated lifetimes
are deduplicated. Omitted selectors and `result` remain restricted to an
unambiguous stage; missing or ambiguous sources are rejected. Each pledge is
attached to exactly one restoration stage. Eighteen focused checks cover
independent expiry value frames, generic outer-storage framing, nested and
repeated-lifetime selection, conditional obligations, and exact diagnostics.
They include a nested read that still fails for missing permission: selecting
a stage does not supply permission for a possibly replaced inner reference.
The 37 specification-parser tests pass, as do all 20 cross-crate resource checks,
including exported independent-expiry contracts and a false control. A failed
conditional obligation now reports `obligation might not hold on borrow expiry`
at the caller instead of an internal error. Native iterator integration remains
in progress, along with constructor conversions and the Vec resource model.

Generic resource conversion now follows the definition's type constructors.
At a call to `f<T>(&mut T)`, `f<T>(Wrapper<T>)`, or `f<T>(Option<T>)`, it converts
only the opaque `T` group and retains the surrounding physical storage.
Array and slice boundaries recover element type tags with checked recursive
helpers; slices pass their runtime extent. Reference metadata is preserved,
and conversion at expiry uses the appropriate pre-call or post-call heap.
Retained iterator-tail resources are returned before their element predicates,
so those predicates have permission to read the tail snapshot. A resource-free
instantiation may omit applying an empty, pledge-free restoration stage only
when its conversion explicitly establishes an empty resource shape.

Six local constructor clients (three positive and three false controls), their
three runtime exercises, and the Rust check pass. A separate `no_std` library
exports `identity_container<T>`; its body, caller update/restoration proof,
false control, runtime exercise, and Rust check all pass (five checks).
Logs are in `target/stdlib-im4-constructor58-{iterator,resources}-spec-logs`;
the corrected Option positive and its false control were rerun separately.
The seven native scalar early-drop/full-prefix checks also pass with the new
constructor conversion (`target/stdlib-im4-native56-iterator-spec-logs`).
The original two-yield mutable-reference write probe now verifies. It is
promoted to `mut_native_mut_ref_write_pass`, with a false control that fails at
the intended assertion. The Rust check and runtime exercise also pass (four
checks, 90.41s for the positive and 143.11s for the false control, with
`--proverEnableTimeBounds` and a 300-second program limit). Stronger assertions
that reread the restored slice exposed a PCG future-projection conflict; a
plain-Rust reproducer isolates it before Viper. PCG now excludes the inaccessible retained inputs from the call's restoration
targets, matching the verifier's visibility rule. This fixes the crash;
the numbered PCG corpus (98.09s), eight shape tests, and four projection-map
tests pass. PCG commit `50e6a46` is pushed to the configured fork.
The stronger mutable-reference and holder tests now reach Viper and fail their
final-value assertions. They remain explicit known-limit probes, including
false/wrong controls that are not counted as successful negative proofs.
This checkpoint establishes the writes and the graph repair, not final
referent-value framing.

The symbolic-array positive still passes with the original solver resource
bounds (126.49s in the concurrent build-58 run). The experimental
`--proverEnableTimeBounds` setting rejects its package after 106.16s even when
the emitted program is identical to the earlier passing checkpoint. This is a
configuration-sensitive proof limitation, not evidence of a constructor
encoding regression or a Viper bug. Keep the original array settings while
continuing to improve proof stability. All five symbolic-array checks pass
with the original settings (`target/stdlib-im4-array58-default-iterator-spec-logs`).

This is a development checkpoint, not an IM4 acceptance result. Symbolic
collection conversion for further element shapes, generic parameters nested
inside collection element constructors, collection state transport, native
reference-element restoration, Vec ownership, complete diagnostics for
unsupported frames, and full regression validation remain required. The PCG pointer on this development
branch is `50e6a46`; the stable work branch retains the earlier pointer until
the combined encoder passes its gates.

The checked `sequence_resource_state(&sequence, index)` observer now reads an
element's referent resources through its saved shallow snapshot. It requires
both a valid index and permission to those resources; the saved sequence alone
grants no heap access. The positive observes a write through a saved sequence;
the false, stale-state, negative-index, upper-bound, missing-permission, and
executable-use controls produce their intended diagnostics. All seven verifier
cases, the Rust check, and the runtime exercise pass in the build-65 run
(`target/stdlib-im4-observation65-iterator-spec-logs`). Observer declarations
are independent of conversion helpers, so an observation does not introduce
unneeded generic pack/unpack obligations. The existing symbolic-array positive
also passes with its original solver bounds (128.37s).

Expiry selectors now accept `input[_]` (and dereferenced inputs) for a unique
opaque resource group. The selector resolves the definition's group identity,
independently of concrete lifetimes that also occur in the container. A generic
identity function checks its group-state expiry pledge, and its mutable-reference
caller proves the final write. Its false control and the missing/ambiguous-group
diagnostics pass. The combined selector/observer run passes all 14 checks;
all 37 specification-parser unit tests pass as well. This selector grants no
additional permissions and does not supply a preservation relation by itself.

An untrusted generic slice reborrow now verifies a universally quantified
per-element group-state expiry pledge. Its false pledge is rejected, and the
Rust check and runtime exercise pass (four checks, build 67,
`target/stdlib-im4-move67-iterator-spec-logs`). `forall` and `exists` now accept
`move` captures, including their explicit triggers; all 38 parser tests pass.
Using value captures for the saved sequences avoids the Java stack overflow
seen with the expanded reference captures in the concrete reborrow probe.
The concrete caller still fails its final-value assertion: collection
conversion currently exports aggregate preservation without a usable checked
relation for each opaque element's state. Experimental stronger conversion
postconditions have not passed and are not an accepted frame.

The next value-frame step connects the checked element observations across
collection conversions. Then state the native constructor and `next`
resource frames at that group stage, using saved pre-call and post-call
sequences. Verify contract well-formedness and the concrete two-yield/holder
restoration probes before extending the prefix-loop summary. Keep the physical
slot identity relation and heap-dependent referent state as separate checked
obligations. No frame may simply be assumed for a verified helper.

The destructor control exposed a separate trait-refinement problem with nested
mutable resources: the checker havoced an outer resource before consuming an
inner permission addressed through its old snapshot. The refinement check now
consumes the complete old resource conjunction before inhaling the new one.
The control's destructor body verifies without trust and its stale-value claim
is rejected. The existing mutable-frame positive and wrong-postcondition/state
controls pass. Two other cases in that test run still fail on the preceding
build as well: generic iterator conversion and `replace_owned` restoration.
Logs: `target/stdlib-im4-refinement69-extern-spec-logs`,
`target/stdlib-im4-refinement68-baseline-extern-spec-logs`, and
`target/stdlib-im4-drop69b-iterator-spec-logs`.

The scalar suffix regression is fixed. The generic `mem::drop` contract returned
the iterator's tail permissions but forgot its values. When rustc proves that
the argument needs no drop glue, the encoder now consumes only its owned value
and keeps the borrowed heap unchanged. Unknown types and types with destructors
still use the ordinary call path. The reference-bearing wrapper positive, false
control, destructor stale-value control, and unknown-type frame control pass
(six checks, `target/stdlib-im4-drop69-iterator-spec-logs`; the untrusted
destructor rerun is in `target/stdlib-im4-drop69b-iterator-spec-logs`).
The symbolic early-break positive, runtime exercise, and wrong-suffix control
pass as well (`target/stdlib-im4-drop68-iterator-spec-logs`). All three scalar
iterator library bodies verify with the original solver bounds. The full
library run now reports only the six generic shared/Vec conversion failures
(`/tmp/prusti-im4-library68-full.log`); these remain an open regression gate.

Directly restored generic inputs now unpack their resource groups using the
pre-call interior snapshots promised by the method contract. Retained input
resources still use the post-call heap, as required for an iterator's remaining
tail. This fixes both shared-iterator library failures and the existing
`refinement_mutable_pass` client (9.60s). The full build-70 iterator library run
has four remaining failures, all at owned Vec iterator conversions; its scalar
and shared iterator bodies pass. Logs: `/tmp/prusti-im4-library70-full.log` and
`target/stdlib-im4-refinement70-extern-spec-logs`. The independent
`replace_owned` restoration diagnostic remains open.

Output conversion now follows the opaque groups actually exported by the
definition-site contract. A structural walk had additionally tried to unpack
an allocator group hidden inside `IntoIter`'s `MaybeDangling` field. The existing
owned iterator positive and three false controls now pass (six checks), as do
the constructor/reference/identity controls (13 checks), in
`target/stdlib-im4-owned72-iterator-spec-logs` and
`target/stdlib-im4-output72-iterator-spec-logs`. The full library advances past
all four conversion failures, exposing three loop invariants with an
unavailable old-state label and one missing loop wand. These are still open;
the successful scalar/shared cases do not establish generic Vec element
ownership or complete the library regression gate.

The expression simplifier now retains a checked function reveal at its original
evaluation point, including when its value is unused or used only in one
conditional branch. Moving the reveal into that branch had removed the
inductive proof fact needed by the specialized collection-state observer.
All 14 simplifier unit tests pass, including four reveal-placement controls
(`/tmp/prusti-im4-simplify73.log`).

Generic `replace_owned` now verifies as well. Sources with no resource-bearing
output dependency return directly when there is no expiry pledge. Packaging a
restoration for several inputs also shares the consumed-call set across their
unblocking graphs, so a single call wand is neither required nor applied twice.
The core-only crate, existing mutable trait positive, and both wrong replace
postconditions pass (seven checks,
`target/stdlib-im4-refinement77-extern-spec-logs`).

The full iterator library regression gate now passes. Resource-only
obligations of the form `P --* true` are omitted at loop boundaries, where P
could mention a label unavailable on the first entry; applying such an
obligation still checks and consumes P. Packaging it needs no stored wand.
The independent build-78 run excludes the experimental collection-state
changes and verifies the full library in 5m11s
(`/tmp/prusti-im4-library78-full.log`). All four selected loop false/wrong
controls produce exactly their intended diagnostics (five checks including
Rust checking, `target/stdlib-im4-loop78-default-iterator-spec-logs`). This
closes those regression failures, not the remaining native reference-element
value and combined prefix-loop gates.

The indexed reborrow now proves the final referent value across the complete
pack → generic expiry pledge → unpack path. Generic contracts and concrete
conversions use the same checked per-element group observer; specialized
aggregate observers export checked pointwise relations without expanding the
global recursive observer in client proofs. The untrusted reborrow positive
and wrong-value control, Rust check, and runtime exercise pass (four checks,
`target/stdlib-im4-reborrow79-time30-iterator-spec-logs`). This run uses a
600-second program limit, 30-second solver assertions, and time bounds. The
positive also passes with the original resource bounds (build 76, 136.61s),
but the negative exceeded its 300-second program limit under those bounds.
Reproduce the accepted configuration with:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator \
  --case 'mut_sequence_state_reborrow_*' --skip-library --timeout 600 \
  --solver-timeout-ms 30000 --solver-time-bounds
```

Every conversion and observer body remains checked. This is an indexed
checkpoint: a universally quantified concrete caller still overflows the
verifier stack or exceeds its time limit, and the experimental whole-sequence
observer did not resolve that failure. Native iterator resource frames and
combined prefix loops remain open.

The first native resource-frame probe exposed an encoder performance problem
before Viper started. A debugger sample showed the debug binding checker
recursively revisiting shared expression nodes. It now caches visits by node
and the types of its free variables, and restores shadowed bindings after lets
and quantifiers. A cache keyed by every lexical scope still repeated work under
irrelevant temporary bindings; the refined cache avoids that repetition.
Free-variable summaries are retained across overlapping constructor checks
and cleared with the VIR arena. All 36 VIR unit tests pass, including shared
graphs with and without enclosing binders and controls that retain type-error
detection across scopes and separate checks
(`/tmp/prusti-im4-bindings84-tests.log`). The iterator frame itself remains
experimental until its native positive and negative gates pass.

All nine existing sequence-observer checks pass with the indexed transport
changes: Rust checking, runtime execution, correct observation, false and
stale-value controls, both index bounds, missing permission, and executable-use
rejection (`target/stdlib-im4-observer82-iterator-spec-logs`). This regression
run uses the original solver resource bounds.

Repeated `after_call` reads now share one captured snapshot within a wand when
the expression, arguments, and temporal context are identical. This avoids
hundreds of duplicate capture bindings in the native contract experiment.
All ten existing `after_call` checks pass, including moving the owner, false
and wrong pledges, and invalid/nested temporal uses
(`target/stdlib-im4-capture86-borrow-spec-logs`). The debug encoder now reaches
the JVM on the native probe, but its expanded expression is still too large;
this checkpoint does not establish the native restoration value proof.

A restricted constructor/projection normalization now runs uniformly over
wand declarations, packages, applications, and quantifier triggers. It only
cancels explicit, type-matched snapshot construction followed by field
projection; it preserves bindings, temporal labels, permission order, and
checked reveals. The native holder experiment's raw VIR shrinks from
31,534,470 to 525,982 bytes, allowing the JVM to construct and verify it.
All 38 VIR tests and all ten existing `after_call` checks pass
(`/tmp/prusti-im4-projections89-tests.log`,
`target/stdlib-im4-projections89-borrow-spec-logs`).

This is a performance checkpoint, not native frame acceptance. The native
probe now reports `apply.failed:wand.not.found`: comparison of Silver's wand
structures identifies an entry-heap `old` beneath temporary bindings in the
head pledge, retained as `old` in the callee but `old[pre2]` in the caller.
The universal reborrow reaches an ordinary final-value assertion failure
instead of exhausting AST construction. A scalar loop regression run also
finds a generated conversion helper's type-preservation failure under
30-second solver time bounds; that helper's body is identical with and without
the new normalization. The scalar `false` control still rejects only its
intended assertion. Temporary expansion, conversion proof stability, native
resource frames, and the remaining IM gates are still open.

The conversion helper's pure type-preservation quantifiers now trigger on the
canonical sequence lookup. Inferred constructor/cast triggers were unstable
under snapshot normalization. The isolated failing scalar unpack helper now
verifies in 30.43 seconds; the same old program without the explicit trigger
failed after 3m08s. Native Rust full-traversal, false, and wrong-value loop cases
all pass their expected checks (five checks including Rust and runtime,
`target/stdlib-im4-trigger92-iterator-spec-logs`, 600-second program limit,
30-second assertions with time bounds). No helper contract or resource
assumption changes in this checkpoint.

Expiry pledges now expand their temporary snapshots before selecting temporal
heaps, including pledges without a quantifier. This removes the differing
`old`/`old[pre2]` structure in the native head wand. An untrusted slice reborrow
with an old heap observation through a temporary verifies, its reachable
`false` client is rejected, and its wrong pledge is rejected at the pledge
itself. The ten prior temporal-capture checks also pass. Logs are in
`target/stdlib-im4-expiry92-borrow-spec-logs`; the wrong-pledge test's annotation
was moved to the diagnostic's opening line and its rerun passes in
`target/stdlib-im4-expiry92-span-borrow-spec-logs`. The native experiment now
advances to a resource-observer permission failure. A checked diagnostic
assertion isolates missing equality between a sized yielded reference's
metadata and unit metadata; the encoded `Pointee::Metadata` type function
currently lacks that compiler-defined equation.

The `Pointee` trait encoding now includes Rust's compiler-defined equation
`T: Sized => <T as Pointee>::Metadata = ()`. Ordinary impl-item encoding cannot
supply it because `Pointee` has a compiler implementation. The equation is
guarded by `Sized`; it adds no referent permissions or heap-state equality and
does not constrain unsized metadata to unit. The native holder probe passes
the previous permission check and reaches its final value assertion, which
still fails without explicit frame triggers. The resource library and selected
cross-crate mutable, associated-type, owned/shared, false, and wrong-value
clients all pass (12 checks,
`target/stdlib-im4-metadata94-resources-spec-logs`, 600-second program limit,
30-second assertions with time bounds).

A call-site wand whose opaque sources all instantiate to checked empty
resource groups may now discard extra expiry postconditions. This weakens the
facts available to the caller. It must still retain or reject any
`assert_on_expiry` obligation; an empty projection mapping alone does not
justify discarding a wand. The scalar reborrow proves its updated value and
rejects a reachable `false`, and the existing conditional obligation positive
and wrong-value controls pass. A whole-value generic obligation instantiated
with `u32` is conservatively rejected at its expiry. Logs are in
`target/stdlib-im4-empty96-iterator-spec-logs` (build 97) and
`target/stdlib-im4-pledges98-iterator-spec-logs`; they use the recorded
600-second program limit and 30-second assertions with time bounds. The scalar
positive takes 308.22s and its false control 150.35s. An initial slice-based
obligation diagnostic also reported a generated unpack helper's type-preservation
failure; it is not counted as a successful negative proof. The smaller
whole-value diagnostic isolates obligation retention and passes in 3.95s.

Explicit quantifier triggers now expand their capture projections before
snapshot normalization, while retaining capture evaluation outside the
quantifier. Previously a trigger went through a tuple local while the body
used the captured sequence directly. Silicon could not associate the
heap-dependent trigger with the function evaluation in the quantified body.
All 12 canonical element-state triggers in the native holder experiment now
match their body expressions. The 10 sequence trigger checks and 14 quantified
borrow checks pass, as do the untrusted generic sequence-state expiry positive,
wrong pledge, Rust check, and runtime exercise after adding explicit triggers.
Logs: `target/stdlib-im4-triggers97-sequence-spec-logs`,
`target/stdlib-im4-triggers97-borrow-spec-logs`, and
`target/stdlib-im4-pledges98-iterator-spec-logs` (same solver configuration).

Native reference-value restoration remains open. The holder caller alone now
fails only its final value assertion (3m15s), and its generated conversion
helpers verify separately (2m28s), using the pinned backend and the experimental
contracts in `/tmp/prusti-im4-native95-contracts`. The full native run reports
a generated observer permission failure; neither isolated result replaces the
full gate. The universally quantified concrete wrong-value caller also exceeds
600 seconds on build 94. These are unresolved proof/solver-encoding cases,
not successful negative controls or accepted native frames.

The universally quantified concrete reborrow gate now passes. Its frame
triggers on a saved sequence element lookup. Trigger encoding unwraps the
logical value of a `Ghost<T>` result: matching the synthetic Ghost constructor
prevented ordinary element accesses from instantiating the frame. The complete
untrusted generic definition and concrete caller prove the updated referent at
an arbitrary in-bounds index (122.73s); the wrong-value control is rejected at
its assertion (215.53s). Rust checking and runtime execution also pass, using
build 105 and the same 600-second/30-second time bounds. Reproduce with
`--suite iterator --skip-library --case 'mut_sequence_state_forall_reborrow_*'`
and the solver options above. Logs:
`target/stdlib-im4-universal105-iterator-spec-logs`.

The complete sequence suite passes 20 of 21 checks with that change. Its
`observation_pass` case loses an owned element's value after an array-to-slice
observation; the same failure reproduces on build 97, before Ghost trigger
unwrapping (`target/stdlib-im4-sequence105-sequence-spec-logs` and
`target/stdlib-im4-sequence97-baseline-sequence-spec-logs`). This remains a
regression gate. The native holder caller also verifies in isolation when the
experimental frame uses lookup triggers, but the full native run still reports
a generated conversion helper's observer-permission failure. Native contracts
and combined prefix loops are not accepted at this checkpoint.

The array-to-slice observation regression is repaired. Its value-preservation
axiom now matches the native array and slice lookups, without the executable
resource-observation marker. This changes instantiation, not the equality
asserted by the axiom. The previously failing observation and all three
sequence negative controls pass, together with Rust/runtime checking (six
checks in `target/stdlib-im4-unsize106-sequence-spec-logs`). Array replacement
and its three negative controls also pass. The separate
`mut_element_array_pass` fails on its first referent read on both builds 105
and 106, so that pre-existing indexed-resource regression remains open;
logs are `target/stdlib-im4-unsize106-iterator-spec-logs` and
`target/stdlib-im4-array108-baseline-iterator-spec-logs`.

The native holder experiment now has one successful complete verification
(215.58s, build 106, `target/stdlib-im4-native107-iterator-spec-logs`), including
the generated conversion helpers. Earlier equivalent runs still fail inside
those helpers, and the two-reference positive remains unresolved. This single
successful run does not yet establish a reliable native acceptance gate.

The indexed reference-array regression is repaired as a separate checkpoint.
Its quantified permissions now match both the native array lookup used by
executable indexing and the canonical sequence lookup used by specifications.
A sequence mentioned only beneath a quantifier did not provide the ground
term needed to activate the previous lookup bridge. Replaying the failed SMT
query reports `incomplete quantifiers`, not a timeout. The change adds no
permission or value assumptions. Fixed-length reads/updates, reference
replacement, and a new const-generic arbitrary-index update/framing pair all
pass: 11 checks in `target/stdlib-im4-array115-iterator-spec-logs` (build 115,
600-second program / 30-second assertion time bounds, fresh targets).

Explicit lookup triggers in the generated conversion helpers also let the
native holder pass on build 114 (156.73s). They do not yet close the native
reference gate: the two-yield reference caller still reports helper permission
and aggregate-state failures. The helper-trigger change remains experimental
pending that gate, and native contracts remain outside the accepted suite.

The full sequence regression gate now passes all 22 checks, including the
core specification library (`target/stdlib-im4-sequence115-sequence-spec-logs`).
The extra native lookup trigger on indexed permissions is restricted to fixed
arrays. Slice restoration wands use only the canonical sequence lookup: a raw
slice trigger captured a physical slice snapshot even when the wand's contract
related equal contents from distinct snapshots. Removing that extra slice
trigger repairs native wand matching without weakening its permission body.
The full holder and two-reference restoration positives, four exact-diagnostic
negative controls, and empty/exhausted iterator cases pass with build 123 and
experimental native contracts (logs in
`target/stdlib-im4-{native,negative,boundary}123-iterator-spec-logs`). These runs
also include the pointwise conversion-helper experiment described below;
this is not yet acceptance of all native resource frames. Initial/untouched
referent framing and preservation of the older aggregate-array gates remain
under investigation before promotion of the native contracts.

Fixed arrays also require restoration to depend only on their canonical
sequence. Their native lookup trigger caused a symbolic-array identity pledge
to fail with `wand.not.found`. Indexed resources now retain the existing
array-to-sequence length equation outside the quantifier. This provides the
ground sequence term needed for the first native element read, while all
resource triggers use canonical sequence lookups. The equation follows from
the existing representation axiom; it grants no additional permission.
Fixed-array and const-generic arbitrary-index positives and their exact
wrong-value controls pass all seven checks
(`target/stdlib-im4-array134-iterator-spec-logs`). The full symbolic generic
array identity/state/expiry positive also passes with the original resource
bounds (271.75s, `target/stdlib-im4-aggregate136-iterator-spec-logs`), repairing
the wand mismatch. These runs include the concurrent metadata normalization
experiment; complete whole-array regression validation is still pending.

Mutable-reference resource keys now normalize unit pointer metadata by its
resolved type. This applies both to a concrete use and inside the generic
resource predicate; non-unit metadata, including a slice's length, keeps its
original value. Normalizing only the concrete snapshot cast made generic
return observations well-formed but broke concrete resource folds. The
consistent normalization verifies untrusted direct and `Option`-wrapped
generic returns with referent-state preservation, and rejects the unequal
state control at its postcondition (four checks,
`target/stdlib-im4-metadata136-iterator-spec-logs`). The full native reference
framing positive also passes (203.57s), including initial values, an updated
first referent, and an untouched second referent after iterator expiry; its
wrong-state assertion is rejected as intended (112.02s). Rust/runtime checks
bring that experimental native framing gate to four checks
(`target/stdlib-im4-native136-iterator-spec-logs`). The regular suite and library
validation of the promoted native contracts are still in progress.

Collection conversion now has two checked proof modes. Lifetime-resource
boundaries use pointwise state preservation and canonical lookup triggers;
whole-value array conversions keep the aggregate observer proof. Pointwise
conversion recursively folds/unfolds each actual resource and proves the
observed state unchanged. It does not ask the solver to equate unused whole
heap maps. The aggregate mode retains the earlier direct sequence equality
proof rather than also transporting a second pointwise observer contract.
Every generated observer and conversion body remains verified.

Both native restoration positives and all four exact-diagnostic controls
passed with the pointwise proof (build 123); the universal reborrow positive
and false control also pass all four checks
(`target/stdlib-im4-universal123-iterator-spec-logs`). With the separate aggregate
proof in build 137, direct arrays, generic identity, and initial/final
array-state restoration pass using the original resource bounds (89.66s,
567.86s, and 479.81s respectively,
`target/stdlib-im4-aggregate137-iterator-spec-logs`). These timings retain a
proof-performance concern; the remaining paired array checks are in progress.
The complete cross-crate resource suite passes all 23 checks, including its
library, associated types, constructors, nested references, and wrong-state
controls (`target/stdlib-im4-resources138-resources-spec-logs`).

Native resource pledges exposed an overly conservative empty-group check for
owned structs and zero-sized tuples. The check now follows finite struct and
enum fields recursively, while keeping arrays, unknown types, and raw-pointer
representations conservative. It does not classify Vec/RawBuf ownership as
empty merely because pointer and PhantomData fields carry no explicit referent
permission. Caller expiry obligations are still retained: the new owned-struct
`assert_on_expiry` control receives the intended unsupported diagnostic rather
than silently dropping its false obligation.

The previously rejected owned-token update/restoration positive and its
wrong-value control pass (188.33s and 207.33s), as do zero-sized iteration,
wrong physical-address distinctness, and false-after-traversal controls
(`target/stdlib-im4-owned141-iterator-spec-logs`). The full iterator library also
passes with its original resource-bound configuration (545.21s, build 141,
`target/stdlib-im4-library144-iterator-spec-logs`). A 30-second time-bound run
failed a shared-iterator postcondition that passes in isolation; the original
settings remain required for this library gate. The full symbolic-array
positive likewise passes under resource bounds (341.07s, build 137) but fails
under time bounds; this proof-configuration limitation remains documented.

The native resource contracts are now in the bundled slice specification.
The regular fixture verifies holder and two-reference restoration, empty and
exhausted iteration, and initial/untouched referent framing, with all eight
exact-diagnostic false/wrong-state controls passing (build 137,
`target/stdlib-im4-native137-iterator-spec-logs`). Mutable-slice `IntoIterator`
initially failed its second final-value assertion in that batch; a fresh full
run passes in 289.59s on build 141, including every generated conversion body
(`target/stdlib-im4-into146-iterator-spec-logs`). This remains evidence of
solver sensitivity, not a consistently passing single batch. Both runs use
600-second program and 30-second assertion time bounds, disabled caches, and
overflow/panic checks. The six former native restoration probes are promoted
from `known_limits` alongside the new boundary/framing/IntoIterator pairs.
This closes the concrete straight-line native resource cases; combined
resource-state loops, reference-bearing Vec views, and the outstanding array
negative rerun remain open. An array consistency control exceeded 600 seconds
in the resource-bound batch; its timeout is not counted as rejection.

The array regression batch is now covered: all four positive proof cases and
five remaining negative controls passed in build 137's aggregate run. The
timed-out consistency control produces its exact intended assertion rejection
in a fresh build-141 run (468.56s), using a 900-second program limit and the
original resource bounds (`target/stdlib-im4-array147-iterator-spec-logs`).
Together these runs cover all ten selected array proof cases, plus Rust/runtime
checking; the original batch itself still contains the recorded timeout.

The first step is a prerequisite for the others: adding an abstract predicate
only to call-site contracts would not establish that a generic implementation
actually preserves it.

Give parameter and opaque element types an abstract lifetime-resource
interface, rather than treating their unknown internals as empty. Check how
PCG's generalized lifetimes, including regions contained in a type parameter,
map to the encoder's lifetime projections. Definition-site verification and
call-site substitution must agree on that interface. Concrete instances must
justify any conversion between abstract and concrete resources; do not inhale
referent permissions merely because a generic type is instantiated.

Exercise associated types, nested mutable references, and wrappers with two
independent lifetime parameters. Extend Vec's slice-view and borrow-expiry
paths as needed so `Vec<&mut U>` and vectors of reference-bearing structs
carry the same resources through indexing, reborrowing, and iteration.
Verify generic helpers once in a separate `no_std` crate, then instantiate
them with owned values, shared references, mutable references, and non-`Copy`
referents. Add an explicitly permission-checked referent observation only if
the existing specification expressions cannot express the required value
relations; keep pure identity observations shallow.

**Exit gate:** cross-crate clients prove identity preservation, referent
updates, and framing through generic helpers and Vec/slice conversions.
Negative cases reject lifetime merging, stale referent values, and duplicated
or leaked permissions. Unsupported shapes receive a clear diagnostic rather
than silently dropping their resources. Document the precise supported type
shapes instead of claiming all nested types are supported.

### IM5: Combine the extensions and close the gates

Revisit the IM1 summary for elements containing references: its state must
distinguish completed slot identities from the heap-dependent values and
resources of their referents. A shallow `P: Seq<&mut U>` alone is not a
record of those referent values. Prove symbolic loops that update all inner
referents and loops that update only a prefix while preserving the remaining
references and referents. Cover slices, Vec views, reference-bearing wrappers,
and separate library/client crates, with matching wrong-value and wrong-frame
cases. Include empty and zero-sized cases and a negative consistency check
after traversal.

The preliminary direct-indexing value gate now passes. The specification-only
`mut_slice_referent_value(&values, index)` observes a shallow referent snapshot
through the current owned slice, checking bounds, slice permission, and indexed
referent permission. It supports `old` without granting access through the
returned snapshot. `mut_element_prefix_pass` verifies a symbolic prefix update
to 7, unchanged slot identities, and unchanged suffix referent values, with
empty, zero/full, and partial-prefix runtime cases. Its loop explicitly frames
the scalar limit as `limit == old(limit)`.

The full build and initial ten-check observation run passed, as did thirteen
focused regressions for nested/replaced references, loop restoration, native
owned-element iteration, and invalid heap observations. After forbidding the
new observation in executable code, the rerun passed all previous cases; the
new rejection passed a focused two-check rerun after its annotation was
corrected to include the method-body diagnostic prefix. Negative cases reject
wrong prefix/suffix values, false after traversal, both invalid index bounds,
missing owner permission, and executable observation. Logs are in
`target/stdlib-im5-observer-iterator-logs`,
`target/stdlib-im5-observer-regressions-iterator-logs`,
`target/stdlib-im5-observer-final-iterator-logs`, and
`target/stdlib-im5-observer-executable-iterator-logs`. The remaining IM5 gate
still requires generic/native loops, Vec views, and reference-bearing wrappers.

The first combined native-reference probe now reaches loop-summary selection:
`body_restore_prefix!` rejects the loop because more than one restoration edge
mentions the iterator (`target/stdlib-im5-loop140-iterator-spec-logs`). Its Rust
and runtime checks pass. The existing annotation summarizes outer slice storage;
reference-containing elements add a distinct resource-restoration stage. This
is not repaired by selecting an arbitrary edge or weakening the uniqueness
check.

The next implementation steps are:

1. Identify the unique outer-storage edge by its source and keep the additional
   element-resource edges explicit. Preserve their checked call conversions
   when packaging/applying retained obligations; an invariant alone must not
   create a converted resource group.
2. Extend the prefix summary with a separate `Seq<ResourceState>` captured
   while each yielded element is accessible. Its length must match the shallow
   slot prefix. A saved `Seq<T>` continues to represent slot identities only.
3. Attach the value concatenation relation to storage restoration and the
   per-index referent-state relation to the resource-restoration stage. Check
   each entry/iteration package with disjoint prefix, active-yield, and tail
   permissions. Start with before-next annotations and reference updates;
   replacement, live-yield annotations, and Vec ownership require their own
   gates.
4. Verify symbolic full and partial traversals, untouched suffixes, empty
   inputs, incorrect prefix/suffix states, and a false-after-traversal control
   before combining wrappers, separate crates, and Vec views.

The first combined encoding experiment identifies a prerequisite to step 1.
The native-reference loop has three overlapping edges: source storage to the
iterator's outer lifetime, source element resources to that same lifetime,
and source element resources to the iterator's element lifetime. Selecting
only the storage edge gets past the original guard, and comparing element
representations after erasing MIR regions handles reborrowed references.
However, retaining the other two edges as independent wands is insufficient:
their first entry package reads a source snapshot at a loop-head label that
has not yet been reached. The full identity-only probe reports
`assert.failed:labelled.state.not.reached` (build 154,
`target/stdlib-im5-loop151-iterator-spec-logs`); it is not a successful proof.

PCG now groups overlapping loop edges only when their inputs are one owner's
storage and its lifetime resources, and all outputs belong to one blocking
value. It preserves path conditions and every input/output. An initial broader
connected-component grouping changed unrelated loop capabilities (including a
panic in `209_rfmig_demos`); that experiment was narrowed before acceptance.
The final PCG change (`c18a518`, pushed to `LrWm3/pcg`) passes the numbered
corpus, eight function-shape tests, four call-projection tests, and the new
joint graph/entry-action regression (`/tmp/prusti-im5-pcg162.log`).

This replaces the planned independent loop stages with one joint obligation
for these co-owned resources; native call stages remain explicit inside its
checked package. The Prusti integration is still under investigation. Where
storage and its element resources are restored together, the consequent
observes that restored storage, rather than an unavailable pre-loop label.
The package defers its initial referent footprint until the storage can be
read; all action footprints and the final resource consequent remain checked.
The reference and generic identity probes initially failed to establish their
entry invariant with build 161: the package could capture real referents when
its hypothetical tail allowed different slot identities. The antecedent now
freezes the tail's shallow identities. This permits referent updates but does
not yet support replacing unprocessed references. Dependent outer captures
must be scoped in creation order, and applying the saved loop wand must use
its antecedent heap, rather than a later PCG projection label. Finally, all
restored inputs use one unblock graph: separate scripts would consume the
same joint wand twice. All resource consequents remain checked.

With those changes, build 166 passes the generic identity loop (155.14 seconds)
and the concrete reference-update identity loop (206.96 seconds). The latter
also rejects `false` and an incorrect traversal count after restoration
(48.89 and 73.45 seconds). Both include empty/two-element runtime checks where
applicable; the concrete probe is promoted as `mut_native_resource_loop_identity_*`.
Logs are `target/stdlib-im5-generic166-iterator-spec-logs` (three checks) and
`target/stdlib-im5-loop166-iterator-spec-logs` (five checks). These establish
ownership, identities, and traversal, not the final referent-value theorem.

The build-169 regression run passes native resource framing (228.47 seconds),
its wrong-frame control (212.49), scalar whole-prefix restoration (91.38),
false after scalar traversal (43.77), and the after-next/no-edge/wrong-type
rejections. Its overlapping-owner case is rejected with the later stable-edge
diagnostic instead of the expected unique-edge diagnostic; this remains an
diagnostic gate in that run. Including lifetime projections on a prefix of
the source place restores the early ambiguity check. Build 172 passes the
exact overlapping-owner rejection (11.08 seconds) and reruns the generic loop
successfully (161.70 seconds), with Rust/runtime checks in
`target/stdlib-im5-guard172-iterator-spec-logs`.

The four-argument summary prototype takes a separate `Seq<ResourceState>`.
It checks matching prefix lengths and restores each source element's state
from either that completed prefix or the tail's state in the wand antecedent.
Build 171 compiles, but its first full value probe exposes an unordered wand
antecedent: it reads the tail snapshot before listing that storage's permission.
Build 173 sorts the output projections so the iterator's outer storage comes
first. Its promoted identity positive failed once on `next` permission, while
both controls passed; a fresh complete rerun passed (77.05 seconds, build 173,
`target/stdlib-im5-identity174-iterator-spec-logs`). Comparing generated code
also exposed varying loop-permission and saved-snapshot order. Build 175 makes
that order stable and passes all ten checks in one fresh batch:
`target/stdlib-im5-identity175-iterator-spec-logs`. The reference identity,
scalar whole-prefix value, and generic identity positives take 78.53, 32.63,
and 39.68 seconds; scalar false, reference false, and reference wrong-count
controls take 35.07, 60.26, and 123.81 seconds. This is the accepted joint
ownership/identity checkpoint; the earlier failed run remains recorded.

The separate state-prefix positive still fails its restoration annotation
(628.46 seconds, `target/stdlib-im5-state173-iterator-spec-logs`). A deliberately
incorrect recorded-state control is rejected there (422.19 seconds), which
alone does not validate the extension. An isolated diagnostic inserts a failing
assertion after the checked entry package and verifies only `m_visit`; it times
out at 180 seconds before that assertion (`/tmp/prusti-im5-entry176.log`). It is
not acceptance evidence. The next step splits the quantified prefix/tail state
relation to avoid conditional heap observations inside one quantifier. No saved
prefix resource-state proof is accepted yet. All these focused verifier runs
use fresh targets, disabled caching, overflow/panic checks, a 600-second whole
program limit and 30-second solver time bounds; helpers are verified too.


Splitting the prefix/tail quantifiers does not resolve the value proof: build
177 fails with an internal `application.precondition:insufficient.permission`
at the restoration annotation (461.23 seconds). Build 179 captures the restored
storage and the two shallow sequences in `let` bindings, but the full value
probe reaches the runner's 720-second deadline. Neither is accepted. The
identity positive and false control still pass with build 179 (295.14 and
148.06 seconds, plus Rust/runtime checks). Its broader iterator spec-library
and retained owned-expiry rejection also pass (322.23 and 33.98 seconds,
`target/stdlib-im5-library180-iterator-spec-logs`), using the existing default
solver resource bounds and a 900-second program limit.

The checked [joint resource wand model](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/joint_resource_wand.vpr)
is a smaller constructor-stage diagnostic. It packages storage and referent
identity wands, composes them into one obligation with an antecedent-state
relation, and verifies that the real tail and its referent permissions remain
available afterward. All bodies are checked, with no added axioms or
assumptions. The [runner](scripts/test_stdlib_iterator_resource_wand.py) verifies
the positive and rejects both an incorrect state increment and `false` after
packaging. All three pass in one fresh run (12.75, 12.81, and 11.32 seconds,
`target/stdlib-im5-joint-resource-wand-logs`; Silicon `6b540aa4`, 180-second
program limit, 30-second solver time bounds). Reproduce with
`./x.py exec python3 scripts/test_stdlib_iterator_resource_wand.py`. This
establishes the small composition, not a complete Rust loop or IM5 acceptance.

Capturing outer iterator/ghost snapshots in the emitted Rust proof still
fails to complete its first package within a 240-second diagnostic limit
(`/tmp/prusti-im5-capture183.log` and `/tmp/prusti-im5-capture184.log`). Those
experiments verify only `m_visit` and are not acceptance runs. Build 185 instead
captures the checked aggregate resource-state observers before comparing
prefix and suffix sequences, so the quantified relation itself reads no heap.
The full Rust value probe fails its restoration annotation in 101.25 seconds
(`target/stdlib-im5-state185-iterator-spec-logs`). A checked assertion of the
same relation inside the entry script also fails (diagnostic 189), so moving
the final assertion alone does not establish the missing frame.

Extending the standalone model with checked recursive pack/unpack helpers
reproduces that failure (diagnostic 190). Intermediate checked state assertions
make it verify, while a single initial or final assertion does not. Giving the
conversion frame a pure `xs[j]` trigger, in addition to its heap-observer
trigger, proves the composition without those assertions (diagnostic 194).
The extended checked model and both controls pass together in 10.58, 17.16,
and 13.05 seconds (`/tmp/prusti-im5-model196.log`). The helper bodies remain
verified; the trigger adds no assumptions or stronger contract.

The same pure-element trigger on the concrete emitted conversion contracts
lets the Rust entry package complete: a deliberately false assertion inserted
immediately afterward is rejected in 15.49 seconds
(`/tmp/prusti-im5-trigger197.log`). This is caller-only localization evidence,
not full acceptance. Adding an observation marker only to the restoration
quantifier does not fix the proof (diagnostic 195). Build 198 adds the element
trigger to the encoder's pointwise conversion frames. Its full state-prefix positive passes in 674.72 seconds, including all
generated observers and conversion bodies, Rust compilation, and empty/two-
element runtime checks (`target/stdlib-im5-state198-iterator-spec-logs`). The
false-after-traversal and wrong-final-value controls pass in 216.52 and 156.23
seconds. The wrong-recorded-state case first times out at 600 seconds; a fresh
full rerun passes its exact restoration diagnostic in 330.89 seconds
(`target/stdlib-im5-badstate201-iterator-spec-logs`). These use fresh targets,
disabled caching, overflow/panic checks, 600-second whole-program limits, and
30-second solver time bounds. The first run's wall time includes compilation
and setup. The four clients are promoted as `mut_native_resource_prefix_*`.
This is the first accepted symbolic whole-prefix referent-value proof.

The broader regression batch passes reference-loop identity (146.99 seconds),
scalar whole-prefix restoration (34.65), generic identity (42.01), scalar false
(52.88), native wrong framing (119.37), reference-loop false (95.27), and
reference-loop wrong count (73.17), plus Rust/runtime checks. Native correct
framing initially fails its preserved-value assertion after 662.04 seconds;
a fresh full rerun with the same proof bounds passes in 168.56 seconds
(`target/stdlib-im5-framing205-iterator-spec-logs`). Keep both failures in the
evidence: the focused gates pass across these runs, but their solver/runtime
sensitivity remains a limitation.

The next gate is exact early-break framing. A first probe cannot capture the
borrowed owner directly in an invariant even under `old`: Rust rejects the
closure's overlapping borrow. The next implementation captures the initial
resource-state sequence before creating the iterator, and compares the tail
to that pure snapshot. Generic/wrapper/cross-crate loops, Vec views, and full
combined acceptance still follow; IM5 is not complete.

The initial-state capture is now a specification-only builtin,
`sequence_resource_states(&Seq<T>) -> Seq<ResourceState>`. Its checked recursive
observer requires every indexed element resource, and its checked boxing
function preserves length and each observed state. Saved states grant no
permissions. Concrete/generic positives and executable-use, missing-permission,
stale-state, and wrong-length rejections pass all nine checks with build 209
(`target/stdlib-im5-observer210-iterator-spec-logs`, proof times 28.91, 64.05,
21.00, 29.88, 25.98, and 33.79 seconds). The first observer run also generated
unused aggregate conversion methods, whose postconditions failed in the
missing-permission case. The observer now has its own encoder dependency;
actual whole-array conversions still generate and verify their required
methods. This preserves the observer body and contract exactly.

The first complete native-prefix regression after that refactor fails with
`apply.failed:wand.not.found` (337.00 seconds), while all three negative controls
pass (430.69, 279.89, and 164.72 seconds,
`target/stdlib-im5-regression211-iterator-spec-logs`). The emitted program differs
from the accepted positive only by removal of two unused conversion methods,
specification identifiers, and zero-sized closure addresses. A caller-only
replay of the unaltered program passes (`/tmp/prusti-im5-apply214.log`); this is
diagnostic evidence, not full acceptance. Silicon's independent `checkTimeout`
defaults to 10 ms. A fresh complete positive with this check budget increased
to 100 ms passes in 247.12 seconds, plus Rust/runtime checks
(`target/stdlib-im5-check215-iterator-spec-logs`). Assertion and program limits
remain 30 seconds and 600 seconds. This supports a matching-sensitivity
hypothesis; it does not establish the cause or eliminate the recorded failure.

Early-break framing is still unresolved. The initial snapshot avoids Rust's
overlapping capture. Adding shallow-identity framing makes the postcondition
well-formed, but build 204 then fails with a postcondition permission error
(580.44 seconds, `target/stdlib-im5-partial208-iterator-spec-logs`). Splitting
prefix and suffix postconditions and adding checked body assertions instead
fails wand application with build 209
(`target/stdlib-im5-partial213-iterator-spec-logs`). Neither run counts as an
early-break proof. The next probes cover that suffix frame, generic state
preservation, wrappers with two independent mutable references, and Vec views.

The observer checkpoint is validated after cleanup and a complete rebuild
(build 221). All nine promoted `mut_sequence_states_*` checks pass
(`target/stdlib-im5-observer223-iterator-spec-logs`); concrete/generic proofs
take 18.50/54.88 seconds and the four exact rejections take 28.60, 30.62,
23.68, and 34.80 seconds. The whole-array state positive and both controls
also pass with the observer refactor (363.42, 377.55, and 385.25 seconds,
`target/stdlib-im5-array217-iterator-spec-logs`, build 209, 900-second program
limit and the established solver resource bounds). No conversion body was
omitted from those runs.

The larger check budget does not close early-break framing: probe 216 times
out at 600 seconds. Expressing the untouched tail as one state-sequence
equality instead of a pointwise invariant also fails with an unpositioned
`assert.failed:wand.not.found` (428.60 seconds,
`target/stdlib-im5-partial222-iterator-spec-logs`). The generic state-preserving
loop fails its restoration annotation (342.06 seconds,
`target/stdlib-im5-generic218-iterator-spec-logs`). Keep these as open gates;
the observer API's acceptance does not imply acceptance of these loops.

The complete concrete reference-update loop also verifies in a separate
`no_std` library with explicit identity and final-value postconditions. Its
client proves unchanged reference identities and all referent values equal
to 7 using the exported contract. The library, positive, and false/wrong-value
clients pass in 567.69, 66.37, 37.52, and 45.17 seconds, respectively, plus
Rust/runtime checks (`target/stdlib-im5-cross212-iterator-spec-logs`, build
209, fresh targets, 600-second program limit and 30-second solver time
bounds). These sources are promoted into the separate `iterator_resources`
suite; verification under the final repository paths follows the next rebuild.
The runner also exposes `--solver-check-timeout-ms` and records that optional
budget independently of the assertion budget. Its default remains unchanged.

On 2026-10-30, the development branch merges the tested Prusti Ferrum adapter
through `a59ed917c`, pinning all three Ferrum crates to
`afdf10cf59106c38d09df32939f1aa9c77d0b989`. The merge preserves the newer
encoder and state-observer work. Rebuild this branch with both backends before
running comparisons; binaries from the earlier integration benchmark do not
contain these changes:

```sh
./x.py build --all --locked --features prusti/ferrum
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator --backend Ferrum
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite borrow --backend Ferrum
```

The adoption baseline runs current iterator/borrow positives and negatives
once on each backend, retaining exact diagnostics, fresh targets, disabled
caches, overflow/panic checks, artifact/source hashes, and separate logs.
The runner uses one verifier worker and explicitly disables Ferrum branch
parallelism. Routine development then uses `--backend Ferrum`; changed
specifications, negative tests, and unexpected backend differences retain
Silicon checks. Both backends remain required before merging the work into
its destination branch. The final Ferrum pin has not yet passed a complete
stdlib matrix; adoption as the development backend does not close that gate
or the outstanding IM proofs.

The first build of the merged branch succeeds with both backends (checkpoint
226; driver SHA-256 `db5c3235b40c666b32a65dfecab9e466fe7da293e15a92bc0f3732d375e30487`).
Its initial Ferrum borrow run matches 90/109 client expectations and passes
52/53 Rust/runtime/library controls; the library fails an existing unsupported
partial-restoration case. Keep its five timeouts, unsupported-shape rejections,
and the previously documented stale `call_wand_shifted_projection` expectation
in the results. The corresponding Silicon run remains separate.

Comparing emitted programs exposes an adapter omission: native conversion
drops `FunctionData.opaque` and `FuncAppData.reveal`. These annotations control
recursive observer expansion in the current encoder. Both generic-array
programs lose the annotations on the native route and hit the 180-second
baseline limit while Silicon verifies them. The imported adapter is therefore
extended to preserve the existing VIR annotations using Ferrum's annotation
metadata; the Ferrum pin and encoder contracts remain unchanged. A real-Z3
regression verifies transparent and explicitly revealed calls, rejects access
to a hidden body and a wrong revealed value, and checks an incorrect opaque
function body even when unused (five scenarios in one test,
`/tmp/prusti-im5-opacity228-test.log`). The first test draft also produced a
correct secondary caller failure for the deliberately incorrect function
contract; the final control omits that caller and isolates body checking.

The original Ferrum iterator run is stopped after 51 completed clients and its
results retained under `target/stdlib-im5-dual226`. Rebuild and rerun the native
baseline with annotation preservation before relying on it for development.
The unchanged Silicon baseline continues from its frozen checkpoint. Two
separate native reference-replacement positives fail an implicit return
permission postcondition while Silicon verifies byte-identical Viper inputs.
Their complete logs and input hashes are preserved in
`/tmp/prusti-im5-dual226/replacement-differences.json`; annotation preservation
does not account for those differences, so they retain the Silicon gate.

Checkpoint 229 rebuilds Prusti from `474b7ae1d` with both backends and the
annotation fix (driver SHA-256
`68d24cd16b0f2f12afc6d8363b41fd629dee98f26d3b12656f70aff214e72ef3`).
All three formerly timed-out generic-array positives now pass natively:
ordinary, state, and symbolic cases take 106.65, 119.93, and 54.27 seconds.
Silicon passes the same cases in 107.86, 114.79, and 51.55 seconds, and all
three generated-program pairs are byte-identical. Each run checks every
generated helper body at the baseline 180-second program limit. The complete
suite comparison continues separately; this focused result does not close
the remaining backend differences or establish a performance ratio.

The complete Silicon iterator baseline matches 289/320 client expectations
and passes all 121 controls (`target/stdlib-im5-dual226/silicon/iterator`). Its
31 mismatches comprise 20 unsupported cases, seven timeouts, three diagnostic
mismatches, and one internal error. The whole-prefix referent-value positive
passes on both backends (135.40 seconds Ferrum, 103.57 seconds Silicon).
The incorrect recorded-state control reaches the baseline limit on both;
the earlier longer-budget acceptance remains separate evidence.

The corrected Ferrum iterator baseline completes with 284/320 client
expectations matched and all 121 controls passing
(`target/stdlib-im5-dual229/ferrum/iterator`). The paired classification is
282 matches on both backends, 24 shared mismatches, 12 timeout-inconclusive
pairs, and the two native-only reference-replacement failures identified above.
The current replacement-return inputs are byte-identical; the other replacement
pair differs only in standalone comments. Neither iterator run unexpectedly
accepts a negative. Ferrum's full-prefix `false` control passes, but its
wrong-final-value control reaches the baseline limit while Silicon rejects it.
The complete borrow baseline matches 92/109 expectations on Ferrum and 91/109
on Silicon, with 52/53 controls passing on each. The library control fails
the existing partial-restoration encoding error; the paired clients have
90 matches, 15 shared mismatches (including the stale projection expectation),
and four timeout-inconclusive pairs. There is no completed native-only borrow
failure. The original expectations and failures remain in the baseline record.

The promoted `iterator_resources` suite passes all six checks on both backends,
including the separate untrusted library body and all generated helpers.
Library verification takes 57.23 seconds on Ferrum and 79.90 on Silicon;
positive/false/wrong clients take 1.48/1.48/1.95 and 6.86/12.09/12.92 seconds,
respectively. These use the established 600-second program and 30-second solver
time limits; the full iterator/borrow suites retain their 180-second baseline
limits. This closes the concrete cross-crate check at its promoted paths.

The adoption comparison is complete: 432 client pairs, 609 checks per backend,
with 379 Ferrum and 383 Silicon client expectations matched and 176/177 controls
passing on each. The source/artifact audit checks all 463 fixture and contract
files plus all six executables per build before any fixture correction. Its
immutable results are in `/tmp/prusti-im5-dual229/audit-baseline-complete.json`.
Routine iteration now uses Ferrum with one worker and branch parallelism off.
The original policy kept Silicon checks for changed specifications, negatives
and backend differences. The current checkpoint above supersedes that routine
policy; major-feature sign-off and final integration retain dual-backend validation. The full
stdlib matrix at the final pin, generic/wrapper loops, exact early-break
referent framing, and Vec resource integration remain open; adoption does not
complete IM5 or qualify a destination merge.

After freezing the baseline, the stale `call_wand_shifted_projection`
unsupported expectation is replaced by a positive and a wrong-value control.
Both pass their exact outcomes at the promoted repository paths, with all
four checks passing on each backend (`target/stdlib-im5-shift231/{ferrum,silicon}`,
build 229). Positive/negative proof times are 9.72/0.71 seconds on Ferrum and
13.63/4.93 seconds on Silicon, plus Rust/runtime checks. The wrong-value case
changes only the asserted value from 3 to 4 and checks the resulting panic
precondition diagnostic at that assertion. The baseline's original counts and
unexpected acceptance are retained; this fixture correction changes no encoder
or library contract and does not retroactively make the baseline pass.

The remaining generic state-preserving, exact early-break, and Vec reference
loop probes are now retained under the iterator suite's `known_limits` directory
as `im5_generic_state_loop`, `im5_early_break_frame`, and
`im5_vec_resource_loop`. Their Rust checks and runtime cases pass at these
repository paths (`/tmp/prusti-im5-known238-*.log`). The corrected temporary Vec
probe also completes encoding with build 229 in 32.27 seconds
(`/tmp/prusti-im5-vec237-encode.log`); this was explicitly `encode-only`, with
no backend proof. The original generic/early-break proof failures above remain
open. The directory README provides full-program reproduction commands and
distinguishes runtime/encoding evidence from verification acceptance.

The two-reference wrapper probe now proves a symbolic full traversal for
`Pair<'a, 'b> { first: &'a mut u32, second: &'b mut u32 }`. It preserves all
element identities and proves first/second referent values 7/8 at an arbitrary
valid index. The initial final assertions used unsupported nested mutable
dereferences in pure code (probe 232). Ordinary Rust scalar reads preserve the
same assertion and encode successfully. The corrected probe times out at the
180-second baseline (probe 233), then passes its complete Ferrum proof in
369.50 seconds with the established 600-second program and 30-second solver
time limits (probe 235). All generated conversion bodies remain checked.

At repository paths, the focused paired runs use the rebuilt build-229 driver
(`68d24cd16b0f2f12afc6d8363b41fd629dee98f26d3b12656f70aff214e72ef3`),
fresh targets, disabled caching, overflow/panic checks, and one worker.
Ferrum branch parallelism remains off. Both use 600-second program and
30-second solver time limits; Silicon uses its default 10 ms check budget,
while the Ferrum run uses the separately configured 100 ms budget:

| Wrapper case | Silicon (`pair236`) | Ferrum (`pair240`) |
| --- | --- | --- |
| Full positive | Pass, 132.59 s. | Pass, 178.84 s. |
| False after restoration | Intended assertion rejection, 95.21 s. | Intended assertion rejection, 361.51 s. |
| Wrong second-field value | Intended assertion rejection, 191.86 s. | Intended assertion rejection, 349.79 s. |
| Fabricated recorded state | Internal `apply.failed:wand.not.found`, 190.04 s. | Intended restoration rejection, 542.61 s. |
| Wrong first-field value | Internal `apply.failed:wand.not.found`, 242.50 s. | Timeout at 600 s; command time 608.30 s. |

Logs and source hashes are in `target/stdlib-im5-pair236/silicon` and
`target/stdlib-im5-pair240/ferrum`. The positive programs coincide after
renaming seven generated specification identifiers and removing standalone
comments. The differing budgets and generated names preclude attributing
their timing difference solely to the backend or the check budget.

The three cases with exact outcomes on both backends are retained as
`mut_native_resource_pair_prefix_{pass,false,wrong_second}` in `src/bin`.
The two unresolved controls move, without source edits, to `known_limits`;
their original failures remain part of the evidence. No encoder or contract
was changed for these proofs. The full wrapper gate remains open. Next,
verify an untrusted cross-crate wrapper helper with explicit resource-state
postconditions, so value/consistency clients need not repeat the entire loop
proof. A prepared prototype passes Rust runtime and encoding checks only;
its library and client proofs are still required. Materializing outer heap
captures remains a diagnostic hypothesis, not an implemented fix.

The cross-crate wrapper prototype now passes all seven checks on both
backends (`target/stdlib-im5-pair-cross247/{ferrum,silicon}`, build 229).
Its untrusted `copy_pair_values` helper copies both values from an arbitrary
exemplar into every element of `&mut [Pair<'_, '_>]`. The contract preserves
element and exemplar identities, preserves the exemplar's resource state,
and exports a quantified final-state equality with the pure `source[j]`
trigger. The loop explicitly preserves the exemplar's shallow snapshot as
well as its referent state. No new trusted contract or encoder change is
needed. The preceding drafts failed a resource-state observation (probe 243)
and an implicit returned-permission postcondition (probe 245); neither counts
as acceptance.

With matching 600-second program, 30-second assertion, and 100 ms check
budgets, the complete library proof takes 256.34 seconds on Ferrum and 297.51
on Silicon. The positive client takes 6.19/13.76 seconds; the false,
wrong-first, and wrong-second controls reach their exact assertion rejections
in 3.92/8.57, 7.70/12.62, and 15.01/12.77 seconds, respectively. Rust and
empty/nonempty runtime checks also pass. All generated helper bodies are
checked, with one worker, disabled caches, and Ferrum branch parallelism off.
The next validation integrates this helper and its four clients into the
existing `iterator_resources` library at repository paths and rechecks the
whole suite. The separate fabricated-state control, generic state loop,
early-break frame, Vec restoration, and combined matrix remain open.

The expanded `iterator_resources` suite then passes all 11 checks on each
backend at its final repository paths
(`target/stdlib-im5-pair-cross248/{ferrum,silicon}`). The combined library,
including both untrusted helper bodies and all conversion helpers, takes
342.97 seconds on Ferrum and 390.68 on Silicon. The new wrapper positive,
false, wrong-first, and wrong-second clients take 4.79/89.89, 3.14/9.28,
6.03/12.91, and 5.29/23.62 seconds, respectively. The existing single-reference
positive and both controls also pass, as do Rust compilation and both
runtimes. The same build and matching proof bounds are used throughout.
The source/artifact audit in `target/stdlib-im5-pair-cross248/qualification.json`
checks all fixture/manifest hashes, the rebuilt driver, full-library selection,
and all 22 exact outcomes. This closes the cross-crate two-reference wrapper
proof; the separately recorded standalone control failures remain unresolved.

The unchanged generic state-preserving loop also passes with build 229 at its
original `known_limits/im5_generic_state_loop.rs` path: 190.62 seconds on
Ferrum and 309.28 on Silicon, plus Rust/runtime checks
(`target/stdlib-im5-generic249/{ferrum,silicon}`). Both runs use the established
600-second program, 30-second assertion, and 100 ms check budgets and verify
every generated helper. The proof needs neither `Copy` nor an equality bound
on `T`; it preserves the full referent-state sequence and shallow element
identities. The earlier Silicon restoration failure remains part of the
evidence, without a claim that these runs isolate its cause. The alternative
aggregate-invariant draft was not needed or executed. Promotion adds a
post-loop false control and an incorrect final-state inequality, followed by
paired validation at their final paths.

At the promoted paths, the generic positive and wrong-state control reach
their expected outcomes on both backends (`target/stdlib-im5-generic251`).
Ferrum takes 142.16/198.56 seconds; Silicon takes 114.01/434.42 seconds.
Rust compilation and the generic runtime also pass. The false control is
rejected at its intended assertion by Ferrum (152.87 seconds), but Silicon
fails the earlier restoration annotation (573.47 seconds). Preserve that
unresolved case in `known_limits`, with its original expected diagnostic;
it does not pass the consistency gate. The other two sources are retained in
the regular suite. No production encoder change was needed for these results.

The next generic prototype verifies the loop in an untrusted library and
exports identity plus whole-state preservation to separate clients. Its
wrong-state client is guarded by nonempty input, so rejection must exercise
element resources. It is prepared but not yet verified; the direct false
control remains open regardless of the prototype's eventual outcome.

The user has authorized a Ferrum performance upgrade at this checkpoint.
Preserve the old build and all results, review the newer revision's changes,
rebuild this branch with the new pin, and qualify current positive/negative
iterator and borrow cases before adopting it. Keep one worker, branch
parallelism off, and Silicon checks for changed specifications, negatives,
and backend differences. Compare matching sources and bounds; upstream
benchmark results do not qualify this branch or the full stdlib matrix.

The upgrade candidate tested all three Ferrum crates at
`2df948c9014de33b7baa13599eaf9d5078906b28`. A fresh locked build of this branch
succeeds; checkpoint 255 has driver SHA-256
`3a7f358ab4da6d1ccf265472c5693210897a976ea868d4f0d322e426371e461e`.
All seven adapter unit tests and eleven real-Z3 integration tests pass,
including opaque/reveal behavior and helper body checking. The selected
iterator/borrow comparison matches ten client expectations on the candidate
versus nine on the old pin; the remaining old-pin case reproduces an existing
conversion-helper timeout, while the candidate rejects its intended assertion.
Both pass all seven Rust/runtime controls. The full cross-crate resource suite
also passes all eleven checks with every untrusted library and conversion
body checked. The heavy prefix run passes both positives and two negatives,
but the Pair wrong-second control fails at loop restoration instead of its
intended assertion. A fresh old-pin run reaches the intended rejection with
unchanged source and bounds. The candidate is therefore not adopted; the
manifest and lockfile are restored to `afdf10c`, and both builds plus evidence
are preserved. The user then directed work back to the remaining IM items,
skipping full verification. The pending Silicon comparison and an exploratory
build at `4cda819` were interrupted, with no proof/build qualification claimed.
Resume focused IM development on the last qualified pin. Investigate trigger
completeness or qualify an earlier performance revision before retrying the
upgrade, then rerun the current gates and retain dual-backend validation before
merging. Detailed evidence is recorded in `prusti-ferrum/VALIDATION.md`.

IM development resumes with the generic cross-crate helper. The initial
aggregate-only contract verifies its library on Ferrum (93.02 seconds), but
both backends reject the concrete caller's unchanged-referent assertion
(`target/stdlib-im5-generic-cross259/ferrum` and
`target/stdlib-im5-generic-cross260/silicon`). The generic caller's own
state-sequence equality is insufficient evidence of concrete value transport.
The concrete test requires a valid index into `&mut [&mut u32]` and reads the
referent before and after the call; its negative twin asserts a changed value.

The helper now also exports a checked per-element state-preservation
postcondition with the pure `source[j]` trigger, following the accepted wrapper
contract. It keeps the whole-sequence and identity guarantees. No trusted
contract, production encoder change, or client assumption is added. At the new
`iterator_generic_resources` repository paths, all seven Ferrum checks pass:
library 219.13 seconds, positive 10.98, false 12.59, wrong referent 36.42, wrong
state 36.41, plus Rust/runtime checks (`target/stdlib-im5-generic-cross261`).
Silicon passes the same seven checks: library 298.41 seconds, positive 15.91,
false 7.72, wrong referent 11.11 and wrong state 22.07. Both use build 229,
600-second program, 30-second assertion and 100 ms check bounds, full helper
checking, and one worker. This addition makes the existing
state frame usable at a concrete call; it does not resolve the separately
archived direct-loop false control or establish general observer-trigger
completeness. Full-matrix verification remains deferred at the user's request.

The additional non-`Copy` referent pair instantiates the same verified helper
with `&mut Owned`, where `Owned(u32)` has no `Copy` implementation. Its positive
checks unchanged identity and field value at an arbitrary valid index, while
the negative asserts a changed field value. Both focused runs pass all four
checks (`target/stdlib-im5-generic-cross263`): positive/wrong-value timings are
9.55/4.75 seconds on Ferrum and 15.72/9.80 on Silicon, plus Rust/runtime checks.
The helper's source is unchanged. The qualification audit checks source and
driver hashes and all 22 outcomes across the two groups and backends. This
closes the generic cross-crate state-preservation proof for the demonstrated
concrete instances, including non-`Copy` referents. Exact early-break framing,
Vec resource integration, archived standalone controls, and the deferred
combined matrix remain open.

The minimal Vec probes now isolate the missing ownership model before any
iterator is constructed. `im5_vec_ref_read` reads a valid referent through
`&mut Vec<&mut u32>`; `im5_vec_ref_view` updates it through `as_mut_slice` and
reads it through the Vec again. Both compile and run, but both backends report
`application.precondition:insufficient.permission` at the Vec referent read,
currently surfaced as an internal error. Ferrum takes 6.68/1.97 seconds and
Silicon 11.73/7.97 (`target/stdlib-im5-vec262`, build 229, 180-second bounds).
The emitted entry contract has Vec storage permission but no indexed inner
referent permissions. Retain these as positive `known_limits` probes, not
expected-error successes, and defer the larger Vec loop until they work.

The Vec implementation must give the owner a logical initialized-element
projection, tied to its contents and length, with the same indexed referent
resources used by slices. Slice-view construction and expiry must transfer
and return those resources; shared snapshots must not manufacture them.
Preserve the conservative raw-pointer ownership guard. First close the two
small probes with wrong-value/false controls, then the native Vec loop and
framing checks. No full matrix is scheduled.

The focused early-break run fails the `position += 1` overflow obligation
after 239.22 seconds (`target/stdlib-im5-early264`, build 229). The original
source and full Viper program are preserved. A checked `position < limit`
assertion immediately after the break guard, followed by computing the next
position before `next`, times out in `m_visit` at the unchanged 600-second
program bound (653.19 seconds including compilation/encoding; `early265`).
That run does not establish the restoration proof. The next probe keeps the
checked arithmetic and expresses the untouched tail as per-element resource
state equality with a pure `tail[j]` trigger, following the successful generic
proof. It preserves the original prefix/suffix postconditions and is still
unresolved: `early267` also times out in `m_visit` at 600 seconds (612.42
seconds including compilation/encoding). All variants pass the
zero/partial/full runtime cases. No timeout or earlier failure is counted as
acceptance. The current pointwise probe stays in `known_limits`; source
snapshots and generated programs for the preceding forms remain under their
run directories.

The next early-break step is a narrower diagnostic of the generated wand and
its captured heap observations, followed by a checked encoder change only if
that diagnostic supports it. Materializing outer captures remains a hypothesis,
not a repair already established by these runs. The separate early-break
library/client draft is prepared but has not been run or accepted. Stop
repeating whole-probe runs without a new encoding or proof obligation to test.

**Snapshot capture and tail-step diagnostic (2026-10-31).** The short
`early268` Ferrum trace contains 2,761 solver checks and 1,326 `unknown`
responses, with repeated historical heap observations in restoration scripts.
Many reported reasons are incomplete quantifier reasoning; this is not evidence
of one slow query or a backend defect. The encoder now evaluates outer loop-wand
captures once before packaging. It also materializes a call's argument/result
snapshots and the post-call observations already bound outside its returned
wands, then reuses those captures for expiry and retained loop obligations.
Historical heap labels, selected obligation IDs, guarded reads, conversion
statements, and quantified reads keep their original roles. These are checked
assignments, with no additional assumptions or resource grants.

The outer-capture-only build 269 passes the scalar prefix positive/wrong-value
pair. On the unchanged pointwise early-break probe it reports failure to
preserve the untouched-tail invariant at line 34 after 491.77 seconds, within
the existing 600-second bound (`target/stdlib-im5-captures269/full-ferrum`).
This narrows the previous timeout but does not establish the positive proof.
Build 270 adds call captures; its generated early-break program contains 222
pre-call and 31 post-call labelled reads, versus 502 and 238 before either
change. This syntactic reduction is not a general performance claim.

Build 270 passes 34 focused checks across Ferrum and Silicon: scalar prefix
proofs, two live shared-reference yields, branch loops, and retained branch
borrows, including their incorrect-value and false controls. Build 273 differs
only by formatting, comments, and an equivalent return-type alias, and passes
the ten checks for `mut_native_resource_tail_state_{pass,false,wrong}` across
both backends. The new untrusted positive proves, for symbolic slice lengths,
that `next` preserves every remaining referent's state and that writing the
yielded referent preserves that state again. Runtime examples cover empty,
single-element, and longer slices. Both negatives fail at their intended
assertions. Full generated helper bodies are included, with fresh targets,
caching disabled, overflow/panic checks, one verifier, Ferrum branch parallelism
off, and 180/30/0.1-second program/assertion/check bounds. The audit of all 44
checks is `target/stdlib-im5-captures273/qualification.json`.

The revised larger early-break probe `early271` used the generic loop's
source-index invariant. Its unfinished Ferrum run was interrupted in favor of
the smaller tail-step diagnostic; `interrupted.json` records this and no
Silicon run followed. The initial build-270 encode-only invocation used an
invalid mode spelling; the corrected encode-only run succeeds and is not
verification evidence. The remaining early-break problem is carrying the
tail-state fact through the loop's combined restoration and invariant checks.
Keep the current whole-loop probe in `known_limits`. Vec initialized-element
ownership/index routing, the archived standalone controls, and combined IM5
acceptance remain open; the full matrix and Ferrum upgrade remain deferred.

The tail-step positive now also proves shallow slice identity, pointwise
resource-state preservation for the suffix, and unchanged actual suffix values
after dropping the iterator. Its new `tail_state_restored_wrong` control claims
the previous element's saved state and is rejected at that assertion. The
canonical pair passes eight checks across Ferrum and Silicon with build 273
at the same 180-second bounds (`target/stdlib-im5-tail277/qualification.json`):
35.51/71.52 seconds on Ferrum and 33.08/34.53 on Silicon. This establishes the
symbolic single-step suffix frame through expiry; it does not establish the
whole early-break loop. A separate diagnostic starting from an arbitrary
saved-state sequence and offset also passes Ferrum (21.84 seconds, `tail274`),
but has not received paired acceptance. The `early276` diagnostic adds an
explicit tail invariant assertion after advancing the position, before the
backedge. With build 273 and the existing 600-second diagnostic bound, Ferrum
rejects that assertion after 392.72 seconds. The failure is already present
inside the body; it is not solely the final backedge invariant check. This
diagnostic remains unresolved and is not a passing negative test.

Vec's `DerefMut` path exposed a separate PCG call-shape defect: normalizing
`Self::Target` overwrote the element region's origin in `Self`, losing the
dependency from the input element resources to the returned slice's outer
borrow. PCG `b1ed5fe` preserves all normalized signature origins and checks
their existing outlives bounds. Its new regression fails before the change
with exactly that missing edge and passes afterward. Nine function-shape
tests, four projection-map tests, the loop-coupling test, and all 250 numbered
corpus inputs pass with one test worker (`target/stdlib-im5-pcg287`). The fix
is pushed to `LrWm3/pcg`, branch `stdlib/call-wand-projections`; it does not by
itself complete Vec resource integration or the whole early-break proof.

Vec resource integration now has a checked implementation. The encoder keeps
Vec's allocator and bookkeeping fields and adds a logical owned slice of its
initialized elements, bounded by the existing length field. `vec_contents`
projects that slice. Resource conversion uses checked array-typing helpers;
shared indexing reads nested referents through existing indexed permissions.
The `as_mut_slice` and `DerefMut` contracts carry both initial and expiry
referent states. Generic return packaging separates storage restoration from
nested-resource restoration using the call's actual region projections, and
re-packs generic frames before later restoration stages. Equal signature
nodes no longer cause a restoration wand to be discarded: a mutable
collection can contain different references on return.

Build 291 was rebuilt from this branch at `e286dec43` plus the recorded Vec
patch, with PCG `b1ed5fe` and Ferrum still pinned to `afdf10c`. Its driver SHA-256
is `5d0d7151d2235664881707271f2401019a04c309fefb18cab6a3078d7e5d0a88`.
The untrusted generic `mutable_view<T>` library and a concrete-reference
return wrapper verify on both backends. Cross-crate clients use pairs of
mutable references to non-`Copy` tokens: updating the left referent preserves
the right referent, while false and wrong-left/right claims are rejected at
their annotated assertions. All seven checks per backend pass, including
library verification and runtime execution (`target/stdlib-im5-vec291`,
`cross-crate-{ferrum,silicon}`). Library times are 10.82/18.64 seconds and client
times 13.57/14.15 seconds for Ferrum/Silicon. The direct `DerefMut` positive
and both controls also pass after the PCG fix (build 287 Ferrum and build 291
Silicon). Earlier build 285 validates direct reads, slice-view writes, scalar
Vec operations, and selected iterator/borrow regressions on both backends;
its six `DerefMut` failures are retained as failed historical evidence.

These focused runs use fresh Cargo targets, all generated helpers, disabled
caching, one verifier, branch parallelism off, a 180-second program bound,
30-second assertion bounds, and 100-millisecond solver-check bounds. The
native reference-element Vec loop now passes on both backends, including
complete prefix values, element identities, and readback through the restored
Vec at an arbitrary valid index. Its first Ferrum controls timed out; revised
controls check the same complete restoration/readback path before the
impossible claim. Ferrum positive/false/wrong-value times are
162.07/131.36/148.97 seconds; Silicon times are 100.62/101.37/95.84 seconds.
The controls reject exactly their annotated final assertions. The accepted
tests are `mut_vec_resource_loop_{pass,false,wrong}`; evidence is in
`target/stdlib-im5-vec291/loop-ferrum` (positive only) and
`target/stdlib-im5-vec293/loop-{ferrum,silicon}`. Earlier timeouts remain
failed evidence, not controls. Exact early-break suffix framing and
the archived generic/pair controls remain open. The full stdlib matrix is
explicitly deferred at the user's request, including final-pin merge
qualification; no focused result below should be read as that matrix pass.

The final focused borrow regression run also passes all 20 checks across
Ferrum and Silicon: independent call wands, retained branches, and shifted
projection restoration, including their negative controls
(`target/stdlib-im5-vec293/borrow-{ferrum,silicon}`). The accepted Vec loop and
borrow results are collected in that directory's `loop-qualification.json`.

The `early292` diagnostic adds checked intermediate state equalities without
changing the early-break postconditions. It captures the current suffix from
`original[position..]` (the iterator-tail helper is specification-only), then
compares its saved states with the entry states before calling `next`. Ferrum
rejects this first comparison after 411.53 seconds, before either iterator
advancement or referent mutation, using build 291 and the existing 600-second
diagnostic bound with all helpers checked. This exposes a separate gap in
connecting the pointwise invariant to a whole-state observation; it does not
establish that mutation or restoration is incorrect. The generated invariant
has only a tail-element lookup trigger, whereas this failed goal compares
saved-state lookups. The next diagnostic adds an alternative saved-state
trigger, preserving the invariant itself. A separate control-flow probe makes
the impossible `None` branch below `position < limit <= len` checked
unreachable; it compiles, runs, and encodes, but has no accepted proof yet.

Adding the alternative saved-state trigger alone (`early296`) does not close
the loop: Ferrum reports that the tail-state invariant might not be preserved
after 452.85 seconds, with all helpers checked at the same diagnostic bounds.
The next comparison separates pointwise-to-sequence observation from looping
in a smaller untrusted helper, then revisits the original-position invariant
used by the accepted generic state-preservation loop. Neither that proposed
comparison nor the control-flow variant is an accepted early-break proof.

The archived generic prefix false control is now closed on both backends and
restored to `src/bin`. The false assertion follows the existing valid final
restoration/identity/state assertions; build 291 rejects exactly that final
claim in 173.59 seconds on Ferrum and 204.67 seconds on Silicon, with all
helpers checked at the existing 600/30/0.1-second diagnostic bounds. The
promoted source hash matches both runs (`target/stdlib-im5-controls294`).
Ferrum's temporary runner misclassified the bare filename because it required
a directory separator; an exact source-hash/diagnostic/file/line/status audit
is saved as `ferrum/audited-results.json`, preserving the original results.
Silicon passes directly with that location-matcher correction. The two
archived pair controls now also pass both backends and return to `src/bin`.
The fabricated-state summary is rejected at the restoration annotation in
352.25/334.72 seconds (Ferrum/Silicon); the wrong-first-field claim is rejected
at its final assertion in 278.26/330.71 seconds. The latter checks both valid
field values first. The same full-program bounds apply, and the promoted
sources match both runs. All eight checks (Rust checks and six intended
negative diagnostics) are recorded in `target/stdlib-im5-controls294/qualification.json`.
The full stdlib matrix remains deferred.

Ferrum verifies both variants of the smaller `observer297` diagnostic in
10.64 seconds with all generated helpers and a 180-second program limit.
Each assumes pointwise source states at an arbitrary offset and proves that
capturing the whole state sequence preserves that equality; one uses only
an element trigger and the other adds a saved-state trigger. This establishes
the observation step outside a loop, not the full early-break invariant.
The next whole-loop attempt uses the original-position invariant from the
accepted generic state-preservation proof, rather than a tail-local index.

The whole early-break positive now passes on both backends with build 291:
297.78 seconds on Ferrum and 353.53 seconds on Silicon (`early300`). It keeps
the original exact prefix-value, suffix-old-value, and shallow-identity
postconditions. The proof adds explicit lengths for the original snapshots,
entry resource states, and completed prefix; expresses the tail-state
invariant in original-source indices; and checks the suffix state immediately
before iterator destruction and again against the restored source afterward.
These are verified obligations, not assumptions or new ownership grants.
The original-index variant alone timed out (`early298`, 611.69 seconds command
time); adding only the lengths moved the reported failure to the final suffix
old-value assertion (`early299`, 508.51 seconds). Both failures remain failed
evidence. No additional encoder or backend change was required for `early300`.

The positive is promoted as `mut_native_resource_early_pass`; only its opening
comment differs from the paired source. Its audit is
`target/stdlib-im5-early300/qualification.json`. All generated helpers were
checked at the existing 600/30/0.1-second diagnostic bounds, with one worker,
fresh targets, caching disabled, overflow/panic checks, and Ferrum branch
parallelism off. Empty, zero-prefix, partial-prefix, and complete traversal
runtime cases pass. A nonempty partial-traversal false control and a separate
untrusted library with slice/Vec positive and wrong-prefix/suffix clients are
prepared next. Those gates are not yet accepted; the full matrix stays deferred.

The nonempty partial-traversal consistency control also passes both backends:
`mut_native_resource_early_false` rejects exactly its final false assertion in
320.40/320.72 seconds (Ferrum/Silicon), with build 291 and unchanged full-program
bounds (`target/stdlib-im5-early302/qualification.json`). It requires
`0 < limit < len`, retains the positive's original postconditions, and checks
all valid restoration/value assertions before the impossible claim. This
closes the whole-loop consistency gate. The separate-crate slice/Vec suite is
now undergoing paired qualification; it is still pending at this checkpoint.

The first separate-crate run (`early-cross301`) is not accepted. The library
times out at the existing 600-second program limit (612.39 seconds command
time); the slice positive fails suffix readback and the Vec positive fails a
nested shared-view observation. Its six controls reach their annotated
assertions, but those results do not compensate for the missing positive and
library proofs. The failed Rust-only imports and the verification failures
remain preserved in distinct evidence directories.

An explicit pointwise suffix resource-state postcondition, following the
accepted generic preservation contract, lets the slice client verify on
Ferrum (`early-client304`, 9.00 seconds). Explicit ghost captures also establish
Vec identity readback. Its actual suffix value still fails. Checked reads
show the value holds before and after the updater call through the slice,
but the relation is lost before reading through the restored Vec (`305`/`306`).
Capturing the entire view state as another diagnostic instead encounters an
internal `apply.failed:insufficient.permission` error (`307`); this is failed
evidence, not a control. The generated program applies the updater's nested
input restoration wand immediately before the Vec view's restoration wand;
the updater currently has no expiry-state pledge on that input. A separate
prototype adds the established pointwise `after_expiry(values[_])` frame,
relating the original source to its `after_call` identities. This is a checked
contract hypothesis, not an assumed encoder repair. The stronger library
without that pledge is separately undergoing paired verification (`308`).
No new production encoder change or full-matrix run has been made.

The final separate-crate contract adds the pointwise suffix-state postcondition
and a checked `after_expiry(*values)` frame from the entry source identities to
their `after_call` identities. `*values` selects the concrete nested lifetime;
`values[_]` selects an opaque generic group and is not appropriate here.
`Seq::from_slice` supplies the supported snapshot inside `after_call`.
Both simpler slice and Vec client positives now pass Ferrum (`early-client311`).
The Vec client captures its identity snapshots explicitly; it needs no extra
per-element diagnostic reads or whole-view state capture.

At the repository paths, the complete `iterator_prefix_resources` suite passes
all 12 checks on Silicon, including the untrusted library and its expiry pledge
(194.80 seconds), two runtime positives, both arbitrary-index client proofs,
and six exact negative diagnostics. Ferrum passes all 11 non-library checks,
but its library verification times out at 600 seconds (612.56 seconds command
time). All use unchanged build 291 and the full generated helper bodies, fresh
targets, disabled caching, overflow/panic checks, one worker, and existing
600/30/0.1-second bounds; Ferrum branch parallelism is off. The immutable source
copies and honest backend qualification are in
`target/stdlib-im5-early-cross312`. The earlier `308` library run was deliberately
interrupted when the missing expiry pledge was identified; it is not acceptance
evidence. The `309`/`310` selector/temporal errors also remain failed diagnostics.

This closes the cross-crate functional proof on Silicon and validates its
client contract and controls on both backends. The sole remaining focused
qualification gap is Ferrum's complete library proof; a solver trace is the
next step. Do not mark dual-backend IM5 qualification complete on the strength
of the client-only Ferrum checks. The full stdlib matrix stays deferred.

**Combined focused qualification complete (2026-10-31).** The separate
`iterator_prefix_resources` suite passes all 12 checks per backend, including
its untrusted `no_std` library, both runtime clients, arbitrary-index slice/Vec
positives, and six exact-diagnostic controls. Its library uses the accepted
whole-loop proof and preserves the same prefix-value, suffix-old-value, and
identity postconditions. Its pointwise resource-state postcondition exposes
the suffix frame to callers. Its checked `after_expiry(*values)` pledge preserves states
when the nested input borrow expires before the caller restores a backing Vec.
Without that pledge, the slice readback can pass while the later Vec readback
fails. The pledge uses `Seq::from_slice` in `after_call`, following the supported
slice-reborrow contract pattern. It verifies in 254.02/194.80 seconds
(Ferrum/Silicon). The Vec client obtains a mutable slice view, calls the
separately verified prefix updater, and proves both identities and actual
referent values through the restored Vec. Wrong-prefix, wrong-suffix, and
false claims are rejected on nonempty paths for both container forms.

The accepted source/artifact audit is
`target/stdlib-im5-early-cross314/qualification.json`; standalone early-break
positive/consistency audits are in `early300` and `early302`. All use rebuilt
build 291, all generated helpers, fresh targets, disabled caching,
overflow/panic checks, and 600/30/0.1-second bounds, with one worker and Ferrum
branch parallelism off. Two initial Rust-only Vec client launches in
`early-cross301` failed because a specification-only helper was imported/captured in ordinary Rust;
they remain separate failed evidence. The accepted client captures its sequence
through the existing public `as_slice` observation. No encoder or Ferrum change
was made during this final proof qualification. The accepted Ferrum library
run enabled `--reportReasonUnknown` and an SMT prover log. The earlier ordinary
Ferrum run timed out and remains failed evidence. The generated programs are
identical after removing comments and applying a bijective renaming of eleven
fresh specification-helper UUIDs. This establishes equivalent obligations,
not the cause of the timing difference; do not claim a backend performance fix
or consistent timing from this rerun. Silicon's accepted run is unchanged.

These results complete the remaining IM implementation and focused proof
items. They join the paired generic/non-`Copy` library checks (`generic-cross263`),
wrapper update library/clients (`pair-cross248`), Vec view and whole-loop checks
(`vec291`/`vec293`), borrow regressions (`vec293`), and all archived controls
(`controls294`). Empty and zero-sized cases are covered by the generic/native
suites; partial/zero/full stopping points are covered by the early-break suite.
No claim is made for arbitrary nested collections, interior mutability,
callbacks/adapters, or unrestricted generic lifetime decomposition. The full
stdlib matrix and final-pin merge qualification remain explicitly deferred;
Ferrum stays at `afdf10c`, with Silicon retained for specifications, negatives,
unexpected differences, and eventual merge validation.

The focused paired proof work above is complete. The full iterator/borrow and
relevant sequence, Vec, collection, and external-specification matrix remains
deferred at the user's request; retain it for eventual final-pin merge
qualification. Continue using Ferrum for routine checks and Silicon for changed
specifications, negatives, and unexpected differences. Run PCG shape tests and
the numbered corpus when PCG changes. Keep fresh targets, disabled caching,
overflow/panic checks, and one verifier. Ordinary focused cases retain their
180-second bound; the combined resource groups use the documented 600-second
bound. Timeouts, crashes, unsupported features, and wrong diagnostics remain
failed evidence, never successful negative proofs.

Commit and push each working step in logical chunks: proof probes and model,
loop encoding, concrete resource support, generic/cross-crate support, then
combined contracts and evidence. Update this plan with revisions, commands,
timings, and exact remaining blockers. Completion requires both functional
and permission checks; runtime tests alone establish neither.

Interior mutability, arbitrary recursive containers, callbacks/adapters,
additional iterator methods, and yields retained across an unbounded number
of loop backedges remain separate follow-ups. The gates above cover finite
reference-bearing shapes and yields completed within each iteration; broader
claims require their own ownership and loop invariants.

**Additional repair gates (2026-11-01).** The owned-tail state-frame repair
377 passes its minimum on both backends (4/4 each), including the original final
referent-value assertion and its exact false control. Combined PCG candidate
382 includes signature normalization and loop coupling; its 250 corpus, ten
shape, six projection, two loop and seven opaque-resource tests pass. Prusti
has been rebuilt from that source, and all four encode-only controls pass. Its
paired broader proof gate replaces the unstarted 378 gate with the same
coverage plus the minimum owned-reference controls; no broad adoption is claimed.

Canonical sequence candidate 379 removes both observer postcondition failures
from the inclusive Vec positive, leaving its current-owner permission failure
on both backends (about 102 seconds each). Ferrum matches 3/4 expectations;
Silicon matches 2/4 because its negative also reports an unrelated generated
sequence-conversion assertion. That extra error is a failure, despite rejection
of the intended false assertion. The candidate remains unqualified.

The checked nested-reference capture model 383 passes all eight expectations
on both backends, including false, wrong-state and missing-permission controls.
Its byte-identical inputs are retained as `indexed_snapshot_capture*.vpr` in
the iterator `known_limits` directory; `capture383-qualification.json` records
source/log hashes and exact diagnostics. It captures an indexed sequence of
shallow pointer snapshots before wand packaging and preserves the old referent
after parent-pointer replacement. Rust integration and struct/enum projection
paths remain open; the earlier injectivity failure in model 381 is retained.

**Nested capture and staged-restoration candidates (2026-11-01).** Candidate
385 implements the checked shallow indexed observer from model 383 and captures
its whole sequence outside each wand's index binder. Four minimal controls
encode, as do four live-old-reference/parent-replacement controls (388); paired
proofs are queued, with replacement gated on the minimum. The initial 384
encode-only attempt omitted observer-function emission and was retired before
verification; 385 includes the generated function bodies and checked contracts.
The first scope is direct mutable-reference chains, not struct/enum projections.

Associated-output candidate 387 combines the earlier alias normalization and
PCG `19327b5` with a final dependent stage whose entire antecedent is supplied
by earlier independent stages' frames. It retains their permission requirements
and verifies the resulting wand; it does not discard a restoration obligation.
All four corrected associated-output update/replacement controls and the open
Vec control now encode. Their proof gate remains pending. Build 386 failed Rust
ownership checking and is retained as failed build evidence.

Metadata candidate 373's broader Ferrum gate 374 finishes at 46/51. It improves
`mut_region_advance_pass`, `mut_region_functional_pass` and their exact negative
controls over full matrix 339, with no new client failures or accepted negatives.
The library and four full-region checks still fail; Silicon is running. For the
full-region implementation, the generated return packages apply the same split
storage wand twice. Stage classification recognizes only top-level `&mut`
syntax, so physical referents inside `Option`/tuples are misclassified. Candidate
390 classifies the actual mapped lifetime projection as physical or opaque
`RegionsIn` instead. With the 373 fixes, all four full-region controls encode;
the generic implementation now applies the split storage wand once. Its paired
minimum remains queued. Initial build 389 failed typed-index compilation.

The separate current-owner handoff still needs contract and PCG integration:
outer expiry must return the current group plus a checked continuation back to
the entry group. Callers must retain that continuation while using the current
owner, compose sequential/conditional calls, and carry one composed continuation
through loop invariants. Merely changing the wand consequent would leave current
versus entry conversions and historical PCG edges disconnected. The checked
models remain the proof foundation; this integration is not implemented yet.

**Metadata repair adopted (2026-11-01).** Candidate 373 passes its complete
minimum on both backends (7/7 each), and broader gate 374 finishes at 46/51 on
each. Both improve the advance/functional positives and their exact negatives
over matrix 339, with no new client failures and no accepted negative tests.
The existing three indexed Vec entry-resource errors in the library and the
four full-region failures remain failed evidence; they are separate queued
repairs. Coverage, frozen source, driver and harness provenance audits pass.
`metadata373-qualification.json` records the complete decision and comparisons.

The main branch now includes the compiler's slice/str `Pointee::Metadata = usize`
equations, checked storage-typing recovery before nested collection packing,
and the known typed empty indirect-resource body for `str` while retaining its
physical storage permission. The slice/str ownership-transfer controls are
retained in the iterator suite; this does not add a str value-observation API.
The remaining candidates and final combined dual-backend matrix are still open.

**Nested wand trigger diagnosis (2026-11-01).** Candidate 385's minimum
finishes at 4/5 on each backend. Explicit package capture locals in candidate
391 also finish at 4/5: Ferrum reports the missing wand, while Silicon times
out on the positive. All four negative expectations match exactly. A raw
Ferrum replay of the exact 391 program reproduces the missing second method
wand; no observer-function failure is reported.

Read-only AST inspection with the pinned Ferrum Silver frontend identifies
the mismatch precisely. The second contract/package pair has identical bodies,
but two quantifier triggers retain different free snapshot locals. Wand
normalization collects holes from the body, not trigger-only expressions, so
those local names survive in the wand identity. Candidate 396 selects a
captured sequence that actually occurs in the resource body as the trigger.
Its own build and four encoding controls pass; both contract/package pairs
now have identical normalized structures. Its paired minimum and conditional
replacement/framing gate 397 remain pending. Candidate 393's indexed-output
reservation experiment retains the original mismatch and was retired before
verification; its source is preserved, and its reservation change was removed
from the working encoder. Failed build 395 is retained as compilation evidence.
`structure394-diagnosis.json` and the `structure393`/`structure396` artifacts
record the exact comparison. This establishes a diagnosis, not proof success.

Stage-classification candidate 390 finishes at 3/7 on both backends. It removes
the duplicate physical split restoration, but both full-region positives still
fail on the tail's indexed indirect resources. Both negatives have the same
extra error and remain failed checks. The candidate is not adopted.

**Nested proof follow-up (2026-11-01).** Candidate 396 finishes at 8/9 on
each backend: the missing wand is resolved, but the nested positive lacks
immediate indexed output resources. Combining checked reservation 393 with the
trigger fix in candidate 398 also finishes at 8/9 each; the remaining error is
now an indexed unfold in the second package. All negative controls match
exactly, and all coverage/provenance audits pass. Neither candidate is adopted.

Exact raw replay 401 locates that unfold. Diagnostic 402 adds one checked
equality between a captured sequence address and the old pointer used by the
unfold. The assertion succeeds and the original unfold failure disappears;
the package's final entry-resource restoration still fails. This identifies
missing quantifier instantiation as part of the problem. An arithmetic trigger
in diagnostic 403 is rejected by the frontend before verification. Diagnostic
404 instead adds an equivalent, checked absolute-index observer postcondition,
with a valid input-selector trigger; its frontend check passes and proof is
queued. It adds no assumed resource or equality.

The exact full-region replay 400 is rejected before verification because the
text frontend disallows conditional trigger terms in that generated program.
It provides no proof location; the original Prusti 390 failures remain the
verified diagnostic evidence. Enum broader gate 364 is running on Silicon;
its Ferrum gate is 28/32, with no regressions against identical baseline tests
and no accepted negatives. Current-main build 405 combines the enum repair
with adopted metadata fixes and encodes all five option controls; its paired
gate is queued. Three repeated-call nested controls (406) are prepared to
check caller permissions and false claims after calls once the minimum works.

**Historical enum reconstruction adopted (2026-11-01).** Current-main
baseline 407 reproduces the original option-slot positive's missing snapshot
permission (4/5 Ferrum expectations). Its driver is byte-identical to the
accepted metadata-only build 373. Candidate 405 adds only the enum historical
snapshot reconstruction: the original positive and exact false/wrong controls
pass on both backends, including Some/None runtime behavior. This establishes
the repair independently of the metadata change.

The adopted code reconstructs an unfolded enum from its actual historical
discriminant and readable active-variant fields, preserving generic casts.
It never substitutes a later owner snapshot. The new `mut_element_option_false`
control is retained byte-for-byte. Original minimum 364 is 5/5 per backend;
its broader gate is 28/32 per backend with no regressions among identical
baseline tests. Current-main gate 405 is 7/8 each: the new escaped-old-reference
option replacement positive still fails a restoration assertion. That failure
is not waived or included in the qualified scope. `enum405-qualification.json`
records the source, build, comparison and exact-control evidence. Option
replacement and final combined qualification remain open.

**Current-main cross-crate and loop follow-up (2026-11-01).** Build 452 is a
fresh rebuild of main `a3a96bdfe` with PCG `269cb9a2e` and Ferrum `0e048a4`,
without the pending encoder experiments. Gate 453 passes all five selected
generic-resource expectations on each backend, including the verified spec
library. The prefix-resource gate passes its runtime, positive and exact
negative clients, but its library reaches the 600-second limit on both
backends (4/5 each). Coverage and provenance audits pass. Follow-up 459 raises
only the whole-program limit to 1800 seconds; assertion and solver-check
limits remain unchanged. The original timeouts remain unresolved evidence.

Owned-loop candidate 455 is the first to pass all ten focused Ferrum checks,
including the extra-`next` positive (143 seconds) and exact false control.
Silicon passes nine: the positive still reports a function-precondition
permission failure. The candidate is not adopted. Exact-source paired replay
462 passes the main method on both backends, and complementary replay 464
passes the remaining methods. Full native Silicon replay 465 also passes,
but does not waive the original failure. Fresh encodings differ in loop-fact
and restoration-obligation order. Candidate 471 orders those collections;
seven fixtures produce identical normalized Viper programs in two fresh
encodings (471/472). Its expanded gate passes 14/14 checks on each backend.
Gate 484 adds nonempty normal-return and early-return wrong-result controls;
all three checks pass on each backend. These focused results do not qualify
the candidate: broader gate 473, using the formatted own rebuild 485, passes
28/32 on Ferrum. The library has three package-local snapshot declarations
used outside their scope, and both native mutable-loop cases encounter an
unused post-call heap read after transferring the source permission. The
extra-`next` false control reports an unexpected permission error after 509.72
seconds. Its Viper program is alpha-equivalent to the passing 471 program,
with distinct generated identifiers preserved by the comparison. Silicon
broader validation is skipped after these failures. Its checked
loop invariants reconnect historical call-entry snapshots, retain stable
functional postconditions and dropped-result resources, and preserve the
discriminant and moved payload of the live yield. Control 463 adds a wrong
result postcondition after normal return, so an explicit false assertion
cannot make the return-restoration path unreachable. Early-return fixtures
458 pass in the expanded 471 gate, with the stronger early-return control
passing separately in 484.

Nested-reference replay 445 locates candidate 424's failure at the second
call's nested restoration assertion inside the second return package. Probe
456 adds checked assertions around the call and package boundaries, but reaches
its 600-second limit without localizing the failure. Candidate
460 captures old RHS and frame observations outside the wands, consistently
in contracts, callers and checked packages; all seven fixtures encode and
their wand structures match. Its Ferrum gate is 12/13: the repeated caller
still fails to find a nested restoration wand. Diagnostic 474 instead exports
the callee's pre/post observation sequences as ghost results and reuses those
exact values in caller obligations. It passes parsing and structure checks, but caller verification fails at
the next indexed-frame permission assertion after 552.47 seconds. The full
callee/program gate 477 is therefore skipped. Own rebuild 486 extends checked
physical-frame equalities through quantified indexed predicates; fixture gate
487 also adds a normal-return false postcondition. All eight fixtures encode
and the repeated caller's wand structures match; solver qualification is pending. Neither
matching structure nor successful encoding establishes resource availability.
Artifacts and exact build hashes are recorded under
`target/stdlib-matrix-20261031-1236`; current status is in
`checkpoint-live.json`.

Constructor-metadata candidate 478 moves slice and string metadata equations
to the existing per-constructor fact encoder. This avoids introducing an
unused string type merely by resolving `Pointee`, while retaining the laws
for constructors actually used by a program. Its own Prusti rebuild succeeds;
the optional-slice/string gate passes 6/7 on Ferrum, failing the slice
unpacking helper's whole-sequence postcondition. The candidate is rejected,
the two metadata files are restored to main, and follow-up 479 is skipped.
The unchanged prefix library also times out at 1800 seconds in 459; its
other four checks pass. No performance improvement is established.
Generated-caller handoff diagnostic 476 and its exact false-postcondition and
missing-current controls pass all six paired expectations in 483. Its
abstract trait stub still has an unverified stronger contract. Eight native
Rust handoff fixtures encode and four positives pass at runtime, but the
current-main single-call positive and negative both fail the current-resource
reservation. The native implementation remains open.

Own rebuild 491 fixes historical snapshot declaration scope and selects
retained postcondition observations before emitting their heap reads. The
affected library and three clients encode; the full library now passes
Viper parsing and type checking. Gate 492 passes all eight checks on each
backend, with complete source/build provenance. Broader owned-loop
qualification remains separate.

The independent naming fix is accepted in `adf34ef15`: generated Viper names
use stable crate and item identities instead of random specification UUIDs.
Rebuild 499 passes both identity tests and all five generic-resource checks
on each backend. Its borrow gate is 5/6 on each: `split_pass` retains the
existing current-resource reservation failure. Clean-main replay 506 reproduces
that failure on both backends with identical original diagnostics; the exact
negative still passes. This failure remains open. Candidate 500 combines the
scope/capture repair with the accepted names; six complete programs repeat
identically, excluding comments, across fresh encodings 500/502. Its minimum
gate passes all 20 checks on each backend, including the library, owned
whole-prefix loops, native mutable loops and extra-`next` negatives. Audit 500
confirms complete coverage and provenance. Gate 510 passes all 13 additional
checks on each backend, covering owned reference writes, indexed mutation and
owned-value regressions. Their audited union covers exactly the 32 checks of
the earlier broader gate, with identical fixture sources and no waived
failures. This owned-iterator repair is adopted with PCG `9dfaf88ca` and eight
regression fixtures. The PCG pin removes phantom post-call outputs for moved
owned arguments. `owned500-qualification.json` records the build and complete
paired coverage; the final combined matrix and unrelated repairs remain open.
Obsolete naming candidates 494/498 and owned minimum 491
are superseded rather than counted as additional passing proof gates.

**Native current-owner handoff prototype (2026-11-01).** Candidate 504 emits
current resources and a checked current-to-entry continuation when an outer
wand restores a mutable owner's storage and nested resources together. Its
concrete Rust callee and false-postcondition control pass on both backends.
The generic caller reaches the remaining entry-resource restoration failure,
instead of failing the current-resource reservation. Candidate 505 captures
the actual continuation after applying the callee wand, selects it using owner
identity and validity guards, and applies matching continuations in reverse
order when restoring entry resources. Boundary conversions for handed-off
resources use the restored current heap. All 16 native checks pass across
Ferrum and Silicon: callee, single-call, repeated-call and conditional-call
positives and exact false-postcondition controls. The conditional negative
requires that its call branch runs. All generated methods are checked; this
is native encoder evidence, unlike the earlier edited-stub experiments.

Candidate 513 adds conservative MIR input provenance through uniquely defined
direct reborrows and moves. Address/type guards alone (509/512) still try to
apply an already consumed continuation for another owner. Provenance narrows
the selected continuations without introducing an aliasing assumption; Viper
still checks every application. All 28 native checks pass on both backends:
the earlier cases, concrete callers, two generic owner parameters, and two
owners of the same generic type, each with an exact false-postcondition
control. `handoff513-qualification.json` checks source, binary and diagnostic
provenance. Projected and ambiguous aliases still need qualification.

The unbounded native-loop baseline 508 fails both expectations. Prototype 514
introduces one checked current-to-entry continuation per opaque input owner,
composes it at ordinary borrow expiry, and carries it through loop invariants.
Its positive still lacks the current-snapshot equality at the loop boundary;
its false-postcondition control is rejected correctly. Rebuild
515 binds that snapshot when the invariant retains the complete mutable
reference resource. It encodes and passes Viper scope/type checks. Its positive
now fails the checked increment's overflow assertion: a body invariant after
the while guard must retain `index < count` to justify that increment. The
negative produces exactly the intended false-postcondition error, but a source
audit found that 514/515 copied an older fixture without 508's `count > 0`
precondition. Those controls do not qualify nonempty-loop reachability. Gate
518 retains the checked strict bound and restores that precondition, using
the same 515 binary. All four checks pass across both backends. No encoder
assumption is added. Broader fixtures 517 cover repeated and conditional
calls, early returns, calls before/after a loop, nested loops and two owners.
All six positives pass at runtime. Gate 519 uses those same executable bodies
with checked strict bounds and nonempty negative preconditions. It passes
8/12 Ferrum checks: before/after and nested-loop cases still refer to a
future loop-header heap while packaging PCG history wands. Candidate 520
integrates this work with adopted owned-iterator main and reproduces those
four failures among 28 native Ferrum cases.

Candidate 523 replaces only the internal pure opaque input history wands
already covered by the checked owner continuation. Storage, projected/mixed
outputs, concrete split groups and explicit prefix-restoration annotations
keep their existing PCG obligations. The new continuation is initialized and
composed by checked wand packages, with current snapshot equalities when the
loop retains the complete owner resource. All 28 native cases pass on each
backend, including the seven unbounded loop variants and exact negative
controls. `handoff523-qualification.json` verifies source/build provenance and
nonempty/conditional reachability preconditions. Broader gate 524 has completed
on that fresh binary, as recorded below. These prototypes remain unadopted; instantiated group
mapping, projected/ambiguous aliases and broader regression qualification
remain open. Package-local captures and continuations must stay in scope.

Gate 516 on candidate 513 is 4/5 for generic-resource checks and 5/6 for borrow
checks on each backend. The borrow failure remains `split_pass`. The library
failure is not evidence of a handoff regression: its emitted program has no
new handoff terms and is alpha-equivalent to the earlier passing library
499, with only crate identity suffixes changed. Direct verification of the
complete, unedited 516 Viper program passes Ferrum in 167.17 seconds (521).
The native library failure remains recorded and needs reproducibility and
adapter-path diagnosis; the direct run does not replace its qualification.


**Integration follow-up (2026-11-01).** Candidate 523 also passes the exact
32-check owned-iterator regression set on each backend. Audit 524 confirms
complete coverage and source/build provenance. Its generic-resource gate is
5/5 on Ferrum and 4/5 on Silicon, where the library still reports loop
restoration failure. Borrow checks are 5/6 on each backend: `split_pass` still
fails its current-resource reservation. These failures remain open.

Gate 525 adds direct/conditional aliases, projected generic owners, two concrete
lifetime groups and a generic callee instantiated with those groups. All six
positive fixtures pass at runtime, but verification passes only 4/12 checks on
each backend. Direct aliases and direct concrete groups pass; conditional aliases,
projected generic fields, concrete-group loops and a generic callee instantiated
with several groups fail on both backends. Wrong diagnostics on their negative
controls are failures, not successful rejection. Candidate 529 propagates logical
input identities through whole mutable-reference assignments and branches, then
captures the selected identity at each call. This adds no address-distinctness
assumption. All 16 checks pass across both backends: direct/conditional aliases,
same-type owners and the existing two-owner loop, each with an exact negative
control. Audit 529 checks fixture, binary and diagnostic provenance. Projected
group mapping and dynamically selected owners inside loops remain open.

Gate 527 adds an arbitrary-count borrowed-counter postcondition and an unchanged
frame. Its runtime check and all four encodings pass, but the functional positive
fails its postcondition on both backends. The wrong-count, wrong-frame and false
controls produce exactly the intended error (3/4 per backend). Diagnostic 530
separates the two postconditions, checks an explicit invariant preserving the
immutable count, and removes the owner call in a baseline. It is queued; those
variants do not replace the original failed qualification. Candidate 531 extends
the checked loop summaries to individual concrete input groups, preserving the
existing treatment of explicit prefix-restoration annotations. Its own rebuilt
binary is frozen and its paired concrete-group loop and alias regressions are
queued. Neither candidate is adopted yet.

Fresh build 526 combines handoff 523 with the pending nested indexed observations
and physical frames. Its three focused programs encode and the complete repeated
caller passes Viper structure checks, but emits no new owner-handoff terms. In the
resumed proof gate, the simple nested positive passes while the repeated-call
positive reaches the 600-second limit. This combination does not establish a
nested-call repair. Independent build 528 combines 523 with canonical generic
sequence observers and their checked pointwise state law from 379. It targets
the remaining inclusive-lookup resource reservation and resource-prefix library
obligations; its paired gate is queued. Both builds remain isolated and unadopted.

**Interruption recovery.** On resuming after the Paseo interruption, no old worker
remained. The incomplete 526 run and partial 529 freeze were preserved and
identified separately. Candidate 529 was rebuilt from its own source, all six
binaries were frozen and hashed, and its complete paired gate reran successfully.
Original pending workspace contents were restored byte-for-byte. Recovery and
build-cache cleanup records are retained with the probe evidence. The resumed
queue uses one verifier with Ferrum branch parallelism disabled; interrupted
checks do not count as completed proofs.


**Further isolated repairs.** Candidate 532 shares consumed-callee-obligation
tracking across output packages, targeting a generic callee instantiated with
several concrete resource groups. Candidate 533 adds conservative provenance
through uniquely defined field reborrows. It identifies the caller's group by
its opaque type, and captures the enclosing input owner's identity at the call;
the actual checked continuation is unchanged. Its tests include distinct generic
field types and two fields of the same type. Candidate 535 adds checked
conditional composition into the selected input's loop summary. Both new loop
positives run successfully and all four positive/negative variants encode; the
dynamic-alias positive also passes Viper structure checks. Their paired native
gates are queued. None is adopted or counted as verified yet.

The complete 526 gate is 6/8 on Ferrum, with full coverage and provenance audited:
both repeated-call expectations reach the 600-second limit, while the simple
nested positive and its exact controls pass. The generated program applies call
restoration wands during function-output packaging. Separate parent and referent
restoration stages across two calls need intermediate parent state. Candidate 534
therefore experiments with a joint checked restoration obligation for dependent
nested groups, restricted to unit-return, pledge-free functions taking a mutable
slice of nested mutable references. The complete antecedent must be proved when
an original individual PCG request selects it; duplicate call/id/guard requests
cannot replay it. This strengthens the required restoration input and is a
conservative, scoped experiment, not unrestricted independent-lifetime support.
Three programs encode and the full repeated caller passes Viper structure checks;
its paired proof gate is queued behind the shorter native gates.

Candidate 528 has completed both borrow gates at 5/7 each: `split_pass` and
`vec_lookup_inclusive_pass` remain failures, and both negative controls pass for
the intended reason. Generic-resource checks now pass 5/5 on each backend,
including the library (310.27 seconds on Ferrum, 116.38 on Silicon). The
resource-prefix gate passes 5/5 on Ferrum, including its library in 217.64 seconds
and both cross-crate controls. Silicon's prefix gate is still running. This is
promising evidence for the canonical observer and checked pointwise state law;
it does not close the outstanding borrow failures or combined qualification.

Twenty superseded frozen builds are being archived losslessly to recover disk
space. Each archive is decompressed and every payload hash is checked before its
online binary copy is removed. The original directory retains its archive
manifest and restore command; the restore helper checks every binary hash again.
Recent qualification binaries remain online. No proof result, source patch or
failed-run log is removed.

The design review also consults
[Sound Automation of Magic Wands](https://dardinier.me/papers/CAV22_Wands.pdf)
and [Automatic Verification of Iterated Separating Conjunctions using Symbolic Execution](https://arxiv.org/pdf/1603.00649).
The former requires one footprint compatible with every permitted antecedent
state; the latter explains framing heap-dependent observations through quantified
resources and controlling their instantiation. Applying those principles here
means freezing continuation identities before packaging, preserving one linear
restoration obligation, and proving pointwise observer laws instead of relying
on arbitrary recursive unfolding. These are design requirements for the pending
encoder work, not evidence that any particular installed backend is defective.

### Combined repair checkpoint (2026-11-01, probes 531–540)

The canonical observer candidate 528 completed its prefix-resource gate at 5/5
on each backend, including both full specification-library proofs. The split
and inclusive-vector borrow positives still fail. These isolated results do not
establish the combined encoder or the full cross-crate suites.

Native provenance audits establish these focused results:

| Candidate | Scope | Result |
|---|---|---|
| 531 | Concrete resource-group loop summaries and prior alias/owner controls | 10/10 on each backend |
| 532 | Consume each shared callee obligation once across output packages | 8/12 on each backend; projected owners and generic multi-group restoration still fail |
| 533 | Conservative projected-field provenance, including equal-typed fields | 12/14 Ferrum; generic multi-group restoration still fails |
| 535 | Conditional owner-summary composition inside loops | 12/14 Ferrum; aliases selected before the loop still fail |
| 536 | Functional counter and frame qualification with a checked invariant preserving the loop bound | 4/4 Ferrum, including all three exact postcondition controls |
| 537 | Joint checked restoration of multi-group owners | 14/14 Ferrum, including both previously failing generic multi-group cases |
| 539 | Pure alias-selection invariants and pre-loop frames for inactive summary values | 14/14 Ferrum, including aliases selected before and within the loop |

The original functional fixture's missing loop-bound invariant failed even
without an owner call. Candidate 536 adds a checked invariant, with no stronger
precondition or changed runtime body; the original failed evidence is retained.
Candidate 538 was withdrawn before proof execution: permission introspection
inside the proposed modular loop invariant did not provide an acceptable framing
argument. Candidate 539 instead uses ordinary ghost selection values and checked
pre-loop captures. It does not assume facts about a future loop heap.

The joint nested-restoration candidate 534 passes all eight focused Ferrum
checks. Its repeated-call positive takes 358.38 seconds and its exact false
postcondition control 482.02 seconds. Individual restoration requests must prove
the entire combined antecedent. This establishes the focused Rust examples,
not general independent early restoration of nested lifetimes.

Fresh combined build 540 merges 539, the joint-restoration generalization 537,
the pending nested indexed observations and reservations, and the canonical
resource observer changes. The merge retained all pending root encoder work;
its only textual conflict was a comment about linear callee obligations.
Source and all six binary hashes are frozen in `build540.json`. Its routine
Ferrum gate contains 50 owner-handoff programs (75 checks including runtime and
Rust checks), the accepted 32-check owned-iterator regression set, full generic
and prefix cross-crate suites, all eight nested-reference programs, and the
remaining focused borrow regressions. Qualification is running; this is not an
adoption or a full-matrix result. A fixture-preparation interruption is retained
separately and is excluded from qualification.

All twenty archival jobs completed with verified lossless payloads. The original
binary directories retain manifests and checked restore commands. No source,
proof result or failed-run evidence was discarded. Continue with combined
qualification, adopt passing changes in logical commits, repair the remaining
matrix failures, and then complete D1–D6. Silicon follows the major-feature
policy above rather than running for each intermediate repair.

### Combined regression follow-up (2026-11-01, probes 540–547)

Build 540 passes the exact accepted owned-iterator inventory at 32/32 Ferrum
checks (`audit540-owned.json`) and the full generic-resource cross-crate suite
at 10/10, including its specification library. All 50 owner programs produce
the intended verifier outcomes, but eight duplicated annotations introduced by
the fixture importer make the harness result 67/75. This remains a failed
harness gate. Rerun 541 uses the identical frozen encoder with corrected,
formatted fixtures; its 75 checks must pass before those files are adopted.

The prefix suite is 11/12: every client, runtime check and negative passes, but
the library exceeds the 600-second verification limit (619.32 seconds including
startup). Comparing its generated program with passing candidate 528 shows an
extra indexed physical reservation and external captures introduced by the
nested repair. Candidate 546 restricts those to dependent restoration stages
with frames or a bundled owner. Its generated prefix program differs from 528,
after normalizing generated crate identifiers, only by unused logical owner-ID
assignments. That is structural diagnostic evidence, not a passing proof.

The repeated nested caller also fails in 540: `assert.failed:wand.not.found`
after 161.94 seconds, while the simple nested positive passes. Its normalized
comparison with passing 534 shows different observer identifiers and unused
owner IDs. The encoder generates four equivalent shallow sequence observers
for concrete references because their task keys retain function-specific
lifetime/context data. Candidate 547, based on 546, canonicalizes ground
physical observer tasks while retaining indexed paths and preserving generic,
alias and inference contexts. Its encoded repeated caller uses one shared
observer. Three Rust programs encode; proof qualification remains queued.
This does not establish a defect in either verification backend.

Separately, candidate 544 extends owner handoff to a stage whose owner storage
is supplied by an already-restored, checked frame. The old split-borrow
contract restores physical storage first, then returns only entry resources in
a later stage; the current-owner reservation consequently cannot be proved.
The candidate's focused gate includes the original positives/negatives, an
untrusted forwarding wrapper and its false postcondition, and a zero-sized
split positive/false pair. Four programs encode and pass full Viper structural
checks. None of these checks counts as verification. Candidate 544 does not
include the later 546/547 changes.

The old frozen harness also silently ignored a misspelled negative selector in
540's borrow job. Its coverage is explicitly incomplete in
`coverage540-correction.json`; the intended `inclusive_wrong_frame` control is
queued in 544 and unchanged-build recheck 542. Main commit `434cfaf11` now
rejects every unmatched case pattern and malformed annotation inventory before
compilation or verification. CLI controls confirm both rejection paths, and
all existing fixture annotations satisfy the preflight. This harness fix is
committed and pushed; the encoder candidates remain isolated.

The serial queue is 540, 541, 544, 546, then an adaptive tail containing 547 and
542. Recheck 542 uses unchanged frozen 540 binaries for associated outputs,
escaped enum replacement, optional metadata, region reconstruction and the
remaining heavy prefix cases. All use Ferrum, one worker and branch parallelism
off. The next combined encoder must retain the successful repairs, resolve
these regressions, and pass its relevant complete gates before adoption and
major-feature/final Silicon qualification. No failed or interrupted result is
waived. Runtime logs, source/binary hashes, queue state and frozen programs are
preserved under `target/stdlib-matrix-20261031-1236`.

### Split restoration and associated-output integration (probes 548–552)

Corrected, formatted owner fixtures in 541 pass all **75/75 Ferrum checks**
with frozen encoder 540. The coverage and source/binary audit is complete;
the failed 540 annotation results remain preserved separately.

The focused split-borrow progression is:

| Candidate | Ferrum result | Interpretation |
|---|---|---|
| 544 | 12/16 | Storage-frame handoff alone introduces an unframed current-snapshot read. The untrusted forwarding wrapper also consumes element resources during its storage stage. Both zero-sized controls pass. |
| 548 | 13/16 | Putting physical frames before dependent current-owner observations fixes the original `split_pass`. Forwarding and inclusive lookup remain failures. |
| 549 | 15/16 | Recognizing a direct physical footprint inside tuple/ADT outputs keeps storage and element restoration in their respective stages. The untrusted forwarding positive and exact false-postcondition control now pass. Only `vec_lookup_inclusive_pass` fails. |

All three inventories and diagnostics are audited. Candidate 549 does not
classify a heap-dependent or indexed footprint as direct aggregate storage.
These results do not yet qualify the combined encoder or a new supported
feature domain. No routine Silicon reruns were added.

The 546 prefix suite finishes at 11/12: all clients and controls pass, but its
library still reaches the 600-second bound. Candidate 550 additionally removes
private integer owner-origin definitions only when the completed CFG never
observes any member of that identity group. Guards, invariants, captures and
other uses retain the complete group; removed expressions are restricted to
integer constants and copies between those private locals. Its encoded prefix
program matches passing 528 after removing comments and normalizing generated
crate names. Its complete Ferrum prefix gate subsequently passes 12/12,
including the specification library in 206.00 seconds. The native result and
source/build provenance are audited in `audit550.json`.

The same build's nested gate finishes at 9/11. The repeated-call positive and
its false-postcondition control both report `assert.failed:wand.not.found`;
the latter therefore fails its expected-diagnostic check. The simple positive
and remaining negative controls pass. The prefix repair does not close nested
restoration.

Canonical-observer candidate 547 still fails the repeated nested caller with
`assert.failed:wand.not.found` (196.90 seconds). Its simple nested positive
passes. Disk exhaustion subsequently interrupted logging during a negative
case: the partial 4/5 result has **incomplete coverage** and is not a completed
gate. Both its failed log and the failed 552 build are preserved. The original
workspace bytes and PCG pin were restored from verified backups; superseded
frozen binaries were archived losslessly, with every payload hash checked
before removing the online copy. Sources and proof logs remain available.

The inclusive lookup diagnosis also exposed a missing integration prerequisite:
current PCG `9dfaf88` does not include experimental associated-output dependency
change `19327b5`. Port `00b72ae` retains that relation on the current iterator
graph and adds an inclusive `get_mut` regression at both signature and call
sites. All 28 focused PCG tests and its complete numbered fixture corpus pass.
The commit is pushed to `LrWm3/pcg`, branch
`stdlib/associated-output-dependencies`; main's submodule pin remains unchanged
pending Prusti qualification.

Candidate 552 combines 549 and 550 with that PCG port, alias-preserving resource
conversions and the checked fully framed final restoration stage from 387.
Its first build failed for lack of disk space, before any binaries were frozen.
The fresh rebuild succeeds with all six binary hashes recorded in
`build552.json`; original pending workspace bytes and the main PCG pin are
restored exactly afterward. Seven valid encode-only programs and three complete
Viper structural checks pass. A misspelled encode-only selector is recorded
separately and corrected; it is not counted as a proof result. The focused
24-check native Ferrum gate is running. Broader qualification is conditional
on that gate passing, and retains the corrected 50 owner fixtures from 541.
Raw diagnostic 551 reproduces the inclusive lookup failure on the unedited
548 program: applying the outer restoration requires the slice's captured
element resources. It does not qualify a Rust client or establish a backend
defect. The next native integration gate must also retain the owner, iterator,
generic, prefix and nested checks before adoption.

Raw nested diagnostic 553 selects `m_visit` from the unedited native 550
program. With the normal 100 ms internal check budget, it localizes the failure
to the first helper call's saved restoration wand in the final package
(164.24 seconds). Raising that budget to 1000 ms reaches the overall 600-second
timeout; it does not establish a backend defect or resolve the failure.
Diagnostics 555 and 556 respectively add checked assertions immediately after
each call and move the existing entry-snapshot evaluations before their calls.
They preserve the original obligations and are intended to distinguish capture
equality from later restoration. Neither modified Viper program counts as
native Rust qualification. Routine work remains Ferrum-only.

### Canonical observer adoption and staged-call follow-up (2026-11-01)

The two canonical resource-observer files are adopted as a separate logical
change. Their bytes match frozen 528, 550 and 561 exactly: type-parameter
collections retain their existing canonical permissions and use the same
sequence observer, whose pointwise observation law is proved by its recursive
body. Qualification includes 528's generic and prefix libraries on both
backends and 550's complete 12-check Ferrum prefix suite. This adoption does
not claim that the pending combined encoder or remaining borrow cases pass.

Combined 552 finishes at **21/24 Ferrum checks**, with complete, audited
coverage and provenance. Escaped-reference replacement fails for a missing
permission in both its positive and false-postcondition control; inclusive
lookup also fails. All split controls, basic associated-output borrowing,
ordinary Vec use and framing pass. Raw diagnostic 557 identifies replacement's
missing permission as the Vec storage predicate before its second lookup.

PCG candidate `791e4ca` preserves nested function-output target sets instead
of adding all component outputs as dependencies of every input. Equal sets
remain coupled; crossing sets keep conservative coupling; loop coupling is
unchanged. Four algorithm tests, 29 focused graph/shape tests, the explicit
early-storage-expiry regression under both borrow-checker modes, and all 252
numbered fixtures pass. The iterator fixture's two old combined-edge
annotations are replaced with four checks covering creation and expiry of both
stages at the original required point and branch guards. The initial failed
annotation run is retained. The candidate is pushed to `LrWm3/pcg` on
`stdlib/stratified-call-restoration`; main's pin remains `9dfaf88`.

Both nested diagnostics succeed: 555's additional checked call-boundary wand
assertions take 329.53 seconds; 556's relocated entry captures take 255.41.
These are diagnostic timings, not a performance guarantee or native Rust
qualification. Fresh build 558 combines PCG 791e4ca with checked assertions for
materialized joint owner obligations. Its borrow gate exposes a new mismatch:
the physical owner can be made concrete between independent expiry stages,
while a later stage still expects its generic storage frame. Its native nested
checks remain in progress. Fresh candidate 561 adds checked conversions around
such frame applications and restores the prior concrete representation
afterward. Both builds and all original pending workspace bytes are preserved;
561's native borrow/nested gate and conditional broader gate remain queued.

### Entry capture placement and frame representation (probes 558–566)

The completed native 558 gate is **19/24 borrow checks** and **9/11 nested
checks**, with complete coverage and matching source/build provenance. The
basic associated-output positive and its false-postcondition control now also
fail a permission check. No negative was unexpectedly accepted. Raw replay
560 localizes the basic case to the later restoration stage's generic Vec
storage frame, after an earlier stage returned that storage in concrete form.

The call-boundary assertions from diagnostic 555 do not fix the full native
nested program: 558's repeated caller fails after 179.97 seconds and its false
postcondition reports the wrong wand diagnostic after 425.23 seconds. After
normalizing comments and generated crate identifiers, the raw and native
programs match, but the successful raw invocation selected only `m_visit`.
That context difference prevents treating the diagnostic as native validation
or evidence of a backend defect.

Fresh candidate 563 replaces those assertions with entry-snapshot evaluation
before the actual method call, after argument conversions. Post-call captures
remain after the call. The move requires a fresh local with exactly one
assignment, an entry-labelled expression, and inputs not written across the
call; dependency-ordered entry captures can move together. Expressions and
heap labels are preserved. Five Rust programs encode and pass full Viper
structural checks. These are not proof results; the full native nested gate
is queued on Ferrum.

Inspection also finds that 561's physical-frame cast emits no conversion in
the three associated/inclusive reproductions: decomposing the canonical
reference constructor retains its generic referent. Its native gate is
withdrawn before execution, with all build and structural evidence retained.
Fresh 565 normalizes that referent in the caller's context before selecting the
checked conversion. Its borrow gate contains 24 checks; broader 566 is
conditional on both that gate and 563's nested gate passing, and includes
nested revalidation with 565. Superseded broader jobs are recorded as skipped,
never passed. Routine fixes and their negative controls remain Ferrum-only;
Silicon is reserved for major feature completion and final integration.

The completed 563 nested gate is **10/11 Ferrum checks**, with audited coverage
and provenance. All six negative controls match, including the false
postcondition after 420.59 seconds; the repeated-call positive still fails
after 107.70 seconds. Exact full-program raw replay 567 also fails, after
348.74 seconds, during the second helper's restoration. The earlier successful
method-filtered diagnostic therefore remains insufficient for qualification.

Native-AST inspection 568 finds the same two wand classes as the parsed
program, but their argument lists differ: implicit full predicate permissions
give 36/19 arguments in the native representation and 40/21 after printing and
parsing. Temporary inspection code is isolated and is not an adoption candidate.
Candidate 569 makes full permissions explicit in the Prusti Ferrum adapter,
preserving supplied fractional and wildcard expressions. Eight structural
adapter tests pass, including equality of implicit/explicit wand shapes and
arguments; five Rust programs encode and pass Viper structural checks. The
eleven solver-backed adapter tests and native nested gate remain queued. This
representation difference is not yet established as the cause of the failure.

The corrected storage conversion in 565 passes the basic associated-output
positive (58.28 seconds) and its exact false-postcondition control (54.51).
Escaped-reference replacement advances past the missing-permission failure,
but its final current-value assertion fails after 265.21 seconds. Inclusive
lookup still fails a permission check. The forwarding wrapper also exposes a
scope error: a representation flag assigned inside a package was declared
outside its proof script. Fresh 573 declares the flag with its application
script. Its additional replacement diagnostic checks the value immediately
after writing while retaining the original final assertions and the original
fixture. Its focused borrow gate is followed by the other unresolved matrix
families in 574, subject to the native adapter tests passing. Broader results
must retain any focused failures and cannot establish combined qualification
on their own.

### Restoration ordering and transferred values (probes 571–580)

The native adapter tests finish at **11/11**, in addition to the eight structural
tests. The explicit-permission candidate 569 finishes at **9/11 nested checks**:
the repeated caller and its false-postcondition control still fail to find a
wand. Coverage and source/build provenance are complete; no negative is accepted.
The representation change is therefore not a demonstrated repair. Diagnostic
572 checks explicit repackaging of the captured obligations without adding
assumptions; its full-program Ferrum run is pending.

Candidate 573 fixes the proof-script scope but introduces duplicate declarations
when a retained obligation is applied in multiple branches. It is withdrawn
before native verification. Candidate 576 allocates a fresh application-local
flag at each use while preserving captured contract identities. All three
encoding and full structural checks pass; the 26-check borrow gate is queued.
Broader 577 will attempt the other unresolved matrix families even if a focused
case fails, preserving those failures in its record.

Exact replay 575 locates inclusive lookup's missing slice permission. Inspection
shows the outer vector restoration consuming that storage before the later
slice obligation needs it as a frame. Diagnostic 578 moves the later slice
restoration earlier within the same guarded action phase. Every statement and
obligation remains; structural checking passes and proof is pending. A successful
diagnostic would still require a dependency-based implementation and native
positive/negative qualification.

Replacement's generated obligation transfers reference resources without a
value-preservation relation between matching resource identities. Candidate 580
adds an identity-guarded relation for plain resource leaves: compare complete
keys across the antecedent and consequent, and preserve the resource observation
when they match. Antecedent keys and observations use `old[lhs]`. Each observation
is supported by permissions on its own side; there is no permission-query guard.
Quantified, conditional and bound groups are excluded until their scoped
correspondence is modeled. Untrusted callees must prove the stronger package.
This follows Viper's [documented antecedent-state observation semantics](https://viper.ethz.ch/tutorial/magic-wands-heap-dependent.html).
The earlier 579 prototype is withdrawn before proof because its key comparison
used the wrong heap. Six encoding and structural checks pass on 580, including
an untrusted generic permutation, a false-value control and a false-contract
control. An untrusted generic slice identity and its false contract additionally
check staged owner restoration. Native proof remains pending; no feature is
qualified by these encoding results.

### Small untrusted contracts and historical reference addresses (581–588)

Candidate 576 completes at **22/26 Ferrum checks**, with audited coverage and
source/build provenance. Fresh application-local flags fix both forwarding
controls. Escaped-reference replacement still loses its final current value,
and inclusive lookup still lacks its slice frame. Diagnostic 578 advances
past that frame failure after reordering, but lacks entry resources; diagnostic
581 applies the returned owner obligations and advances again, to an old-vector
resource observation. Neither diagnostic establishes native qualification.

Candidate 580 fails native well-formedness: placing `old[lhs]` around a pure
resource key makes that expression an argument evaluated when creating the
wand, before the antecedent heap exists. Structural checking did not catch
this. The earlier concern about using fixed keys directly was too broad.
Candidates 585–586 keep fixed keys outside `old[lhs]`, retain it on the
heap-dependent resource observation, and exclude unsupported key expressions.
586 also recognizes heap-free generated snapshot constructors; its emitted
untrusted exchange contract contains the intended transfer facts. All four
encoding and structural checks pass. Native proof remains required.

The unchanged 576 baseline's small-contract gate 582 matches **5/11 checks**.
The generic exchange loses caller values, an untrusted slice identity cannot
establish its staged owner obligation, and direct mutable-reference replacement
fails permission restoration. Candidate 585 restricts a current-owner handoff
to a stage that also restores that owner's storage. Its untrusted identity
definition and exact false-contract control both pass, but its caller still
fails. The forwarding wrapper now reports a resource-permission failure, so
this restriction is not qualified for adoption.

Inspection of direct replacement finds a separate address error: when the
original loan expires, the assigned reference storage already points to the
replacement target. Candidate 587 selects the reference address using the
borrow's historical lifetime-projection label, with the same precedence as
existing labelled resource accesses. It also omits an empty transfer conjunction.
It rebuilds Prusti from its own source; five programs encode and pass structural
checks, including branch-dependent writes and two successive replacements.
Native positive and exact false-value controls are queued. Original pending
workspace contents and the accepted PCG pin are restored after the build.

The serial queue compares the small 586/587 contracts, then uses frozen 587
for the broader unresolved families rather than repeating the slow gate on
586. Any focused failures remain open alongside broader results. Full-program
diagnostic 588 will localize the small identity/inclusive failures only if
they remain on 587. Routine changes and all their controls continue on Ferrum;
Silicon is reserved for major feature sign-off and final integration.

### Audited small restoration regressions (586–593)

Candidate 586 matches **10/13 Ferrum checks**; its untrusted generic exchange
now preserves both caller values and rejects the exact false-value and
false-contract controls. Candidate 587's historical reference-address fix
matches **18/19** small checks, including direct, branched and repeated
replacement with exact negative controls. The larger gate remains **11/17**:
Vec replacement, forwarding and inclusive lookup are still open.

The phase-local ordering repair in 590 removes the small identity caller's
permission failure, leaving a final value failure. Its full focused gate is
**17/21**. Candidate 591 extends guarded value transfer to one quantified scope,
retaining both bounds and complete resource identities. Its small gate is
**23/23**, with complete coverage and matching source/build provenance,
including the untrusted generic const-array definition, the slice-identity
caller, and all exact controls. The fifteen Rust regression programs from this
gate are committed as the required behavior; this does not adopt the pending
combined encoder or qualify the unresolved larger cases.

Diagnostic 592 confirms that IndexMut's generic restoration factors a returned
element resource through the index argument before restoring the collection.
At `I = usize`, that intermediate resource cannot carry the element's state.
Candidate 593 combines overlapping nested-resource stages while keeping
physical storage and disjoint groups separate. It also checks scalar/indexed
transfers using complete underlying resource identities. It builds; native
qualification is pending. An additional generic element fixture initially
failed Rust E0508 because its specification moved a non-Copy slice element;
the corrected borrowed-snapshot specification adds no Copy/Clone bound.
The failed source and encoding result are retained.

### Isolated historical-reference repair and remaining gates (591–595)

The historical-address repair was rebuilt independently on main `392ffe06f`
with the existing PCG `9dfaf88` pin (probe 595). A mutable loan now reads the
assigned reference from the heap version recorded by its lifetime projection.
Replacing the reference's storage must not redirect expiry to its new referent.
The direct replacement positive and exact false-value control both pass Ferrum.
The conditional and repeated replacements still fail value assertions without
the pending transfer contracts: the complete isolated gate is **6/10**, including
preflight and three runtime checks. This is an adoption of the address repair
only, not completion of the replacement milestone. The combined candidate 591
passes all six of these proof controls and its complete small gate is **23/23**.
Probe 595's patch and binary hashes are retained in `build595.json`; its source
patch is `682e868cab3348aa3c24e1faffda9a229e72c9da6bf3d79a5273a01fb17d30da`.
The original pending encoder changes remain separate from this repair.

Candidate 591's larger replacement gate remains **4/6**: the positive loses the
updated element value and the negative fails at that earlier assertion, rather
than its expected location. Candidate 593's broader scalar/indexed transfer
rule is **24/27** on its small gate and is not accepted. Its new untrusted
`first_element` definition cannot establish a value equality for every indexed
resource with an equal underlying key. Diagnostic 594 restricts that equality
to the actual selected index and retains the entire program: the definition
and selected value then verify, but the client's unchanged-second-element
assertion still fails. This localizes the missing index correspondence; the
weakened diagnostic is not a general fix or a passing qualification.

The full-program nested-call experiment 583 still fails wand matching after
201.72 seconds despite deduplicating identical captures. That transformation
is not adopted, and this result does not establish a backend defect. The wider
matrix-family gate 577 was changed before execution to frozen candidate 591,
whose small ownership gate passes; it excludes candidate 593. Its results must
still be audited, and known focused failures remain open independently.

Next, qualify scalar/indexed value transfer with its actual resource-index
correspondence, preserve current-owner resources together with their entry
restoration obligation through forwarding, and resolve nested-call wand
identity. Current and entry resource layouts cannot be assumed interchangeable.
Combine accepted repairs, rerun the remaining matrix failures and final matrix,
then stop at the user-requested boundary below. Routine checks remain Ferrum;
Silicon is reserved for completed feature and final integration sign-off.

The wider frozen-591 gate has completed its owned-iterator group at **30/32**
(audit `audit577-owned.json`): coverage and source/build provenance match, and
all negative controls report their exact expected diagnostics. The iterator
specification library and `native_for_extra_next_pass` remain failures. The
owner-handoff group and subsequent generic, prefix, nested, region, enum,
metadata and heavy-prefix groups are still being run serially.

Inspection of the library's generated contracts found a concrete mismatch:
`transfer_frames` rejects generated associated-type domain functions as stable
keys. Consequently `Pointee::Metadata` suppresses the trait call's value frame,
while the sized wrapper requires the corresponding frame. Candidate 600 starts
from 591 and recognizes only generated `_assoc_type_` functions returning
`Type`, whose domain declarations are heap-independent. Its own Prusti build,
six encodings and six structural checks pass; native qualification is queued.
Its source patch is
`3dcc3859ba09a6fd1248fe050962be19a2c356a005afdc9dc62d97feb55cb045`.
Candidate 599's separate proposal to add value frames to owner continuations
is held without native qualification; it does not repair this specific missing
trait-call frame and is excluded from 600. Neither candidate is adopted.

Two further diagnostics are queued after the wider gate. Probe 596 retains the
full nested-call program and returns four explicit ghost capture values from
the callee, with checked equalities to its original entry/post observations.
It tests capture identity without dropping the original proof obligations.
Probe 597 checks a standalone resource-state coherence lemma for two indexed
wrappers of the same underlying resource, including an inhabited false-assertion
control. Both pass structural checking only so far. Their results cannot be
substituted for native Rust regression qualification.

### Wider regression audit and guarded observations (577, 601–607)

All nine groups of 577 have been audited against their exact selected
fixtures, source hashes and frozen binaries: **112/170 pass**, with no
unexpectedly accepted negatives. The generic, metadata and further prefix groups
pass. The prefix library reaches its 600-second backend limit; all its clients
and exact negative controls pass. The repeated nested-call positive and its
false-postcondition control fail a restoration value equality after 583.78 and
598.37 seconds, respectively. These are completed proof failures, not timeouts.
Both whole-region positives and negatives fail earlier indexed-permission
assertions. Enum replacement also fails permission restoration; the other enum
controls pass. None of these failures establishes stabilization.

Candidate 601 combines the pure associated-type key repair with consistent
value frames on owner continuations and loop summaries. Its six encodings and
six structural checks pass. Native checks cover the small untrusted ownership
gate, all 75 owner-handoff checks and the affected iterator library/clients.
The candidate remains unaccepted while those checks are queued.

Further structural inspection in 604 finds invalid quantifier triggers in the
whole-region encoding: a sequence lookup contains a redundant conditional
read under an enclosing enum branch. Candidate 605 keeps the conditional only
when an observation is hoisted outside that branch. Its nine encodings and
nine structural checks pass, including both whole-region variants. The same
small repair was rebuilt independently on main with PCG `9dfaf88` in 606;
six encodings and six structural checks pass, and its metadata/enum proof
controls are queued. Neither candidate is adopted. Structural success does
not resolve the whole-region permission failure.

The smaller generic optional-slice regressions in 607 reproduce this syntax
failure independently: all three variants encode but have invalid triggers on
baseline 595, and all three pass structural checking on isolated 606. They
cover a generic length method with non-Copy elements and Some/None/empty
clients, with exact false-value and false-postcondition controls. Their native
gate is included in 606's 17 selected checks; proof results remain pending.

The earlier `first_element` test also lacks an explicit contract for its
unchanged-second-element assertion. Fixtures 602 retain that assertion in a
stronger positive with checked selected-resource and untouched-index expiry
pledges. Exact false-value and false-postcondition controls accompany it; a
separate negative retains the original weaker contract and must reject the
untouched-value assertion. All four encode and pass structural checks on 601;
native results remain pending. The old failing fixture is retained as evidence.
Diagnostic 603 extends the pending pairwise coherence lemma to finite
sequences, with state frames and an inhabited duplicate-value false control.
Its two structural checks pass, but it is held pending the result of 597.
The duplicate-value controls matter because predicate permission can exceed
one in [Viper's predicate model](https://viper.ethz.ch/tutorial/predicates.html).
Wrapper ownership alone must not be treated as a distinctness axiom; the
coherence helpers must prove their equalities and preserve an inhabited
false-assertion control.

### Checked restoration and capture experiments (600–614)

The metadata-key candidate 600 completes at **23/23 small checks**, **7/13 owner
controls**, and **6/6 iterator-library/prefix checks**. Combined 601 completes
at **67/112**: 23/23 small, 32/75 owner and 12/14 iterator checks. Its iterator
library and all selected positives pass, including extra-next in 384.77 seconds.
The extra-next false assertion instead encounters an earlier permission error;
the nonempty false-result control reaches the 600-second backend timeout.
Coverage, exact diagnostics and source/build provenance are audited. No negative
is unexpectedly accepted, and those two failures remain open.

Owner continuation frames alone are not enough to compose the final value facts.
Full-program diagnostic 610 keeps the original contracts and assertions, but
returns the antecedent resource directly when its complete key already matches
the required entry resource. The original restoration history runs otherwise.
The positive verifies in 1.01 seconds; an added false assertion is the sole
failure in its 0.76-second negative control. Compiler candidate 611 applies this
checked branch to flat resource footprints, owner continuations and loop summary
composition. It passes **59/75 owner checks and 23/23 small checks**, an improvement
of 27 owner checks over 601. Eight positives and their eight negative controls
still fail for multiple owners, projected fields or split groups. Diagnostic 614
case-splits complete cross-owner keys and considers a permutation of the whole
footprint; it still fails a package value frame. Diagnostic 619 instead skips
each unchanged owner’s history independently, while retaining the original
contracts and assertions. Its full-program positive and false control are queued.
No key-distinctness assumption or frame removal
is used, and 611 is not adopted.

The full nested-call capture diagnostic 596 verifies in 408.46 seconds with
checked explicit entry/post capture results. Compiler candidate 609 implements
returned capture witnesses for concrete signatures containing nested sequence
observations. Generic type/const and trait interfaces retain their previous
protocol. Its own build and six complete encoding/structural checks pass.
Native qualification completes at **17/19**: 9/11 nested checks and 8/8 small
controls. The repeated positive still fails a package value frame in 439.40
seconds, and its false-postcondition control reaches the backend timeout.
The raw diagnostic does not establish native qualification on the newer
combined implementation.

The pairwise indexed-resource coherence lemma 597 and finite-sequence lemma 603
both verify, including inhabited duplicate-value controls rejected only at their
false assertions. Full-program diagnostic 608 adds the checked sequence lemma
to the original generic `first_element` encoding. The generic definition and
selected-value proof verify; the only remaining failure is the client's
unchanged-second-element assertion, whose original contract does not promise
that frame. All original obligations remain. This result supports implementing
the coherence proof in the encoder, not accepting that unsupported client claim.
The stronger specification drafts in 602 remain unqualified at **2/6** checks:
old-state observer permissions and caller restoration still fail, and the weak
contract's negative reaches an earlier value assertion. Preserve these failures
while correcting the contracts and implementing the checked correspondence.

The isolated enum observation repair 606 completes at **15/17**. Existing metadata
and enum controls pass 12/12, and the new generic false-postcondition control
fails exactly as expected. The new positive and false-value control still have
package observation permission errors. Removing the invalid trigger is therefore
insufficient for these programs; the repair remains unadopted. The next repair
must preserve guarded observations when Viper lifts wand arguments outside a
conditional branch.

Exact extra-next replay 612 retains the full failed program and adds solver
reason/prover logging. Diagnostic 613 compares the existing separate-method-solver
option on the full positive/negative pair, with unchanged limits, one worker and
branch parallelism off. Neither changes the development default or substitutes
for native qualification. All evidence remains under
`target/stdlib-matrix-20261031-1236`, including audits 600, 601, 602, 606, 608, 610
and 611. Finish the remaining repairs and combined qualification, then observe
the mandatory pause before D1–D6 and automation work.

### Checked indexed correspondence and guarded captures (615–620)

Diagnostic 616 extends the checked sequence lemma to all pairs of equal resource
keys, preserving every permission and observation. In the full original program,
the only failure remains the unsupported second-element assertion. Compiler 617
implements the checked pair, range and whole-sequence methods, and invokes them
when restoring scalar resources into an indexed sequence. Its selected-element
gate passes **6/6**, including the exact negative that must reject the untouched
second-element claim. The small restoration controls pass **23/23**. These results
are audited for coverage, source hashes, binary provenance and exact diagnostics.
Replacement qualification remains open; the initial replacement positive still
encounters a permission error. No combined encoder adoption is claimed.

Contract experiment 620 reuses the exact own-built 617 compiler and makes only
an explicit suffix resource-state promise. It places the entire entry observer
inside `old`, and removes the earlier unconditional selected-resource promise,
which would be invalid if a later borrower changes that resource’s identity.
The original failed 602 fixtures remain preserved. The new positive, false-value,
false-postcondition and weak-contract negative are queued for native checks.

Isolated capture candidate 615 passes **19/21** checks: metadata 7/7, enum 5/5,
address 4/4 and generic optional-slice 3/5. Capturing only final-package arguments
does not fix the new generic cases: the temporary output reservation still
contains a free heap observation inside a variant guard. Candidate 618 applies
the same guarded capture/substitution to that reservation and its application.
Its own build, nine encodings and nine complete Viper structural checks pass;
96 native checks are queued, including the full existing owner-handoff group.
The original pending encoder files were restored byte for byte after each build.
Neither capture candidate is adopted on the strength of structural checks alone.

### Owner history ordering and current capture qualification (621–630)

Full-program diagnostic 625 verifies in 7.32 seconds, with its sole false
assertion rejected in 7.17 seconds. It compares complete resource keys, bypasses
unchanged histories, and restores one owner's entry resource before consuming
another owner's equal-key current resource. Complete permutations need no history
replay. Every branch proves the original wand, including all value frames; no
key-distinctness axiom is added. Compiler candidate 628 implements this strategy
for two flat owner footprints and retains the original strategy for other layouts.
Its eight encoding and structural checks pass. The full owner gate is audited at
**71/75**, improving by 12 checks over 611, and the small gate passes **23/23**.
The remaining positive/negative pairs are `generic_split_groups` and
`projected_same_type`. Experiment 630 retains projected field histories separately
and tests the same checked ordering in the latter complete program.

Capture candidate 618 completes at **47/96**. Its generic optional-slice gate is
4/5, metadata 7/7, enum 5/5, address 4/4 and owner 27/75. The 75 owner outcomes are
identical to an audited run of the adopted-main compiler (623); these are existing
failures, not newly introduced capture regressions. The generic definition's
observer error is repaired, but the generic caller still fails restoration.
Direct optional-slice experiment 626 passes 4/5: its false controls are exact,
while the positive cannot prove the returned length after RangeFull indexing.
Experiment 629 uses built-in array-to-slice coercion to isolate that boundary.
The original failed clients remain preserved.

Combining captures with the staged encoder exposed an incorrect classification
of physical storage inside nested references. Candidate 624 classifies the
resource group's lifetime kind, instead of treating only outer argument group
zero as physical storage. All 13 encodings and structural checks pass. Its native
gate completes at **48/52**: selected-element 6/6, small controls 23/23, enum 5/5,
generic optional-slice 4/5, metadata 6/7 and explicit suffix 4/6. The suffix
specification's definition verifies, but its caller cannot apply the restoration
wand. The metadata negative has an extra resource-unpack helper postcondition
failure as well as the intended false assertion, so it remains a failed check.
Candidate 622's earlier encoding failure and all intermediate evidence are kept.

The full extra-next false replay 612 fails an earlier indexed-resource permission
check in 516.78 seconds and reports incomplete quantifier reasoning. This remains
a failed negative qualification. The separate-method-solver comparison is queued;
no default backend setting has changed. Source, binary, exact-diagnostic and
coverage audits remain under `target/stdlib-matrix-20261031-1236`. These repairs
continue within the current stabilization scope; the mandatory pause below still
applies before deferred feature work.

### Adopted guarded-observation repair (618, 629, 631)

The isolated capture repair from build 618 is adopted with three direct
optional-slice regression fixtures. It removes redundant variant conditionals
from inline indexed observations, while retaining guarded captures for every
restoration package and temporary output reservation. Reservation packaging and
application use the same saved values. The staged encoder matches the exact
own-built 618 source; pending combined work remains separate.

Both baseline 631 and repaired 629 pass **5/5 Ferrum checks**, including direct
Some/empty/None clients with non-`Copy` elements and exact false-value/false-post
controls. The Viper front end rejects all three baseline programs for invalid
conditional triggers; all three repaired programs pass structural checking.
The isolated repair also retains 16/16 metadata, enum and address controls, and
all 75 owner outcomes match adopted main exactly (27 pass, 48 existing failures).
This is a bounded capture/trigger repair, not completion of generic caller
restoration, RangeFull associated-output values, or combined stabilization.
Those failed clients and the latest combined failures remain in the evidence.
No additional Silicon proof run was made for this small fix; major/final
qualification retains the user's Silicon policy.

### Projected histories and delayed collection conversion (632–638)

Candidate 632 retains actual struct-field projection paths when grouping owner
histories. It splits a flat footprint only when the paths account for every
resource leaf; whole-owner histories, loop summaries and unsupported layouts
retain the original proof. All ten encodings and structural checks pass. Its
native Ferrum gate is audited at **102/104**: owner **73/75**, small controls
**23/23**, and selected-element controls **6/6**. The projected same-type positive
and exact negative now pass. The generic split-group pair remains failing;
stabilization is not complete and this combined candidate is not adopted.

The separate-method-solver comparison 613 passes the full iterator positive in
90.61 seconds, but its negative fails an earlier `next` permission precondition
in 193.44 seconds. It therefore does not qualify, and backend defaults remain
unchanged.

The full replacement replay 627 locates the missing scalar permission after a
later Vec indexing loan expires. The current referent is held inside an indexed
wrapper when the delayed `mem::replace` continuation requests it. Diagnostic 633
opens that wrapper, executes every original restoration operation, and closes
the same wrapper after the intervening physical folds. The complete positive
verifies in **143.16 seconds**. Diagnostic 635 adds a final false assertion and
rejects exactly that assertion in **274.64 seconds**, with no earlier error.
Both source hashes and exact results are audited. These are full-program Viper
diagnostics, not Rust regression qualification.

Candidate 637 implements temporary scalar exposure using complete resource keys
and index/path captures from encoded array expansions. A permission check selects
an actually held wrapper; checked unfolding, generic conversion and refolding
must restore it before the next call, phase or package boundary. It retains all
original restoration and frame obligations. Its own Prusti build is complete;
all eight encodings and structural checks pass. Its focused Ferrum gate is
audited at **35/35**: selected-element 6/6, small controls 23/23 and replacement
6/6. The two replacement positives verify in 169.20 and 154.42 seconds; the
negative rejects exactly its intended false assertion in 287.46 seconds.
Build 636 is retained as the
unverified predecessor before the package-boundary flush was added.

Independent candidate 634 extends explicit checked capture outputs to ordinary
generic functions, retaining the associated-item interface. Its 13 encodings
and structural checks pass, but the optional-slice caller still fails with an
application permission error. Remaining controls and raw localization 638 are
complete: **52/56** checks pass (generic optional-slice 8/9, metadata 6/7, enum
5/5, selected-element 6/6, small controls 23/23, explicit suffix 4/6). The
metadata positive fails a checked raw-sequence helper assertion; the original
suffix caller still cannot apply its wand. This candidate is not adopted.
Raw 638 identifies a generic continuation requiring an entry slice-storage
frame before that frame's physical restoration has run. Reordering those two
complete applications in 639 clears that error but fails the caller's final
resource reservation: available entry-snapshot resources do not establish
resources for the current slice snapshot. Negative 640 fails at the same earlier
obligation and does not qualify. Neither ordering nor capture witnesses alone
complete this ownership transition.

The next transport experiment must retain the current nested resources together
with their entry-restoration continuation, binding parent-storage observations
at the appropriate restoration stage. It must preserve early owner reuse and
later reassignment; assuming current storage equals entry storage would bypass
the problem. Start with complete positive/negative Viper programs and checked
callee definitions before changing the general encoder.

The implementation is saved and pushed on the temporary recovery branch
[`stdlib/stabilization-checkpoint`](https://github.com/LrWm3/prusti-stdlib/tree/stdlib/stabilization-checkpoint):
`266391deb` records candidate 628 and selected-element fixtures, `83e0499f3`
records the projected-field repair, and `0a0144c75` records delayed scalar
wrapper conversion. Each commit's encoder, adapter and PCG contents match its
frozen tested candidate exactly. These are recovery commits, not a declaration
that combined stabilization is complete. Broader run 643 tests all nine unresolved
families (170 checks) on the exact 637 compiler. Regression pair 644 additionally
checks a symbolic index and projected reference field on that same compiler.
These runs must finish before integration decisions. All evidence remains under
`target/stdlib-matrix-20261031-1236`. The mandatory pause below is unchanged.

The suffix mismatch is now localized structurally: its quantified old
heap-dependent point observation retains `old(...)` in the callee wand and
`old[pre0](...)` in the caller wand. Their shapes differ even when the other
capture witnesses agree. Full-program experiments 645/646 replace those four
observations with indexing into an old whole-sequence resource-state snapshot;
the existing checked sequence-observer postcondition supplies the pointwise
law. Contract, package and application shapes then agree exactly. Native Rust
candidate 647 expresses the same entry capture explicitly in the checked
generic suffix pledge, with selected-value, untouched-value, postcondition and
missing-frame negative controls. All five encodings and structural checks pass;
Ferrum proofs remain queued. Neither the original failed fixture nor any
original full-program obligation has been discarded.

Candidate 648 separately extends checked equal-key transfer frames through
conditional enum branches, retaining both branch guards. The old traversal
handled implications but skipped conditional resource expressions. Its own
Prusti rebuild, 12 encodings and 12 structural checks pass; the native optional
slice, metadata, enum, region and small ownership controls remain pending. This
is not yet a qualified repair of the current-versus-entry ownership transition.

For the remaining generic split-owner composition, small proof pair 649/650
tests a different interface before any encoder change: two independently
packaged continuations each consume and restore half a resource permission.
During final restoration, one half can retain observations while the other
continuation runs, allowing checked overlap facts before full ownership is
reassembled. The model includes composition, all four cross-owner alias cases,
and an inhabited construction with an exact final-false control. It does not
split an existing magic wand. Both complete programs pass structural checks;
the positive passes Ferrum in 0.35 seconds and the negative fails only at its
intended final assertion in 0.20 seconds. Exact source hashes and diagnostics
are audited. The two models are preserved in the iterator suite's
[`known_limits`](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/README.md).
Complete-program experiment 654/655 now changes the scalar and opaque
continuation interface to paired half-resource continuations and supplies
checked concrete composition/coherence helpers. The original Rust bodies,
client obligations and concrete final restoration contract remain; only that
contract's restoration proof is replaced. Both full-program structural checks
pass. The complete positive passes Ferrum in **4.44 seconds**; the negative
fails only at the intended final assertion in **4.43 seconds**. Exact source
hashes and diagnostics are audited in `audit654-655.json`. The smaller 652/653
control also passes using the existing full-permission spelling of snapshot
function reads, so the raw proof needs no observer-interface change.

Next implement the paired protocol consistently for ordinary opaque transfer
wands as well as owner handoffs, retained call obligations and loop summaries.
Boundary conversions run once per logical stage. Compose both halves of the
complete history before concrete restoration: restoring each full step
separately can lose an overlap between the final and original owners that is
absent from an intermediate owner. Generate checked partial-resource coherence
helpers from known layouts, retaining complete keys and actual field paths.
First qualify the native failing positive/negative pair, then all owner,
replacement and generic loop controls. Detailed checked-program derivation and
integration constraints are retained in
`target/stdlib-matrix-20261031-1236/half-resource-integration-notes.md`.
The paired interface is now implemented experimentally in candidate 656 as a
shared final VIR lowering. It rewrites flat scalar/opaque continuations in
contracts, invariants, packages and applications together. Full applications
consume two instances; a half-package proof consumes one. Boundary conversions
remain outside those applications. Conditional/quantified footprints retain
their existing protocol, and conversions requiring full resources keep their
original checked preconditions. No arbitrary existing full wand is split.
The fresh own-branch build passes four encodings and four structural checks.
Its first native gate is audited at **5/7**: the ordinary concrete positive and
exact negative pass, while both generic split-owner cases retain the original
value-frame failure. Pairing alone is insufficient.

Complete-program pair 657/658 inlines the already checked 654 helpers. The
positive passes in **7.99 seconds** and the negative fails only at the final
false assertion in **6.53 seconds**; all methods and original contracts remain
checked. Candidate 659 uses this derivation to compose linear opaque histories
before concrete restoration. Existing checked boundary folds/unfolds expose
the two half footprints; complete-key guards preserve overlapping scalar
observations. The original final wand, including its value frames and pledges,
remains the proof obligation. Four native encodings and structural checks pass.
The complete native focused gate is now audited at **7/7**: the generic
split-owner positive passes in **4.32 seconds**, its exact negative in
**4.26 seconds**, and the ordinary concrete ownership pair also passes.
Candidate 660 adds focused linearity and branch
edge tests and avoids lowering branch-edge package expressions before their
permission context is known. Its own rebuild, three protocol unit tests and
four encodings/structural checks pass. The full owner gate is **74/75**: the
generic split pair is repaired, but `owner_handoff_loop_before_after_pass`
regresses with a missing accumulated summary after the loop. The existing
false-postcondition control requires at least one iteration and passes; fresh
zero-iteration positive/negative and nonzero positive controls are queued in
663. Unedited complete-program replay 662 reproduces the missing summary at
1957.9–1988.66 in 1.97 seconds. Checked identity shortcuts are being tested in
664/665. No failure is waived, and neither candidate is adopted on main.
The implementation is preserved and pushed in recovery commits `20a9c32cf`
(shared lowering), `34afb19df` (composition/coherence) and `89bd3eaed`
(branch-edge fix and tests). All code bytes match frozen candidate 660.

The distinction matters: [Dardinier et al.'s CAV 2022
paper](https://dardinier.me/papers/CAV22_Wands.pdf) shows that fractions of
arbitrary magic wands cannot in general be freely recombined. No such operation
or additional axiom is proposed here.

Run 643 and its owner retry 651 are complete and audited at **157/170**
checks on the exact frozen own637 compiler. Coverage and source/build
provenance are complete: owned iterator 30/32, owner handoff 73/75, generic
collections 10/10, prefix library/clients 11/12, nested references 9/11,
whole-region restoration 3/7, enum 7/8, metadata 7/7 and further prefix
controls 7/8. No negative was unexpectedly accepted. The original owner
setup failure (omitted fixtures) remains recorded; retry 651 supplies the
complete 75-check inventory in a fresh namespace.

The extra-next positive and nonempty wrong-result control still fail indexed
permissions. The prefix library and early-return prefix positive time out.
The nested repeated caller fails, and its false-postcondition control times
out. Whole-region positives and negatives retain indexed permission errors;
the enum replacement positive and generic split-owner pair remain failures
on this compiler. Candidate 660 repairs the split pair but has the separate
loop regression described above; results from the two compilers are not
combined into an acceptance claim.

The new zero-iteration controls in 663 are audited at **4/6**: the nonzero
positive passes, but both zero-iteration proof cases fail before the intended
assertion. The complete checked identity-shortcut programs 664/665 still
fail to find the accumulated summary. Own compiler 666 builds and passes
seven encoding/structural checks, but its native qualification was skipped
because that prerequisite failed. It is not adopted. Diagnostic 673 checks
summary availability across the pre-loop composition and post-loop call.

The separately verified composition helper in complete-program pair 678/679
passes the positive and rejects only the final false assertion. Native candidate
680 implements that derivation for whole opaque resource groups. It passes
seven encodings/structural checks, **12/12 focused checks** and **80/80 full
ownership checks**, all audited for coverage and provenance. The helper accepts
immutable snapshot, type and metadata keys for each stage; its body proves the
new paired continuation and all original equal-key value frames. Other resource
shapes retain their checked proof path. The own-branch source hash is
`a1254b3d…`, driver hash `ce988621…`, with unchanged candidate PCG `791e4cae`.
No Ferrum change or additional axiom is involved.

Recovery commit `f4a938990` preserves all 53 ownership fixtures; `d7cb048e5`
contains the helper implementation, byte-matched to frozen build 680. Neither
is adopted on main yet. The associated-output gate then exposed a physical-storage regression:
680 passes only 2/6 first-element checks because a fractional physical
continuation cannot satisfy a full slice-conversion contract. No failure is
waived. Candidate 681 restricts pairing to opaque indirect histories and keeps
physical continuations full. Its own rebuild, three protocol unit tests and
11 encodings/structural checks pass; the audited native gates are now **12/12
focused, 80/80 ownership, 6/6 first-element, 23/23 small controls and
6/6 replacement checks** (127/127 across these gates, with the focused
ownership cases also included in the full group). Recovery commit `7a5c8d600`
preserves this restriction and matches frozen source `a6e9e1a8…` / driver
`0808b5e5…`. Main adoption and combined qualification remain outstanding. The projected/dynamic
replacement pair separately passes an audited 4/4 checks on frozen compiler 637.
The explicit whole-state suffix capture in 647 passes only 4/7 checks; both
raw whole-state diagnostics 645/646 fail an indexed-resource precondition.
These failures remain open. Existing nested-reference cases are prepared for
a serial rerun on the exact 681 compiler.

Further serial Ferrum work covers projected/dynamic indexed replacement,
whole-state suffix captures, guarded optional resource frames, and the
remaining prefix capture regression. Each diagnostic retains the original
contracts; a raw Viper proof or encoding success is not native qualification.
One verifier and disabled Ferrum branch parallelism remain required. Finish
these existing restoration/collection repairs and the combined qualification,
then pause before D1–D6 and automation scheduling.


The call-boundary repair in 685 moves the caller's old-state label after
resource packing and before physical argument casts. Full-program diagnostic
683 verifies in 4.70 seconds; 684 rejects only its final false assertion.
The rebuilt compiler passes **7/7 whole-state suffix checks**, while the
original pointwise spelling still fails two of six checks. Those failures
are retained. Quantified old point observations have different continuation
shapes at definition and call sites.

Candidate 686 lowers canonical old sequence-resource observations at a bound
quantifier index to indexing the old sequence of states. The sequence observer's
full permission precondition remains checked; the lowering adds no axiom or
assumed frame. Other observation shapes keep their existing encoding.
Both original and explicit whole-state suffix forms now pass **7/7 checks
each**, including mutated-value, untouched-value, false-postcondition and
missing-frame controls. The focused ownership gate passes **12/12** and
first-element compatibility passes **6/6**, all audited. The full ownership gate subsequently passes **80/80** and small controls
**23/23**. The generic cross-crate gate fails at **9/10**: its specification
library reports a loop-restoration error, while all client/runtime/preflight
checks pass. Thus 686 finishes at **144/145 attempted checks**; metadata is
skipped. A progress update that inferred library success from the clients
continuing was explicitly corrected after audit. Eight encodings/full AST
checks and the three protocol unit tests pass.

Recovery commits `11407f764` (call state), `0b0e730d3` (quantified observation)
and `a214da3fd` (eight suffix fixtures) are pushed. All recovery encoder/adapter/VIR
bytes match frozen own686 source `54b5743b…`, driver `48923708…`.
Broader candidate 687 reuses these exact binaries for the remaining existing
collection/restoration families after the serial prefix diagnostics and 682
nested-reference comparison. Guarded-frame candidate 648 is separately audited
at **53/60**; its optional and whole-region failures remain unresolved.
No new compiler adoption or full stabilization is claimed.

Candidate 688 narrows the quantified old-point lowering to expiry pledges,
where continuation matching requires it, and retains the previous pointwise
encoding for ordinary postconditions. Its own rebuild and protocol unit tests
pass; source `c4f63d41…`, driver `33851229…`. The generic library now passes
in 160.57 seconds and the complete generic group passes **10/10**. Both suffix
forms retain **14/14**, focused ownership **12/12**, first-element **6/6** and
small controls **23/23**. Recovery commit `229c56f8b` matches all 20 changed
encoder/adapter/VIR files. After removing comments, blank lines and generated
crate disambiguators only, its entire generic-library Viper program equals the
previously passing 643 program.

Metadata then passes only **6/7**: `mut_take_slice_pass` fails the aggregate
unpacking helper's old-state equality postcondition. Candidate 688 therefore
finishes at **71/72 attempted checks**, not full qualification. The helper is
unchanged from passing 643; the complete normalized metadata program differs
only by the call-state label placement in another method. Full-program 690
adds an explicit assertion of the original postcondition immediately before
return. That assertion passes, but return checking still fails the identical
postcondition (405.89 seconds); its negative 691 is skipped. Diagnostic 692
reveals the old observer's checked definition at the added assertion and is
in progress. This is not yet evidence of a backend defect.

Prepared broader runs 687 and 689 will be skipped because their respective
686 and 688 core gates failed; encoding and structural success do not count as
verification. Full-program prefix diagnostics 667/668 time out at 600 seconds.
Diagnostic 671, which inlines only two unconditional final-package snapshot
captures while preserving guarded captures, passes in 307.64 seconds. Its
negative control 672 remains pending behind the metadata investigation.
Neither compiler repair is adopted on main, and the mandatory stabilization
pause remains ahead of D1–D6.

The metadata reveal probe 692 also fails the original return contract
(529.44 seconds). The added final body assertion passes; no backend defect or
repair is established. Probes 695/696 next reveal the same checked observer
in the return contract itself, retaining all logical postconditions and methods.
Both full AST checks pass; verification is queued after the existing nested
comparison 682.

Prefix negative 672 reaches the 600-second backend timeout (605.38 seconds
including cleanup), so 671/672 is **not a qualified positive/negative pair**.
The first compiler prototype 694 builds and passes nine structural checks but
leaves the relevant captures unchanged: its eligibility test omitted indexed
physical resources. Candidate 697 adds that case while retaining captures for
guarded reads and scalar/mixed storage stages. Its own build, three protocol
unit tests, nine encodings and nine complete AST checks pass. Inspection confirms
that the two intended final-package captures are now inlined. Frozen source is
`b1d476cc…`, driver `3dfa15a2…`. Its Rust positives and exact negatives form a
new focused gate on the later 688 base; they do not waive the older raw timeout.
Further regression groups stop if this native gate fails. No main adoption or
stabilization completion is claimed.

The exact-681 nested comparison 682 finishes at **9/11 audited checks**.
`mut_element_nested_call_pass` and its false-postcondition control both fail
with `apply.failed:wand.not.found`; the latter does not count as a valid
negative. The simpler nested proof and the other negative controls pass.
Candidate 697 also passes full encoding/AST checks for this repeated-call pair,
bringing its structural total to eleven; native verification remains pending.
Its source checkpoint is pushed as `faab3f3c0`, still unadopted on main.

Metadata fallback 698/699 reorders the same aggregate-unpack postconditions to
output permissions, whole-state equality, typing and pointwise framing. Both
full AST checks pass. It runs only if the contract-reveal positive 695 fails,
inside the same single-worker queue. No assertion or logical postcondition is
removed. Diagnostic caller-only programs 700/701 are prepared for fault
localization if needed; they are not queued and cannot qualify a feature.

The metadata contract-reveal probe 695 passes the complete program in
245.94 seconds. Its sole-false control 696 rejects exactly the final assertion
at line 2954 in 334.18 seconds; both source hashes and diagnostics are audited.
Fallbacks 698/699 are not run. Compiler candidate 702 implements the reveal in
aggregate-unpack return frames, retaining the original logical contracts. Its
entire metadata program equals 695 after normalizing comments, whitespace and
generated crate IDs. Own source `eb3cedb8…`, driver `21f8b7e8…`, three protocol
unit tests and nineteen encodings/full AST checks pass. One encoding runner
entry initially named a nonexistent fixture; that failed setup record is kept,
the name is corrected, and only that entry is re-encoded. Recovery commit
`4f472b770` contains the exact change; native metadata qualification is pending.

The native indexed-capture candidate 697 now passes **8/8 heavy-prefix checks**
and **5/5 guarded optional-slice checks**, both audited with complete coverage
and provenance. Early-break and generic prefix positives take 315.23 and
183.85 seconds; the three intended negatives take 430.05, 145.02 and 185.71
seconds. The prior raw672 timeout remains failed; these are fresh native Rust
results on the later candidate. Repeated nested calls, suffix compatibility,
cross-crate prefix/generic libraries and ownership regressions remain in the
serial gate. Candidate 702 follows with its combined native checks. The
caller-only 700/701 diagnostics run in 702's serial prelude and cannot establish
whole-program qualification. Main adoption and the final combined matrix remain
blocked on the outstanding gates and existing repair families.

Candidate 697 finishes with **22/24 audited checks**: heavy-prefix 8/8,
guarded optional 5/5 and nested 9/11. The repeated nested positive and its
false-postcondition control both fail a final physical value-frame package;
the negative is not accepted. Later groups are skipped. Caller-only
diagnostics 700/701 respectively time out and fail an earlier wand-availability
assertion; neither qualifies the complete program or establishes a backend bug.

Candidate 702 finishes metadata at **6/7 audited checks**. The native slice
positive still fails the aggregate-unpack return equality, despite raw 695/696
passing. Its later groups are skipped. Thus the reveal change remains an
unadopted experiment. The unchanged-contract ordering fallback 698/699 is now
being tested separately; the previous raw success does not waive this failure.

Checked models 703–707 pass all five expected outcomes: physical snapshot
coherence across two half-permission indexed wrappers, a two-permutation
restoration composition, two sole-false controls and a missing-intermediate-frame
control that fails the intended final package. Their sources are preserved in
`stdlib_iterator_spec/known_limits/indexed_physical_*.vpr`; these are finite
models, not native or unbounded qualification. Own compiler candidate 708
strengthens checked physical transfer frames to compare independent source and
target indices with equal underlying address/type. Permission keys and guards
remain unchanged. Build and three protocol unit tests pass; generated-program
checks and the native regression gate remain pending. No main adoption or
stabilization completion is claimed.

Metadata ordering probes 698/699 both finish with their expected outcomes:
the complete positive passes in 323.74 seconds, and the sole-false control
fails only at line 2954 in 361.17 seconds. Source hashes and exact diagnostics
are audited. Candidate 710 removes the unqualified old-observer reveal and
orders aggregate conversion return obligations as permissions, whole-state
frame, typing and pointwise frame. It applies this order to packing and
unpacking, retaining every logical obligation. Its own build, three protocol
unit tests, seven encodings and seven full AST checks pass (source
`aba93ba8…`, driver `721d4020…`). Native metadata, generic/prefix libraries and
ownership checks remain pending; raw success is not native qualification.
The single-worker queue first runs candidate 708's indexed-frame gate and
diagnostic 709 over the existing optional-owner, region, enum and extra-next
families on exact compiler 702. These independent diagnostics retain all known
failures and cannot establish combined qualification. Ferrum's remote main is
unchanged at `a2eec9a` when checked at 09:46 UTC; there is no newer runtime pin
to adopt at this point.

Candidate 708 remains unqualified: the repeated nested-call positive reaches
the 600-second verifier timeout (607.39 seconds including the command). The
simpler positive then runs substantially longer than its earlier 31-second
baseline. The candidate is stopped at that point; its partial audit contains
three preflight/runtime successes and the failed repeated positive, with seven
missing native checks. The interrupted simple positive and remaining negatives
are not counted as successes. The owned verifier process tree is stopped before
releasing the serial worker; `probe708/interrupted.json` records the action.
The stronger cross-index experiment is not included in subsequent candidates.

Diagnostic 709 uses the exact own compiler 702. Its optional-owner group is
**8/9 audited checks** in 39.07 seconds; `mut_option_generic_len_pass` still
fails with `apply.failed:insufficient.permission` at the length call. Region,
enum and extra-next diagnostics remain pending. An initial runner indentation
error started no verifier and is archived before correction. The finite
partial-heap model 711 fails return/application wand matching; its controls
are skipped. Revised models 716–719 bind old and current observations outside
the wand. Their corrected full AST checks pass, but verification is pending;
the initial let-syntax parse failure is retained separately.

Candidate 715 follows 710 with a smaller indexed-call change: materialization
inlines only unconditional old `p_Param_snap` bindings when both sides of the
restoration contain exclusively indexed resources. Final packages already use
that policy. Guarded/deep observers and mixed/scalar stages retain captures;
permission operands and value obligations are unchanged. Its own build, three
protocol unit tests, seven encodings and seven complete AST checks pass
(source `c645e935…`, driver `67e03d14…`). Native tests are queued after 710;
this remains an unadopted experiment. Stabilization and the mandatory pause
have not yet been reached.

The refreshed 709 region and enum groups are **3/7** and **7/8 audited checks**.
Both region positives and both wrong-value controls fail on indexed generic
permissions; enum replacement still fails an indexed permission assertion.
The extra-next positive now passes in 375.49 seconds on own compiler 702,
where the older combined baseline failed. Its three negative controls remain
pending, so the family is not yet marked repaired.

The next current-owner design must distinguish an ancestor's entry state from
its restored descendant's current state. For `Option<&mut [T]>`, the original
slice can still require restoration when the current option is `None`.
Observing only the current option loses that owner; freezing every dereference
instead loses the slice's current elements. The proposed scoped policy freezes
the ancestor slot at entry and observes the restored slice storage now, retaining
the original variant guard. It must be shared by predicate construction and
generic boundary conversions, including their physical casts. Merely changing
`is_handoff_node` or adding the ancestor as a physical frame is insufficient.

Before integrating that policy, qualify the explicitly captured model
716–719, including false, wrong-current-value and missing-current-resource
controls. Then select a unique restored physical owner and carry its guard,
origin and group consistently through both callee packaging and caller
continuation histories. Bundling must recognize resource-free return types
such as `usize`, while retaining every parent-frame obligation. Existing
optional-owner positives/negatives and new mutation/replacement controls must
pass before widening to escaped outputs or multiple physical owners. This is
an unimplemented repair plan within the existing stabilization scope, not D1.

Full-program diagnostics 720/721 preserve the current enum positive and add
only one final false assertion for the negative. Both AST checks pass. They
test `--separateMethodSolver` with the existing single-worker Ferrum pin,
without method filtering, as candidate 715's serial prelude after 710. The
Rust negative can also differ in cleanup/control flow, so its earlier success
does not establish a solver defect. No backend setting is adopted from these
pending diagnostics.

Research review for the remaining restoration work reinforces two constraints.
[Dardinier et al., *Sound Automation of Magic Wands* (CAV 2022), Section 4](https://dardinier.me/papers/CAV22_Wands.pdf)
shows why arbitrary fractional wands cannot generally be recombined. Our
application of that result is to retain separately checked ordinary packages
over half-resource operands, rather than split an existing wand or add a
recombination axiom. The current checked models establish only their stated
resource and frame obligations.

[Becker, *Generalized Verification Support for Magic Wands* (2017), Sections 4.3–4.4](https://ethz.ch/content/dam/ethz/special-interest/infk/chair-program-method/pm/documents/Education/Theses/Nils_Becker_BA_report.pdf)
describes lost value information for inhaled wands and explicit `old[lhs]`
frames as one workaround; it also describes observing a wand through an
`applying` expression in loop invariants. This supports investigating checked
frame/coherence obligations and stable observation captures. It does not
establish that the current backend has the historical implementation limitation,
nor justify weakening any current permission check. Any use of an `applying`
observation must itself prove the required antecedent and match the retained
wand; no such new encoder change is adopted here.

Diagnostic 709 completes at **23/30 audited checks**: optional-owner 8/9,
region 3/7, enum 7/8 and extra-next 5/6. The extra-next positive and both
wrong-result controls pass (375.49, 151.27 and 166.93 seconds), but its false
control fails an earlier generic permission application in 490.97 seconds.
This is not an accepted negative or a repaired family. All selected coverage
and source/build provenance checks pass; the seven proof failures remain open.

Models 716–719 now pass **all four audited outcomes** with explicitly captured
old/current values: the positive succeeds and the controls fail only at the
intended false assertion, wrong current value and missing current resource.
Sources are preserved as `known_limits/owner_handoff_entry_ancestor*.vpr`.
This qualifies the finite proof pattern for the scoped plan above, not its
Rust implementation. The boundary monitor has resumed the native 710 gate;
there are no deliberately stopped controllers at this checkpoint. Candidate
715 remains queued afterward, including its full-program 720/721 prelude.

Candidate 710 completes at **16/17 audited checks**: metadata 7/7 and generic
9/10. The generic specification library fails loop restoration; later groups
are skipped. Comparing its complete program with passing 688 identifies only
the final-package snapshot inlining introduced by 697. Candidate 722 restricts
that optimization to indexed physical histories on both sides of the wand and
retains captures for generic-resource-only histories. Its own build, three
protocol tests, five encodings and five full AST checks pass (source
`023cb374…`, driver `690e5fef…`). Its generic library program matches 688 after
removing only comments, blank lines and generated crate identifiers. Fresh
native verification remains required; textual equality is not qualification.

The full-program solver-context diagnostic 720 fails an enum permission
assertion in 18.19 seconds; 721 is skipped. No backend setting is changed.
Candidate 715 also fails the repeated nested-call positive with a missing-wand
assertion; its remaining controls are still running. Neither experiment is
adopted. The serial queue next runs 722's generic, metadata, heavy-prefix,
prefix-library, owner and suffix gates, stopping broadening on failure.

Candidate 723 implements the scoped optional-slice prototype on 722. Predicate
construction and boundary conversion share a reference-depth policy: observe
the ancestor option at entry, then the restored slice currently. The joint
physical/generic stage retains all parent frames and the complete entry-resource
continuation. Primitive resource-free returns are included only for this
optional-slice shape. Caller history groups are rebased through the actual
opaque Rust type when the group is unique; runtime owner address/type checks
and checked continuation applications remain. No trusted axiom or permission
assumption is added. Its own build and three protocol tests pass (source
`748d5824…`, driver `f507c8ea…`); encoding and structural checks precede the native
optional-owner gate. New controls clear the option, rotate reference-containing
slice elements and reject false or wrong-current-value assertions. This remains
an unqualified prototype, recorded separately from main. Recovery commits
`2d7088fa6` and `07a562398` preserve the exact 722 and 723 compiler sources.

Candidate 723's seven encodings and seven complete AST checks now pass.
The first version of four new controls used an unsupported mutable-reference
dereference inside a specification expression. Those failed encodings and
sources are archived; the corrected controls read the value in ordinary Rust
before asserting it. No verifier was run on the failed fixtures. Recovery
`fba986054` preserves the corrected tests. Native qualification is still queued.

The enum failure has a separate concrete hypothesis: scalar resource views
handle structs and direct mutable references, but omit enum payloads. Candidate
724 adds payload views guarded by the corresponding discriminant. A wrapper is
opened only when its permission is held and the raw scalar permission is absent;
the existing checked reclosure remains. This changes proof operations, not the
required resource contract. It is based on 722 independently of the optional-owner
prototype. Its own build, three protocol tests, five encodings and five full AST
checks pass (source `c92aa8fd…`, driver `8106c22c…`). The native enum group includes
a new wrong-current-value replacement control before scalar regressions.
Recovery `dce156794` is the exact independent compiler checkpoint; `443affe15`
adds that control. The 723 prototype remains preserved at `07a562398`; these
candidates have not been combined or adopted into main.

Candidate 715's repeated positive fails in 227.60 seconds with a missing-wand
assertion; its postcondition-false control reaches the 600-second timeout
(602.12 seconds including the command). Neither is a passing check. The remaining
controls are still running. The serial queue is 722, then 723, then 724, retaining
one verifier worker and no branch parallelism. No Silicon run is needed for
these individual repairs; combined major-milestone qualification remains open.

Candidate 715 finishes naturally before its proposed stop can take effect.
Both stop attempts fail their active-case guard before sending any signal.
The complete audit is **9/11**, with only the repeated positive and
postcondition-false control failing; later groups are skipped. Candidate 722
then passes **all 10 generic-library checks**, including the complete library
(173.58 seconds), clients and exact negatives. Coverage and source/build
provenance are audited. Its metadata, prefix and ownership gates remain in
progress; this repairs the isolated generic capture regression without claiming
combined stabilization or reaching the requested pause point.

Candidate 722 also completes metadata **7/7** and focused prefix **8/8**, giving
**25/25 audited checks** across its first three groups. The prefix positives
take 208.35 and 386.20 seconds; both false controls and the wrong-state control
produce their exact expected diagnostics (304.84, 173.86 and 241.98 seconds).
The exported prefix library, owner and suffix groups remain pending. This
combines the metadata ordering repair with the scoped capture policy on one
own-built compiler; it is still not a complete stabilization or matrix result.

The two remaining region positives report the same missing indexed tail
resource in the shared generic `next` definition, at native position ID 573.
Diagnostic 725 preserves the complete 709 program unchanged and adds only
failure reporting to locate its exact source operation. Its full AST check
passes; it is queued after 724, with no method filter or backend-setting change.
This replay is for fault localization, not qualification.

Candidate 722 completes its selected native gate at **129/129 audited checks**:
generic library 10, metadata 7, focused prefix 8, exported prefix library 12,
owner histories 80 and combined suffix restoration 12. The exported prefix
library passes in 278.82 seconds. All selected coverage, source/build provenance
and exact negative diagnostics match. The combined suffix selection has 12
checks; adding the earlier separate seven-check runs would duplicate common
checks. The exact compiler is retained at recovery `2d7088fa6`.

This qualifies the selected core regressions on one own-built compiler, not the
full stdlib matrix or the remaining optional-owner, enum, nested-call, region
and extra-next cases. Native 723 now tests optional-owner restoration, followed
by its clearing/rotation controls if the existing group passes. Candidate 724
and diagnostic 725 remain serially queued. The mandatory pause before D1–D6
has not yet been reached.

Candidate 723 completes at **3/9 audited checks**: all six native optional-owner
cases fail while packaging the current-resource reservation. Its later clearing,
rotation and full-owner groups are skipped. Candidate 724 completes at **8/9**:
the enum replacement positive advances past the initial packing failure but
fails a later physical scalar conversion. All four exact negative controls,
including the new wrong-current-value control, pass. Neither candidate is
adopted; their original failures and complete provenance remain recorded.

Full-program diagnostic 725 locates the region failure at the checked indexed
tail-resource assertion, lines 4325–4346, in the shared generic `next` definition.
Diagnostic 726 locates the enum failure at the historical replacement-reference
conversion, lines 8548–8549. Both preserve the original complete programs and
fail; they are localization evidence, not passing qualification.

The outer-wand model exposes a missing obligation in 723's proposed contract.
A future caller can change the owner pointer, so the outer antecedent must supply
the current resource together with a checked current-to-post history. Model 731
composes that history with the saved post-to-entry history and checks explicit
physical and current-value frames. Its positive and false, missing-history and
wrong-current-value controls all match. Models 727 and 728 retain the exact
missing-current and missing-frame failures. All six outcomes and full AST checks
are audited in `audit727-734.json`; byte-exact models are committed under the
iterator suite's `known_limits`. This finite proof guides the next encoder
revision but does not establish native optional-owner support.

Candidate 735 is independently based on 724. It defers scalar reclosure past
bookkeeping-only removed assignment and aggregate edges; actual array repacks,
unsize conversions and existing call/phase boundaries still close wrappers.
Its own build and three protocol tests pass. Fresh encoding, complete AST and
native enum/scalar regression checks are required before adoption. Restoration,
combined qualification and the mandatory pause remain the active scope.

Candidates 735, 737, 738 and 741 each complete the enum gate at **8/9**,
with the same replacement positive still failing and all four exact negative
controls passing. Later scalar groups are skipped. The checked wrapper exposure
in 738 advances through the physical permission obligations to the final
cross-indexed/raw value frame. Extending snapshot inlining in 741 does not solve
that frame. These are preserved recovery checkpoints, not accepted repairs.
Candidate 747 instead tests checked observer reveals at enum/reference resource
packing and unpacking on the 738 base; its completed native gate is also 8/9.

The exact optional replay 739 corrects an earlier shorthand diagnosis: the first
failure is missing **physical slice ownership**, at lines 2208–2253, because the
dependent generic stage precedes its physical stage. Diagnostic 740 moves only
the existing physical application ahead of it. That passes the first operation
but fails the caller's current-resource reservation at lines 2415–2428. Neither
run changes contracts or removes obligations; order alone is insufficient.

Models 742–745 check a narrower optional-owner protocol. Its outer antecedent
requires the physical owner to agree with the captured post-return resource
footprint; referent values may still change. The positive, false assertion,
wrong value and changed-owner controls all match with full AST checks, and the
byte-exact models are preserved under `known_limits`. This is an explicit,
checked expiry requirement, not a general replacement for caller histories.
Native 746 applies it only to the existing optional-slice shape with no
resource-bearing output. Its build, three unit checks, seven encodings and
seven AST checks pass, but native qualification is only **5/9**: both positives
and their assertion-false controls still fail. Both false-postcondition controls
now produce the exact expected diagnostics. The clearing/rotation and full-owner
groups are skipped. Diagnostic 748 locates the remaining observation error in
the complete direct client before another revision. No candidate is adopted.

Candidate 749 moves the guarded historical entry observations outside both
restoration packages. The direct optional client then passes (22.57 seconds),
as do all four exact negatives; only the generic wrapper fails its final value
frame. The complete gate is **8/9 audited**. Inspection shows that the returned
conditional history has no value frames because `transfer_frames` does not
traverse conditional expressions.

Candidate 751 adds correspondence for conditional resources with stable guards,
retaining both discriminants, quantifier ranges and complete resource keys.
Its own build, three protocol tests, seven encodings and seven AST checks pass.
The existing optional gate now passes **9/9 audited checks**, including the
direct positive (19.17 seconds), generic wrapper (4.42 seconds) and all four
exact negatives. The additional clearing/rotation gate subsequently finishes
at **3/7 audited checks**: all four native controls fail; its two runtime
checks and Rust preflight pass. The full owner-history gate is skipped.
Exact recovery `6c66c7425` is pushed; main has not adopted this encoder
candidate and combined qualification remains open.

The enum candidates 747 and 750 both remain **8/9 audited**. Observer reveals
and a direct two-resource-permutation proof branch do not establish the final
physical value frame. Diagnostic 752's unguarded initial observation also fails:
its chosen expansion key is initialized only for an earlier selected variant.
Diagnostic 753 instead uses the actual argument index and checks the current
variant/address guard before observing that wrapper. It retains the entire
original program, final quantified frame and a checked scalar equality; it is
complete: its guarded scalar equality fails (33.19 seconds). Diagnostic 760
also fails the equality inside an actual final indexed wrapper, before
unfolding it. This narrows the search to earlier restoration steps; it does
not establish a backend defect. No enum repair is accepted yet.

Candidate 758 imports the original optional slice's guarded physical permission
with a checked assertion before observing its current snapshot. Full-program
diagnostic 755 shows this removes the missing snapshot permission. The own
compiler build, three protocol tests, seven encodings and seven AST checks
pass; native results are **9/9 existing optional checks and 3/7 new controls**,
with complete audited coverage. The original new controls still fail. Exact
recovery `433772796` is pushed; no encoder adoption is claimed.

Diagnostic 756 separates the caller's length failure: the optional `Some`
variant is known before the call, but its slice length is not. The test uses
array `IndexMut<RangeFull>`, which currently has no functional contract. The
pinned Rust implementation delegates array indexing to slices, but its generic
bound permits custom slice `IndexMut` implementations as well. Candidate 764
therefore attempts a conditional standard-`SliceIndex` contract, retaining the
exact target bounds and associated output. It builds and passes seven encoding
and AST checks, but inspection finds that existing conditional-call support
intentionally excludes external trait-implementation methods and refinements
with preconditions. The functional contract is not selected. Intermediate
759/761–763 builds were rejected before any proof run; no validator was bypassed.
Recovery `0a17900c8` preserves the unqualified contract experiment separately.

Additional direct-coercion controls (766, on the exact 758 compiler) isolate
optional owner restoration from this newly exposed array API gap. They retain
the count, cleared-slot and current-value assertions. The original RangeFull
fixtures and their failures remain intact; these extra controls cannot replace
them in a combined qualification claim. Candidate 764's array group is only
2/6: its bounds negative is incorrectly accepted because the conditional
contract is not selected. Its custom-index negative also needs the trait
refinement annotation; the corrected fixture is preserved in recovery
`4cef831a7` but has not been rerun. Neither result is waived. Completion of
restoration, existing generic collection/ownership stabilization and combined
qualification remains required before the user-requested pause.

The coercion diagnostics isolate two missing history-discharge points. Full
program 773 passes after packing the current concrete element resources,
applying the already-returned current-to-entry history, and unpacking the entry
resources immediately before the original local array operands' borrows expire.
The original caller retained that history but never consumed it. Applying the
same repair to rotation (775–777) removes the caller's permission failure but
leaves the callee's current-resource reservation failure.

In 778–780, moving the existing guarded swap-history assertion and application
inside the optional owner's current-to-entry continuation fixes that remaining
failure. The rotation positive passes in 31.97 seconds; the false and wrong-value
controls each fail solely at their intended assertion. All original contracts,
bodies and assertions remain. Together with 773 these four full-program
diagnostics pass their outcome, AST and source-hash audits. They motivate a
native encoder change, not a completed compiler qualification. Candidate 781
failed at compilation; candidate 782 corrects its tuple consumers and expression
comparison and tests the integrated scheduling and local-array discharge.

Candidate 782 completes 96 audited native Ferrum checks: optional coercion 7,
existing optional 9, and owner histories 80. Additional controls 785 then expose
two incomplete cases: repeated calls on one local array and calls on two
independent arrays. Both positives fail permission checks; both negatives still
fail exactly as intended. Matching a retained history by its entry sequence
does not reliably identify an owner whose contents have changed.

Candidate 788 instead matches the physical owner address, which array-to-slice
unsizing preserves, and applies that owner's histories newest first. It retains
the runtime consumed flag and all checked pack/apply/unpack operations. Its own
build, three unit tests, six encodings and six full AST checks pass. All 103 native
checks are audited passes: local history 7, optional coercion 7, existing optional
9 and owner histories 80. Recovery `577b418c0` preserves the encoder and
`912cac5b3` adds the four repeated/independent fixtures separately from main.

Enum diagnostics 772 and 774 remain unqualified. Checked physical coherence
helpers remove the previous final value failure, but reserving the immediately
returned indexed resources exposes a missing entry resource in the outer
package. Diagnostic 783 localizes that loss to untouched entries; the selected
entry restores. A separation helper using only abstract `p_Param` permissions
fails its own address-inequality assertion (784), and is rejected.

Full diagnostic 786 passes after that helper converts both concrete `u32`
resources to their actual field permissions, checks separation, and refolds and
converts back while preserving both snapshots. All original obligations and the
helper body are checked. This is not a distinct-address assumption for generic
types or zero-sized values. Diagnostics 789–791 test a smaller repair consisting
only of this checked initial separation and reservation of immediately returned
resources, including the original false and wrong-current controls.

The smaller diagnostic gate 789–791 passes its full-program outcome, AST and
source audits: positive 57.36 seconds, false-value control 23.58 seconds and
wrong-current control 24.46 seconds, each negative with its sole intended error.
It needs no experimental cross-index coherence helper.

Candidate 793 encodes the checked separation lemma from the existing indirect
lifetime projection, retaining enum guards and full index/path/type keys. It
selects concrete primitive referents with actual field permissions, unfolds and
refolds both resources, and proves their address inequality. The checked helper
preserves both snapshots. Generic and zero-sized referents receive no distinctness
assumption. The call runs at method entry; the final package reserves immediately
returned indexed resources when these separation calls are present.

The candidate combines this change with 788's local histories and the enum
exposure scheduling from 738. Its own build, three unit tests, eight encodings and
eight full AST checks pass. All 124 native checks are audited passes: enum 9,
first-element 6, replacement 6, local-history 7, optional coercion 7, original
optional 9 and owner histories 80. Recovery `1b6a6c44b` preserves the exact
compiler source. Broader 794 uses that same compiler. Generic resources 10 and
metadata 7 pass, including the specification library in 256.37 seconds. Its
early-break resource positive then times out at 600 seconds, whereas candidate
722 passed it in 208.35 seconds. The generic-prefix positive passes in 256.85
seconds; the remaining diagnostics are still running. This regression is not
waived, and there is no full-matrix or milestone sign-off.

Comparing the generated early-break programs isolates additional quantified
transfer-frame guarantees from optional results. Candidate 806 limits these
derived variant frames to restored variant-containing owners, retaining all
permissions, source pledges and non-variant frames. Its early-break program
matches 722 exactly after removing comments and normalizing generated crate
identifiers. This is diagnostic evidence only: the native heavy-prefix group
and enum/optional/ownership regressions must pass before adopting the change.

Array experiments 795 and 797 build and pass eight encoding and AST checks each,
but generated code still lacks the conditional bounds assertion and immediate
contract. Both are withdrawn before native verification. The investigation now
distinguishes the MIR trait declaration from the statically resolved array
implementation and preserves the implementation's own generic substitutions.
An experimental checked-precondition path and immediate-only array contract remain
unqualified; conditional multi-stage expiry is unchanged. Diagnostics 796 and 798
trace selection. The original RangeFull fixtures and out-of-bounds negative remain
required, and the earlier incorrectly accepted negative is not waived.

Diagnostic 798 confirms that the resolved array implementation's contract is
found, but selection rejects its `Index::Output == SliceIndex::Output` bound.
Candidate 800 normalizes both sides without erasing regions, rejecting unresolved
obligations and introduced region constraints. It builds and passes eight
encoding and full AST checks. The standard-index bounds assertions are now
present; the custom-index control does not receive them. Native qualification is
queued, and recovery `be9a91fac` preserves this unqualified candidate.

Two additional direct array controls cover reference-containing elements. The
projection checker must quantify lifetimes in generic arguments before their
substitution into both sides of the equality: repeated use of one type preserves
its lifetime identity, while separate arguments remain independent. Candidate 801
exposes an extra-binder encoding error, corrected in 802. Its ten encoding and AST
checks then pass, but a guard stops native verification because the reference
contract is still absent. Diagnostic 803 proves the output equality normalizes
without region constraints and identifies a nested trait-bound rejection instead.
Candidate 804's region-sensitive nested query remains insufficient; diagnostic
805 identifies `RangeFull: SliceIndex<[&mut u32]>` as the failing nested goal.
Candidate 807 tests continued selection in the original universal inference
context, with bounded work and all final lifetime-constraint checks retained.
The existing conditional and lifetime-negative controls must pass first. None
of these encoding-only results establishes array verification or conditional
multi-stage expiry support.

### Remaining restoration diagnostics and fixed-path frames (2026-11-02)

Broader native 794 finishes the heavy-prefix family at **7/8**: the early-break
positive reaches the 600-second limit, while the generic positive and every
negative control match. Prefix and suffix expansion are recorded as skipped,
not passed. Its region family remains **3/7**: both positives and both negative
controls encounter the same early missing tail-resource permission. In the
nested family, the repeated-call positive still fails wand matching and the
false-postcondition control times out; the simpler positive passes. These
failures prevent adoption and milestone sign-off.

Full-program diagnostic 809 adds one checked equality between the current
iterator-tail snapshot and the tail returned by `split_at_mut`, before the
restoration package. It retains all original obligations. Diagnostic 810 is an
unchanged full-program replay of the repeated nested caller, intended to recover
a precise failure position. Both pass structural checking and await serial
Ferrum execution. Earlier method-filtered successes remain diagnostic only.

Candidate 812 builds on 806 and omits derived frame clauses between indexed
resources whose fixed sequence paths are provably different. Those clauses have
false identity guards. All permissions, applicable frame guarantees and source
pledges remain. Its own Prusti build, three paired-resource unit checks, eight
Rust encodings and eight full-program structural checks pass. The complete
native nested family is queued after the unchanged diagnostic baseline. This is
a performance hypothesis, not a claimed restoration fix or qualified compiler.
Its source patch is `fc8ad28846388e52d89f6d36f44cfedac3515bc5b8e1506c91b202e6d6eab69c`;
the driver is `196dcaaa72be403b1fad8022f24c9c0790c317277c7e8413aab6092e348927e4`.

The queue remains one worker with branch parallelism disabled: complete 794,
then array candidate 800, diagnostics 809/810, focused 812, and candidates
806/807. Only sleeping controllers with no child processes were replaced to
reorder pending work; no active verification was interrupted. Routine checks
remain Ferrum-only. Complete and combine existing restoration/collection
repairs, qualify the combined source, then pause before D1–D6 and automation.

## Follow-on scope authorized on 2026-11-01

The user initially authorized the remaining IM and matrix repairs followed by
the deferred features below. Their later instruction on 2026-11-01 sets a
mandatory stopping point: **finish restoration and stabilize the existing
generic collection and ownership proofs, then pause and report back before
starting D1–D6.** They want to schedule automation work next. This supersedes
the earlier instruction to continue directly into the follow-on milestones;
it does not stop the current restoration and stabilization work.

At that stopping point, report the accepted commits and dependency pins,
validation results and any material limitations, with qualified changes
committed and pushed. Do not start new feature work or schedule automation
without the user's next instruction. The earlier sixteen-hour return estimate
is not a deadline for the current work.

Finish and combine the independently qualified current repairs first:
Ferrum upgrade, optional-slice conversion, owned iterator resources and loop
coupling, indexed enum snapshots, associated-output dependencies, and current
owner/entry owner handoff. Recheck every matrix failure with its required
positive and exact negative controls, then run the complete matrix on the
accepted combined source and freshly rebuilt Prusti. Preserve all failed and
superseded evidence. No pending candidate is accepted merely because it builds
or because a negative reports some error.

After that gate, pause. These milestones remain planned for a later resumption:

| Milestone | Implementation and proof target | Completion evidence |
|---|---|---|
| D1: nested and recursive ownership | Extend indexed resource paths, checked conversions and observations to nested arrays, tuples/options, `Vec<Vec<T>>`, owned pointers and finite recursive heap shapes, including reference-containing and generic elements. | Arbitrary-length nested traversals, replacement with escaped old references, untouched-region framing, generic/non-`Copy` cross-crate proofs, and exact alias/permission/value controls. No empty-resource shortcut based only on missing lifetime projections. |
| D2: yields retained across loop backedges | Represent an unbounded collection of live yielded references and its delayed source-restoration obligations; compose them with D1 ownership. | Collect and mutate retained yields, expire them in different orders, handle empty/partial/full traversal, and restore the source. Reject reuse while a conflicting yield remains live and incorrect restored values. |
| D3: callbacks, iterator adapters and collection | Model closure captures and callback effects, then compose `take`, `skip`, `enumerate`, `zip`, `map`, `filter` and `filter_map`; connect `collect` to Vec and the modeled hash collections. | Symbolic length/order/value or membership laws as appropriate, short-circuit and early-drop behavior, captured mutations and borrow expiry, cross-crate clients, and false/value/frame/permission controls. Checked Rust wrappers must establish composition beyond trusted API contracts. |
| D4: remaining hash APIs | Add vacant/occupied Entry operations and insertion/update callbacks, keys/values/shared and mutable iteration, owned iteration, and heterogeneous borrowed-key lookup with explicit compatible key laws. | Insertion/removal/update membership, cardinality and framing; iterator coverage without assuming a hash iteration order; mutable-value restoration; equivalent and inequivalent borrowed queries; collisions and exact negative controls. |
| D5: interior mutability | Extend heap-dependent observations and alias tracking for `Cell` and `RefCell`, including dynamic borrow guards and their release. Review the pinned Rust implementations before writing contracts. | Shared-alias visibility for get/set/replace/swap, successful and failed borrow attempts, simultaneous shared guards versus exclusive guards, guard expiry and mapping, cross-crate use, and rejection of stale snapshots or invalid borrow assumptions. Account explicitly for normal-return and panic behavior. |
| D6: combined qualification | Integrate the completed feature gates, update the support matrix and run all relevant suites with the final dependency pins. | Fresh full dual-backend verification, exact coverage and source/build provenance, runtime behavior checks where meaningful, and no unreviewed backend differences or deferred failures inside the claimed domains. |

Use primary research and pinned implementation sources when a milestone needs
a new ownership or effect model. Start with checked small models and failing
Rust reproducers, then extend to generic, unbounded and cross-crate proofs.
Keep one verifier worker and Ferrum branch parallelism off. Use Ferrum for
routine fixes and all their controls; use Silicon when signing off a completed
major feature and for final combined qualification. Commit and push working changes in logical
chunks. Any remaining restriction or genuine blocker must be recorded against
its milestone rather than silently narrowing the completion claim.

## Validation and delivery rules

### Evidence required for every gate

Record the revision, toolchain/backend versions, invocation, configuration,
expected result, actual result, and verifier time. Use the focused runner from
P1-A for positive, negative, overflow-enabled, and cross-crate cases. Record which
tests establish generic behavior and which are only concrete regressions.

A negative test must fail for the intended verification or diagnostic reason.
Crashes, unsupported-feature errors, timeouts, and `known-failure` annotations do
not count as rejection of an incorrect proof. A timeout also does not justify a
fixed-length fallback. Track timings against the current 180-second per-file
verifier timeout with one parallel verifier before changing any limits; the
earlier 90-second runs remain historical evidence.

For every blocker, retain a minimal reproducer and classify it as a specification,
resolution, snapshot, borrowing, trait-refinement, or solver-encoding issue. Record
the dependent gate and proposed fix. A milestone stays incomplete while one of its
required cases is blocked.

### Trusted contracts and execution assumptions

Review each trusted contract against the pinned Rust API and implementation, and
add executable behavior checks where practical. Check boundary cases, return
values, mutation of other arguments, and preserved regions. Client proofs and
negative tests are useful evidence but do not prove the trusted contracts sound.

The functional laws describe normal returns. Record allocation failure, capacity,
panic, and destructor assumptions separately, including applicable preconditions;
do not infer absence of those effects from a functional postcondition. Keep
overflow checks enabled for milestone clients. Full allocation, unwinding, and
arbitrary destructor verification are outside the initial milestone.

### Reviewable implementation sequence

Deliver environment/test wiring first, then external-specification fixes, model
and encoder support, and collection contracts with their proof clients. Keep
infrastructure fixes and contract additions in separate reviewable changes. Reuse
or adapt upstream work only after checking its fit with the recorded baseline and
running the relevant local probes.

Update this document's support matrix as evidence arrives. A public support claim
must name the supported operations, element/key domain, and completed proof gate.

## Dependency and support matrix

Baseline inspection, P1-A through P1-D, and P2-A are complete locally, including **M1**
for owned values with stable value snapshots. M2 and M3 also pass locally within
their documented domains. General outlives equivalence is conservatively
unsupported; a hosted CI run is not claimed.

| Gate or capability | Depends on | Expected Prusti work | Baseline status / completion criterion |
|---|---|---|---|
| Baseline inventory | Checkout `87487f774` | None | Inspection complete; no new proofs claimed. |
| P1-A: reproducible test execution | Setup, pinned submodules/toolchain | Focused harness and CI wiring | Verified locally; positive, negative, overflow, and cross-crate execution demonstrated. |
| P1-B: external-contract foundation | P1-A | Targeted rewriting/resolution/substitution/metadata fixes | Verified locally: 43 focused checks, bundled unwrap/replace, cross-crate and core-only clients, conservative lifetime matching, bounds, duplicates, and refinement. |
| P1-C: sequence observations | P1-B | Model connection, explicit triggers, required generic snapshots | Verified locally: 21 focused checks, arbitrary-index observations, explicit triggers, cross-crate proofs, and precise diagnostics. |
| **M1: unbounded functional Vec** | P1-A through P1-D | Minimal contracts plus reproduced blockers | Verified locally: 21 Vec checks, symbolic append/pop/round-trip and whole-prefix loop proofs for owned non-`Copy` elements, cross-crate clients, and negative cases. |
| P2-A: Vec and shared slices | M1 | Contracts, sequence lemmas, any slice encoding fixes | Verified locally: 27 checks cover symbolic mutations, shared lookup/ranges, splits, swaps, boundaries, ordering, and framing. |
| P2-B: mutable collection views | P2-A, two-state snapshots | Pledges, expiry, generic reference framing | Verified locally: 74 checks cover slice and Vec views, lookup, indexing, splits, reborrowing, inclusive range state, and negative framing cases. |
| P2-C: shared and owned iterators | Sequence model, generic trait probes | Opt-in models, refinement, lifetimes, loop support | Passed locally: 58 iterator checks plus focused symbolic early-break checks cover owned and shared traversal, exact lengths, refinement, and loops. |
| **M2: Vec/slice phase complete** | P2-A, P2-B, P2-C | Integration and cross-crate validation | Passed locally: component gates and the sequence, Vec, collection, borrow, external-spec, and iterator integration checks pass. |
| P3-A: explicit key models | M2, logical Map/Set | Key-model interfaces and conditional contracts | Passed locally: 20 checks cover projections, explicitly trusted key laws, collisions, and conditional empty map/set observations. |
| P3-B: core hash collection operations | P3-A, P2-B expiry support | Conditional map/set contracts and mutable value restoration | Passed locally within the modeled key domain, including generic, non-`Copy`, collision, and negative clients. |
| **M3: HashMap/HashSet phase complete** | P3-A, P3-B, P2-B expiry support | Map/set contracts and mutable lookup | Passed locally: 51 hash checks cover the modeled core operations and mutable lookup with explicit key laws and same-type queries. |
| `IterMut` | P2-B and iterator models | Yielded-reference tracking and owner restoration | Core contracts bundled: native construction, forward next, exact length, mutable-slice IntoIterator, simultaneous yields, and source restoration. Whole-prefix loop values and reference-containing elements remain outside this gate. |
| Functional `IterMut` loops (IM1–IM2) | Native core gate, IM0 | Prefix summaries and functional loop restoration obligations | Focused paired gates pass: symbolic scalar/reference traversal, exact early-break prefix/suffix values, shallow identities, and nonempty consistency controls. Resource loops use explicit state summaries and checked expiry assertions; the longest groups use the documented 600-second proof bound. |
| Reference-containing slice/Vec elements (IM3–IM4) | Native core gate, IM0 | Indexed referent resources and generic lifetime-resource interfaces | Focused paired gates pass for concrete/generic resources, generic preservation including non-`Copy` referents, single-/two-reference update loops, Vec views, complete Vec loops, exact partial updates through separate-crate slice/Vec clients, and all formerly archived controls. Combined IM5 focused qualification is complete; the full stdlib matrix and its regression repairs are in progress. |
| Adapters, closures, callbacks, general `collect` | Iterator and higher-order contracts | Separate closure/callback and composition work | Deferred from all three gates. |
| Interior mutability and general nested/recursive collections | Snapshot and ownership extensions beyond IM3–IM5 | Additional modeling and permission support | Deferred; the reference-element plan does not establish unrestricted nested-generic support. |
| Entry APIs, hash-collection iteration, heterogeneous lookup | M3 and additional models | API-specific state, representative, and borrowed-key contracts | Deferred from M3. |

On 2026-10-28, the lifetime-preservation change at `e9dd7929c` passed the
complete borrow suite: 80 checks in 1420.13 seconds, including generic
non-`Copy` values, two distinct inferred lifetimes, and a negative owner-value
case. The longest check was `inclusive_missing_pass` at 61.47 seconds. The
runner configuration and toolchain remain those recorded for P3-B. A later
follow-up narrows the change to types with inferred regions and no aliases or
unevaluated constants; its new-lifetime positive case was also checked
separately.

On 2026-10-28, the mutable-argument lifetime-resource implementation at
`65312bd57` passed the full borrow suite (80 checks, 1820.32 seconds; longest
`frame_pass`, 86.62 seconds) and iterator suite (75 checks, 1239.22 seconds;
longest `spec-library`, 72.37 seconds). The optional-empty addition and the
negative simultaneous-yield branch were also checked separately. The runner
used the pinned Rust, Viper, Z3, and Java versions and the verification
settings recorded above. These checks cover an explicit borrowed tail and
restoration of its length; they do not establish a functional standard
`IterMut` model or restoration of its elements.

Subsequent untrusted slice-backed iterator experiments establish element
restoration, rather than just the length of the remaining slice. For a
nonempty tail, two simultaneously live results can be mutated and expired in
either order; the original slice is then proved to contain both mutations.
A generic variant also restores two non-`Copy` elements in both use orders.
Negative cases reject
incorrect restored values. The `after_call` pledge expression captures the
post-call tail separately from its state at expiry; mutable-slice observations
require permission and respect the requested heap state. A further untrusted
generic contract separates the expiry cases and covers both empty and nonempty
tails; clients prove two mutations and a subsequent exhausted call for both
integer and non-`Copy` elements. An earlier
combined contract remains a solver timeout, recorded under
`stdlib_iterator_spec/known_limits`.

The complete borrow and iterator suites subsequently passed 91 and 109 checks,
respectively, using the `after_call` encoder before the later generic-lifetime
fix and diagnostic changes. The generic non-`Copy` positive and wrong-value
cases, including two simultaneously live yields, were verified separately with
the generic-lifetime fix.

The slice-backed evidence alone does not give `core::slice::IterMut` an element-restoration
contract: its raw-pointer representation does not carry the slice-backed
iterator's encoded lifetime resources. In particular, any resource model must
distinguish logical elements even when zero-sized elements share a physical
address. Reference-containing generic elements are also outside the verified
examples.

The untrusted borrowed-tail iterator also verifies an arbitrary-length loop
that mutates each yield, drops the iterator, and reuses the original slice.
Its false-assertion counterpart is rejected. The encoder preserves singleton
loop abstractions and the original reference's shallow value, honors lifetime
projection heap labels, and imports a composed wand's inputs before its
`old[lhs]` reads. This establishes the loop's resource restoration; the
whole-prefix value postcondition still needs a functional loop invariant.

The native iterator prototype now supplements its raw-pointer representation
with a verification-only borrowed slice tail. `mut_iter_tail` observes that
tail's identity in specifications, while `mut_slice_contents` separately
requires element permissions. Test-only construction and `next` contracts
transfer and reassemble those permissions with expiry pledges. The runtime
layout is unchanged, and the logical tail identity does not assert distinct
physical addresses for zero-sized elements.

A call-shape regression exposed a separate PCG issue: normalizing
`Iterator::Item` created a fresh lifetime equal to the iterator's element
lifetime, but that equality was lost with the inference context. PCG now
resolves equal region variables before building the shape. This keeps the
yielded-reference dependency alive even when the iterator is dropped first.
The regression failed before the fix; all eight function-shape tests, the
alias and closure tests, and all 248 numbered PCG fixtures pass afterward.

With that fix, the native and legacy mutable-iterator regression run passed
31 checks, including runtime checks, two live non-`Copy` yields after iterator
destruction, empty/exhausted states, loop ownership restoration, and exact
negative diagnostics for values, addresses, and permission misuse. The
quantified-`old` capture fix also passed all 97 borrow checks on the preceding
PCG revision. These runs disabled caching and used a 180-second verifier
limit with one parallel verifier. Native contracts are still test-only at
this checkpoint.

The subsequent bundled contracts cover `slice::iter_mut`, `IterMut::next`,
`ExactSizeIterator::len`, and mutable-slice `IntoIterator`. The former fixture
contracts have been removed. `mut_iter_refs` records reference identities
without granting referent permissions; element values use the heap-aware tail
observation. The verified `no_std` helper `set_first_native` exports a first
element mutation and unchanged-tail guarantee to separate clients. Ordinary
mutable-slice `for` loops restore source ownership.

The expanded integration run exposed an empty-resource enum case after the
PCG lifetime fix: `Option<&T>` produced a discriminant-guarded `true` instead
of no indirect resources. Shared loop invariants consequently demanded a wand
that had never been packaged. The indirect-predicate encoder now omits the
enum resource when every variant has no indirect resources. The complete
specification library and the shared loop proofs verify with this fix.

Bundled integration evidence combines 43 native/legacy checks with the
18-check shared/loop rerun after the empty-enum fix, plus four vector and four
zero-sized-loop checks. Replacing superseded results and deduplicating common
checks leaves 61 passing iterator checks. The initial library failure is
replaced by its successful complete verification in the shared/loop rerun;
no timeout or internal error counts as a negative success. A separate
13-check borrow run also passes after the PCG change. All runs use fresh
targets, disabled verification caching, overflow and panic checks, and the
180-second single-verifier configuration. The native gate covers the listed
core operations and ownership restoration; whole-prefix loop value proofs,
reference-containing elements, and further iterator methods remain deferred.
