# Iterator contract progress

The fixture also probes `slice::IterMut` separately from the bundled iterator
model. A trusted, test-only observation describes the unyielded element
snapshots. Trusted external contracts connect construction to the original
slice and describe forward advancement, the returned reference's initial
value, and exhaustion. A separate reference sequence and address-only
observations connect each yield to its original position without reading the
referent from a mutable-reference snapshot. Cross-crate clients check two
simultaneously live mutable yields and their identities; negative clients
reject an incorrect identity, value, and owner restoration under those limited
contracts. A runtime check
also covers the shared address of zero-sized slice elements. Restoring the
original slice after mutation requires heap-aware permissions. These probes
do not expose a bundled `IterMut` contract.

The `mut_native_*` cases provide separate test-only contracts for native
`core::slice::IterMut`. Its encoded representation now includes a logical
`&mut [T]` tail, observed by the specification-only `mut_iter_tail` marker.
The tail carries the remaining element permissions; each `next` pledge
reassembles the yielded element and the final tail at expiry. This supports
two simultaneous yields, non-`Copy` elements, empty and exhausted iterators,
and source-slice reuse after mutation. Yielded references remain usable after
the iterator itself is dropped. A loop also restores ownership of the source;
it does not yet prove a value invariant for the entire updated prefix.

The marker supplies an identity, not permission to read elements. Negative
cases check that a saved tail cannot be read after a yield, that incorrect
restored values fail, and that zero-sized elements are not assumed to have
distinct physical addresses. Executable code and ordinary pure functions
cannot call the marker. Reference-containing element types and additional
iterator operations are outside these examples.

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
