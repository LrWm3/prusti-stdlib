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

**Status:** IM0 and the IM1 standalone proof-model gate are complete. IM2,
carrying that value relation through Rust loop encoding, is in progress. Work
began from Prusti `22ea24a6b` and PCG `a4eb802`. The completed native gate supplies
construction, forward advancement, lengths, simultaneous yields, early
iterator destruction, and ownership restoration. It does not establish the
two extensions below. Keep that support claim unchanged until their gates pass.

The execution order is IM0, IM1, IM2, IM3, IM4, then IM5. Functional loops
first use owned elements with stable snapshots. Reference support then adds
the separate permissions needed to observe and update nested referents.
The final gate combines both extensions.

| Step | Deliverable | Depends on | Exit gate |
|---|---|---|---|
| IM0 | Minimal reproducers and resource/state inventory | Native core gate | Complete: [17 probes and the resource ledger](prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/README.md#im0-native-loop-values-and-references-inside-elements) record the observed failures and controls. |
| IM1 | Proven loop restoration summary | IM0 | Complete: the standalone summary, negative checks, and untrusted Rust expiry-pledge controls pass. |
| IM2 | Functional native iterator loops | IM1 | Rust clients prove whole-slice updates and exact early-break prefix/suffix behavior for symbolic lengths. |
| IM3 | Concrete reference-containing slice elements | IM0; implemented after IM2 | Reads, writes, framing, and expiry work for shared references, mutable references, and reference-bearing structs/enums. |
| IM4 | Generic and cross-crate reference resources | IM3 | Generic definitions retain the same referent permissions under substitution and through Vec/slice views. |
| IM5 | Combined acceptance and support update | IM2, IM4 | Symbolic iterator loops over reference-containing elements prove final values and framing across crate boundaries. |

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
Keep Silicon checks for changed specifications, negatives, and backend
differences, and require dual-backend validation before merging. The full
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

Run focused paired tests during each step, then the full iterator and borrow
suites and relevant sequence, Vec, collection, and external-specification
suites at acceptance. Run PCG shape tests and the numbered corpus whenever
PCG changes. Use fresh targets, disabled caching, overflow/panic checks, and
the current 180-second limit with one parallel verifier. A timeout, crash,
unsupported-feature error, or wrong diagnostic is an unresolved gate, not a
successful negative proof. Reproduce solver failures before changing limits.

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
| Functional `IterMut` loops (IM1–IM2) | Native core gate, IM0 | Prefix summaries and functional loop restoration obligations | Planned: symbolic full-traversal and early-break value/framing proofs, followed by combined acceptance in IM5. |
| Reference-containing slice/Vec elements (IM3–IM4) | Native core gate, IM0 | Indexed referent resources and generic lifetime-resource interfaces | Planned: concrete and cross-crate generic reference shapes; combine with functional loops in IM5 before extending the support claim. |
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
