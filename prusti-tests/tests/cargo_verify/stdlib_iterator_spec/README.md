# Iterator contract progress

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

The same capability models `slice::Iter`: its constructor records references
to the source elements. Clients use `next` directly and through the verified
generic `advance` wrapper. A cross-crate `first_shared` wrapper returns the
first source reference. Clients verify non-`Copy` elements, source values, and
exhaustion; the `no_std` library also checks each yielded reference in a
symbolic traversal. The shared-slice `IntoIterator` contract preserves the
same source relationships, and `prusti_for!` verifies a symbolic shared count.
Negative probes reject incorrect references, state, and loop invariants.

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
shared iterators. This does not complete P2-C or M2. An existing owned iterator
of borrowed elements supports both direct `next` calls and the generic wrapper.

The `known_limits/for_count` probe omits the marker and reproduces the original
gap: a symbolic count does not preserve the relation between the hidden
iterator and the number of items seen, so the increment may overflow. The
`for_named` probe shows why naming the iterator does not expose it to the
invariant: `for` holds its mutable borrow for the entire loop, so Rust rejects
a second borrow.
Shared traversal can use the original source reference directly in its loop
invariant. Reproduce either probe after building:

```sh
PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true PRUSTI_NO_VERIFY_DEPS=true \
  ./x.py exec target/debug/cargo-prusti --manifest-path \
  prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml \
  --bin for_count
```

Replace `for_count` with `for_named` for the borrow-checking probe.
