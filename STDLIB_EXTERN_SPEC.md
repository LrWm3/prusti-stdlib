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
include empty inputs and reject incorrect exit-count claims. They observe the
live tail; they do not prove that the processed prefix is restored to the owner.

The constructor's initial source–tail value equality has also been checked
inside the actual emitted Viper package, using the generated slice predicates
and the existing constructor pledge. A deliberately opposite assertion fails.
This checks only the base case; the generated loop wand still omits the value
relation on its backedge and exit.

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
