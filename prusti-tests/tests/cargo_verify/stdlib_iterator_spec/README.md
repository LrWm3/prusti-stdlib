# Iterator contract progress

The fixture checks bundled contracts for `slice::iter_mut`, mutable-slice
`IntoIterator`, and `IterMut`'s `next` and `len`. These are trusted normal-return
contracts reviewed against the pinned standard-library implementation.
`mut_iter_refs` observes the identities of the unyielded references in order;
address-only observations connect each yield to its original position without
reading its value from a mutable-reference snapshot. Runtime checks also cover
the shared address of zero-sized slice elements.

The native iterator's encoded representation includes a logical
`&mut [T]` tail, observed by the specification-only `mut_iter_tail` marker.
The tail carries the remaining element permissions; each `next` pledge
reassembles the yielded element and the final tail at expiry. This supports
two simultaneous yields, non-`Copy` elements, empty and exhausted iterators,
and source-slice reuse after mutation. Yielded references remain usable after
the iterator itself is dropped. A loop also restores ownership of the source;
functional prefix assertions additionally require a checked `body_restore_prefix`
summary.

The `no_std` specification library verifies `set_first_native`, which uses a
native iterator to update the first element and preserves the remaining
slice. Separate clients use its exported contract to check the mutation and
framing. Other cases exercise length queries and ordinary `for` loops over a
mutable slice without local external specifications. Native mutable iterators
use heap-aware observations rather than the pure `ModeledIterator` capability
used by owned and shared iterators.

The vector cases check that `Vec::iter_mut()` composes with the mutable-slice
reborrow contracts: writing through a yielded reference updates the vector
and preserves untouched elements. Zero-sized loop cases also exercise
traversal and restoration without assuming distinct physical addresses.

The marker supplies an identity, not permission to read elements. Negative
cases check that a saved tail cannot be read after a yield, that incorrect
restored values fail, and that zero-sized elements are not assumed to have
distinct physical addresses. Executable code and ordinary pure functions
cannot call the marker. The `mut_element_*` clients separately check direct indexing of concrete
mutable-reference elements, fixed arrays, optional references, and structs
with two independent lifetimes. They prove mutation and framing through
ordinary Rust scalar reads, with exact wrong-value and false controls. Native
straight-line iteration now also restores mutable-reference elements and
reference-bearing holders, including initial and untouched referent values,
empty/exhausted cases, and mutable-slice `IntoIterator`. The
`mut_native_{holder,mut_ref}_restoration_*` and `mut_native_resource_*` cases
exercise these resource contracts. The `mut_native_resource_loop_identity_*`
cases additionally verify joint source/referent ownership restoration and
unchanged slot identities after a symbolic reference-update loop, with false
and incorrect-count controls. The three-argument prefix summary records
shallow identities. The four-argument form additionally records a
`Seq<ResourceState>` captured while each yielded element is accessible.
`mut_native_resource_prefix_*` proves that a symbolic traversal sets every
referenced value to 7 and rejects an incorrect recorded state, a wrong final
value, and `false` after traversal. Vec views and whole reference-element Vec
loops now pass focused paired checks. `mut_native_resource_early_pass` also
proves exact prefix updates and unchanged suffix referents on both backends;
its nonempty consistency control also passes both backends. Separate-crate
slice/Vec qualification remains open. The untrusted
`iterator_resources` library covers single-reference and two-reference wrapper
loops across crates. Solver configuration and current evidence are recorded
in `STDLIB_EXTERN_SPEC.md`; the full stdlib matrix is deferred.

The `mut_native_resource_pair_prefix_*` cases extend
the symbolic full traversal to structs containing two independent mutable
references. The positive proves unchanged element identities and values 7
and 8 in the respective fields at an arbitrary valid index. Both backends
also reject `false` after restoration and an incorrect second-field value.
The final field observations use ordinary Rust reads followed by scalar
assertions; nested mutable dereferences in pure assertions remain unsupported.

Run this focused group with the combined-proof limits:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator \
  --backend Ferrum --skip-library --case 'mut_native_resource_pair_prefix_*' \
  --timeout 600 --solver-timeout-ms 30000 --solver-check-timeout-ms 100 \
  --solver-time-bounds
