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

The 32 checks cover `Vec::as_mut_slice`, `slice::split_at_mut`, mutable indexing,
and the supported parts of `slice::get_mut`. Generic clients verify changes
through both halves of a split, restoration to a slice or Vec owner, and halves
that expire separately.
An exported optional-borrow wrapper exercises both `Some` and `None`, and a
generic marker probe covers substitutions in expiry pledges. The selected
value and length of a checked mutable lookup are restored after a reborrow;
an absent lookup preserves the whole sequence. Slice and Vec indexing restore
the selected value after assignment. Trait-borrow probes cover concrete pledges
and abstract calls without assuming implementation-specific guarantees. Negative
cases reject incorrect values, framing claims for splits, and invalid bounds.

P2-B remains incomplete. The current `get_mut` contract does not prove that
unselected elements remain unchanged after a successful lookup or mutable
indexing. Stronger type-conditional contracts also remain open. The
`known_limits` fixture retains concrete reproducers outside the passing suite:

- `lookup_frame` cannot prove an untouched element remains unchanged.
- `conditional_pass` cannot use a type-conditional postcondition even with
  its trait bound; `conditional_abstract` correctly rejects the same claim
  without the bound.

For example, reproduce the first limitation after building with:

```sh
PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true PRUSTI_NO_VERIFY_DEPS=true \
  ./x.py exec target/debug/cargo-prusti --manifest-path \
  prusti-tests/tests/cargo_verify/stdlib_borrow_spec/known_limits/Cargo.toml \
  --bin lookup_frame
```

The element domain and normal-return assumptions remain those documented for
the earlier collection gates.
