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
probe illustrates a remaining obstacle to linking references yielded by
`IterMut` back to the source slice after they expire.
