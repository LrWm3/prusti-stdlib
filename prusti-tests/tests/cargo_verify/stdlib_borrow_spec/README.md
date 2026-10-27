# Mutable collection views (P2-B, in progress)

Run from the repository root on the pinned toolchain, after building Prusti:

```sh
./x.py build --all
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite borrow
```

The runner explicitly verifies the `no_std` `borrow_specs` library before its
separately compiled clients. It compiles the clients with ordinary Rust, runs
the positive binaries, enables overflow and panic checks, disables verification
caching, and uses fresh Cargo targets. Logs and positive Viper programs remain
in `target/stdlib-borrow-spec-logs/`.

The checks cover `Vec::as_mut_slice`, `slice::split_at_mut`, mutable indexing,
and the supported parts of `slice::get_mut`. Generic clients verify changes
through both halves of a split, restoration to a slice or Vec owner, and halves
that expire separately.
An exported optional-borrow wrapper exercises both `Some` and `None`, and a
generic marker probe covers substitutions in expiry pledges. The selected
value and length of a checked mutable lookup are restored after a reborrow;
an absent lookup preserves the whole sequence. Slice and Vec indexing restore
the selected value after assignment. For scalar indices, checked lookup and
slice and Vec indexing also restore the whole sequence as an update at the
selected position, preserving every other element. The symbolic proofs include
generic owned elements and a separately verified cross-crate wrapper. Bounded
`start..end` mutable lookups and indexing also preserve the prefix and suffix
outside the selected range; their returned slice length is known.
Trait-borrow probes cover concrete pledges
and abstract calls without assuming implementation-specific guarantees. Negative
cases reject incorrect values, framing claims for splits, and invalid bounds.

The trusted ghost-only `scalar_index_model` is interpreted as `Some(i)` for a
`usize` index `i` and `None` for other sealed `SliceIndex` types. The `usize`
implementation exposes its classification; other index types remain opaque to
clients. A negative test rejects deriving a scalar classification for a range.
The trusted ghost-only `range_index_model` is interpreted as the endpoints for
`Range<usize>` and `None` for other index types. A corresponding negative test
rejects a range classification for a scalar index.

P2-B remains incomplete for other range forms and stronger type-conditional
contracts. The `known_limits` fixture retains reproducers outside the suite:

- `open_range_frame` cannot prove that `start..` preserves the prefix.
- `conditional_pass` cannot use a type-conditional postcondition even with
  its trait bound; `conditional_abstract` correctly rejects the same claim
  without the bound.

For example, reproduce the first limitation after building with:

```sh
PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true PRUSTI_NO_VERIFY_DEPS=true \
  ./x.py exec target/debug/cargo-prusti --manifest-path \
  prusti-tests/tests/cargo_verify/stdlib_borrow_spec/known_limits/Cargo.toml \
  --bin open_range_frame
```

The element domain and normal-return assumptions remain those documented for
the earlier collection gates.