```

Use `--backend Silicon` for paired validation. The first-field wrong-value
and fabricated-state controls have returned from `known_limits` after passing
both backends with build 291. Earlier wand-matching errors and timeouts remain
recorded as failed historical evidence (`target/stdlib-im5-controls294`).

The `mut_native_resource_generic_prefix_{pass,false,wrong_state}` cases
exercise whole-prefix preservation for `&mut [T]`, without `Copy` or equality
bounds. The positive captures the initial referent-state sequence and proves
that both it and the element identities survive a symbolic native traversal.
Runtime cases include empty input, owned non-`Copy` values, shared references,
mutable references, and zero-sized elements. The wrong-state control
incorrectly claims the final state sequence differs from its initial value.
Both cases reach their expected outcomes on Ferrum and Silicon with build 229
(`target/stdlib-im5-generic251`). The post-loop false control also passes both
backends with build 291 after placing its false claim after the valid final
restoration assertions (`target/stdlib-im5-controls294`). Select this group with
`--case 'mut_native_resource_generic_prefix_*'` and the same proof bounds
shown above.

`sequence_resource_states(&elements)` captures all indexed referent states
in a ghost `Seq<ResourceState>`. It requires the corresponding permissions;
the result is a value snapshot and gives no access to the heap. The
`mut_sequence_states_*` cases check concrete and generic observations, length
and pointwise agreement, missing permissions, stale snapshots after mutation,
and rejection in executable code. This snapshot can be taken before creating
an iterator, so a later invariant need not borrow the original owner again.

A separate untrusted iterator stores an explicit borrowed tail and implements
`next` with `mem::replace` and `split_at_mut`. Its regressions restore multiple
simultaneously yielded elements, including non-`Copy` values, and handle
exhaustion. `mut_region_loop_pass` also verifies an unbounded mutation loop
followed by reuse of the original slice; a false assertion after that loop is
rejected. This proves ownership restoration, while a functional invariant for
the entire updated prefix remains to be supplied.

On 2026-10-27, the iterator suite passed 71 checks at encoder revision
`98a6bcee2`, with overflow and panic checks enabled and the cache disabled.
The separately added two-yield zero-sized negative case also produced its
expected diagnostic. The suite took 656.03 seconds; its longest check was the
specification library at 32.88 seconds. Rust, Viper, Z3, and PCG versions are
recorded in the P3-B evidence in `STDLIB_EXTERN_SPEC.md`.

This fixture checks the bundled contracts for `Vec::into_iter` and
`Iterator::next` on `Vec::IntoIter`. The sealed `ModeledIterator` capability
observes the remaining sequence. Constructing an iterator preserves the
vector's element sequence; `next` returns its head and leaves its tail, or
returns `None` and preserves an empty sequence. These are trusted normal-return
contracts reviewed against the pinned standard-library implementation.

The separately verified `no_std` library counts a symbolic number of owned
elements in both an explicit loop and `prusti_for!`, and exports their
contracts. `prusti_for!` names the iterator so a body invariant can refer to its
remaining sequence; the current item has already been removed when the
invariant is checked. Its runtime test covers an empty iterator, non-`Copy`
values, a single evaluation of the iterable, and labelled `break` and
`continue`. Clients check generic owned values
without `Copy`, `Clone`, or equality bounds, concrete non-`Copy` values, empty
and repeatedly exhausted iterators, and cross-crate use. Its verified generic
`advance` wrapper also exports the head-and-tail contract for an iterator of
borrowed items. Negative probes reject
an incorrect yielded value, an unchanged nonempty state, an incorrect `None`,
and permanent exhaustion for an arbitrary `Iterator` without model evidence.
The capability is sealed so arbitrary iterator implementations cannot assume
these finite-sequence or permanent-exhaustion laws.

The verified `prefix_owned` helper exercises a symbolic early `break` and
returns exactly the smaller of the input length and a positive stopping point.
A cross-crate client checks non-`Copy` elements; a negative client rejects
claiming that a shortened traversal processed the whole input.

The same capability models `slice::Iter`: its constructor records references
to the source elements. Clients use `next` directly and through the verified
generic `advance` wrapper. A cross-crate `first_shared` wrapper returns the
first source reference. Clients verify non-`Copy` elements, source values, and
exhaustion; the `no_std` library also checks each yielded reference in a
symbolic traversal. The shared-slice `IntoIterator` contract preserves the
same source relationships, and `prusti_for!` verifies a symbolic shared count.
With its optional initializer capturing the initial iterator sequence, the
shared traversal also checks each yielded reference against its source index.
Negative probes reject incorrect references, state, and loop invariants.

For modeled `ExactSizeIterator` implementations, the `len` contract equals
the remaining sequence length. The separately verified `remaining_len` helper
exports that relation across crates. `advance_with_lengths` uses
`ExactSizeIterator::len(&*iterator)` to select the modeled iterator when its
generic parameter is `&mut I`, and verifies the length before and after `next`.
Clients check owned and shared lengths before and after advancement, including
empty iterators and non-`Copy` values.
Negative probes reject an incorrect length and a length claim for an iterator
without model evidence.

Ordinary `for` loops over an owned `Vec` can name their compiler-generated
iterator in a body invariant with `for_iterator::<std::vec::IntoIter<T>>()`.
The library verifies a symbolic count this way, and clients exercise nested
loops and another same-typed iterator in the body. The marker resolves only a
unique compiler-generated `Iterator::next` for the enclosing loop, with the
exact requested type and a readable loop-boundary capability. It is rejected
in executable code, outside a body invariant, under `old` or nested
specification closures, and for iterator types containing explicit lifetimes.

For example, a body invariant can relate the number already processed, the
current item, and the remaining elements:

```rust
for item in values {
    body_invariant!(Int::from(seen)
        + for_iterator::<std::vec::IntoIter<T>>().remaining().len()
        + Int::from(1) == Int::from(expected));
    // Process item, then advance seen.
}
```

The opt-in macro exposes the iterator directly:

```rust
prusti_for!(iterator, for item in values => {
    body_invariant!(Int::from(seen) + iterator.remaining().len() + Int::from(1)
        == Int::from(expected));
    // Process item, then advance seen.
});
```

When a proof needs the initial iterator sequence, put a ghost snapshot in the
initializer. It runs once, after conversion and before the first `next`:

```rust
prusti_for!(iterator, for item in values => init {
    let before = ghost! { iterator.remaining() };
} {
    body_invariant!(seen < expected);
    body_invariant!(iterator.remaining() == (*before)[seen + 1..]);
    // Process item, then advance seen.
});
```

Run after building Prusti:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite iterator
```

