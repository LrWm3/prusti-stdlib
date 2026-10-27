# Owned iterator contract progress

This fixture checks the bundled contracts for `Vec::into_iter` and
`Iterator::next` on `Vec::IntoIter`. The sealed `ModeledIterator` capability
observes the remaining sequence. Constructing an iterator preserves the
vector's element sequence; `next` returns its head and leaves its tail, or
returns `None` and preserves an empty sequence. These are trusted normal-return
contracts reviewed against the pinned standard-library implementation.

The separately verified `no_std` library counts a symbolic number of owned
elements in a loop and exports its contract. Clients check generic owned values
without `Copy`, `Clone`, or equality bounds, concrete non-`Copy` values, empty
and repeatedly exhausted iterators, and cross-crate use. Its verified generic
`advance` wrapper also exports the head-and-tail contract for an iterator of
borrowed items. Negative probes reject
an incorrect yielded value, an unchanged nonempty state, an incorrect `None`,
and permanent exhaustion for an arbitrary `Iterator` without model evidence.
The capability is sealed so arbitrary iterator implementations cannot assume
these finite-sequence or permanent-exhaustion laws.

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

Shared slice iterators, direct calls where selecting the capability requires
region-bearing evidence, and symbolic `for`-loop proofs remain open; this does
not complete P2-C or M2.

The `known_limits/for_count` probe reproduces the current `for`-loop gap: a
symbolic count does not preserve the relation between the hidden iterator and
the number of items seen, so the increment may overflow. `borrowed_items`
reproduces conservative rejection of the iterator contract when the owned
iterator's item type contains a lifetime. Reproduce either after building:

```sh
PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true PRUSTI_NO_VERIFY_DEPS=true \
  ./x.py exec target/debug/cargo-prusti --manifest-path \
  prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml \
  --bin for_count
```

Replace `for_count` with `borrowed_items` for the lifetime case.
