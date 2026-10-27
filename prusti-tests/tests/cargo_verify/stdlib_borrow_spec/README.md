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

The initial four checks verify a `Vec::as_mut_slice` contract that exposes the
entry sequence and restores the Vec's sequence from the slice's final contents
when the borrow expires. The generic exported `swap_view` wrapper proves that
changes through the mutable slice reach its owner. A generic client proves
length preservation, exchanged values, and restoration after two swaps; its
runtime instance uses owned values without copy, clone, or equality traits.

This is initial P2-B progress, not a completed gate. Mutable indexing, checked
mutable lookup, disjoint splitting, reborrowing, and their negative probes remain
to be established. The element domain and normal-return assumptions remain
those documented for the earlier collection gates.
