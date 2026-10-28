# Mutable iterator reference snapshots

The `mutable_ref_snapshot` probe attempts to model the remaining mutable
references and relate each reference to the corresponding source element:

```sh
./x.py exec target/debug/cargo-prusti --manifest-path prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml --bin mutable_ref_snapshot
```

Ordinary Rust compilation succeeds. Verification reports an unsupported
mutable-reference dereference in pure code. This restriction is intentional:
a mutable reference's pure snapshot does not constrain its referent value, so
reading that value without the appropriate heap state would be unsound. The
probe illustrates why reference values cannot be recovered directly from a
pure mutable-reference snapshot. The test-only iterator fixture now uses
address-only observations to relate yielded references to the source without
performing that dereference.

This is a snapshot and borrowing limitation, affecting the optional `IterMut`
owner-restoration milestone. A possible next step is an identity-only model
for yielded references combined with a heap-aware expiry rule that transfers
each final value back to its original slice position. The identity-only part
is now prototyped; the expiry rule remains open. Such a model must also
account for multiple simultaneously live yielded references and exhaustion;
removing the pure dereference restriction alone would not establish those
properties and would be unsound.

The `expiry_ref_heap` probe attempts a single-element version of that expiry
rule using `before_expiry` to read a saved reference. On the pinned toolchain,
verification reaches a Viper insufficient-permission error which is currently
reported as an internal backtranslation error. The iterator's lifetime
projection has no encoded predicate for its raw-pointer-backed elements, so
the saved reference's address alone does not provide permission to read it.
Reproduce with the command above, replacing `mutable_ref_snapshot` with
`expiry_ref_heap`. A complete solution needs a lifetime resource for the
iterator's remaining elements and a way to transfer and reassemble the
individual element permissions as yields expire.

The `region_next` probe replaces the raw-pointer-backed iterator with a
struct containing an explicit `&mut [u32]`. Its trusted `next` has no
functional guarantees and its body always returns `None`; the caller must
still handle an unconstrained `Some`. With the first possible result live,
the second call fails with an insufficient-permission error, currently also
reported as an internal backtranslation error. This happens without any
pledge. Adding an iterator lifetime predicate alone therefore will not allow
successive yields: the call contract must retain a resource for the
unconsumed tail while the yielded reference owns a separate element resource.
The command above can also be run with `--bin region_next`.