The suite runs ordinary Rust and runtime checks, explicitly verifies the library,
and checks diagnostic locations with overflow and panic checks enabled. It is
also included in the `all-tests` CI job. As with the other collection contracts,
allocation, unwinding, arbitrary destructors, interior mutability, and general
reference-containing collection snapshots are outside the tested domain. The
borrowed-item probe starts with an existing iterator; it does not establish a
model for constructing a vector of references.

Some lifetime-dependent evidence remains unsupported, including markers for
shared iterators. P2-C's stated iterator gate and M2 integration pass. An
existing owned iterator of borrowed elements supports both
direct `next` calls and the generic wrapper.

The `known_limits/for_count` probe omits the marker and reproduces the original
gap: a symbolic count does not preserve the relation between the hidden
iterator and the number of items seen, so the increment may overflow. The
`for_named` probe shows why naming the iterator does not expose it to the
invariant: `for` holds its mutable borrow for the entire loop, so Rust rejects
a second borrow. The `exact_length_mut_ref` probe records that method syntax
on `&mut I` selects a forwarding iterator implementation without the sealed
model capability; explicitly selecting the underlying `I`, as in
`advance_with_lengths`, verifies the length relationship.
Shared traversal can use the original source reference directly in its loop
invariant. Reproduce either probe after building:

```sh
PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true PRUSTI_NO_VERIFY_DEPS=true \
  ./x.py exec target/debug/cargo-prusti --manifest-path \
  prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml \
  --bin for_count
```

Replace `for_count` with `for_named` or `exact_length_mut_ref` for the other
probes.
