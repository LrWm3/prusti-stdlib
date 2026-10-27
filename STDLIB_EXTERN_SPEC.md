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

“Unbounded” means verification for arbitrary symbolic finite lengths, with no
fixed collection-size bound and no loop unrolling. Rust representability limits
and operation preconditions still apply. Length-only contracts, three-element
examples, and successful compilation do not establish functional verification.

External implementations remain trusted. The verification target is client code
that calls standard-library operations, rather than the allocator, raw-pointer
implementation, or unsafe internals of the standard library.

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

The inclusive-range contract accounts for Rust's hidden iterator state. Its
subsequence law is conditional on equality with the snapshot of
`RangeInclusive::new(start, end)`. Fresh literal ranges satisfy that condition;
equal endpoints alone do not establish it. Tests verify singleton ranges,
valid empty `4..=3`, invalid `5..=3`, and rejection of an inclusive
`usize::MAX` end. A negative probe rejects inferring a singleton from equal
endpoints of an arbitrary inclusive range. Modeling its iteration state remains
outside this gate.

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
borrow-expiry contracts. P2-C iteration and the overall M2 gate remain open.

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

**P2-B remains incomplete for other range forms.** An experimental model using
separate start/end classifiers verified isolated open-ended examples but caused
the already passing bounded-range fixture to hit the 90-second whole-verification
limit. A second experiment extending the bounded-range classifier with an
open-end marker also timed out on that fixture. Neither was retained. The
`open_range_frame` reproducer shows the missing outside-frame proof under the
retained contracts.
Type-conditional postconditions are not selected even when their bounds hold,
and the current refinement syntax does not support conditional expiry pledges.
These are retained as
[reproducers](prusti-tests/tests/cargo_verify/stdlib_borrow_spec/known_limits/)
outside the passing suite. The specification encoder caches by function
definition and the specification query selects only the base contract. A
conditional-contract solution would need call-site type arguments and the
caller's trait evidence, as well as conditional expiry pledges; an abstract call
must not acquire an implementation-specific guarantee.

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

### P3-B: Core map/set operations and mutable map values

**Prerequisite:** P3-A; the `get_mut` work also depends on P2-B.

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

## Validation and delivery rules

### Evidence required for every gate

Record the revision, toolchain/backend versions, invocation, configuration,
expected result, actual result, and verifier time. Use the focused runner from
P1-A for positive, negative, overflow-enabled, and cross-crate cases. Record which
tests establish generic behavior and which are only concrete regressions.

A negative test must fail for the intended verification or diagnostic reason.
Crashes, unsupported-feature errors, timeouts, and `known-failure` annotations do
not count as rejection of an incorrect proof. A timeout also does not justify a
fixed-length fallback. Track timings against the existing 90-second per-file
verifier timeout before changing any limits.

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
for owned values with stable value snapshots. General outlives equivalence is
conservatively unsupported. Phases 2 and 3 remain open; a hosted CI run is not
claimed.

| Gate or capability | Depends on | Expected Prusti work | Baseline status / completion criterion |
|---|---|---|---|
| Baseline inventory | Checkout `87487f774` | None | Inspection complete; no new proofs claimed. |
| P1-A: reproducible test execution | Setup, pinned submodules/toolchain | Focused harness and CI wiring | Verified locally; positive, negative, overflow, and cross-crate execution demonstrated. |
| P1-B: external-contract foundation | P1-A | Targeted rewriting/resolution/substitution/metadata fixes | Verified locally: 43 focused checks, bundled unwrap/replace, cross-crate and core-only clients, conservative lifetime matching, bounds, duplicates, and refinement. |
| P1-C: sequence observations | P1-B | Model connection, explicit triggers, required generic snapshots | Verified locally: 21 focused checks, arbitrary-index observations, explicit triggers, cross-crate proofs, and precise diagnostics. |
| **M1: unbounded functional Vec** | P1-A through P1-D | Minimal contracts plus reproduced blockers | Verified locally: 21 Vec checks, symbolic append/pop/round-trip and whole-prefix loop proofs for owned non-`Copy` elements, cross-crate clients, and negative cases. |
| P2-A: Vec and shared slices | M1 | Contracts, sequence lemmas, any slice encoding fixes | Verified locally: 27 checks cover symbolic mutations, shared lookup/ranges, splits, swaps, boundaries, ordering, and framing. |
| P2-B: mutable collection views | P2-A, two-state snapshots | Pledges, expiry, generic reference framing | In progress: 40 checks cover scalar and bounded-range mutable views and framing; other range forms and conditional specifications remain open. |
| P2-C: shared and owned iterators | Sequence model, generic trait probes | Opt-in models, refinement, lifetimes, loop support | Untested; concrete and generic traversal proofs pass. |
| **M2: Vec/slice phase complete** | P2-A, P2-B, P2-C | Integration and cross-crate validation | Untested; all required gates pass. |
| P3-A: explicit key models | M2, logical Map/Set | Key-model interfaces and conditional contracts | Untested; equality/hash/model evidence is enforced. |
| **M3: HashMap/HashSet phase complete** | P3-A, P3-B, P2-B expiry support | Map/set contracts and mutable lookup | Untested; key, cardinality, framing, and mutable-value cases pass. |
| `IterMut` | P2-B and iterator models | Yielded-reference tracking and owner restoration | Deferred from M2; investigate after mutable access works. |
| Adapters, closures, callbacks, general `collect` | Iterator and higher-order contracts | Separate closure/callback and composition work | Deferred from all three gates. |
| Reference-containing elements, interior mutability, nested collections | Snapshot and ownership extensions | Additional modeling and permission support | Deferred; no unrestricted nested-generic support claim. |
| Entry APIs, hash-collection iteration, heterogeneous lookup | M3 and additional models | API-specific state, representative, and borrowed-key contracts | Deferred from M3. |
