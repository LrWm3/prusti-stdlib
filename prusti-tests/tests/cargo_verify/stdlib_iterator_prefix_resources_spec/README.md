# Exact prefix updates across crate boundaries

The untrusted `no_std` helper `set_prefix` uses native `IterMut` and a checked
prefix restoration summary to set exactly the first `limit` referents to 7.
Its contract preserves every element identity and every suffix referent's old
value for arbitrary slice lengths and any valid limit. Explicit snapshot
lengths and suffix-state assertions before and after iterator destruction
connect the resource summary to those final value claims. An explicit pointwise
resource-state postcondition exports the suffix frame. The `after_expiry(*values)`
pledge preserves the source's referent states when the updater's nested input
borrow later expires; this is needed when a caller subsequently restores the
backing Vec. The library must prove the pledge as well as its return-time
postconditions.

Separate untrusted clients apply this contract to `&mut [&mut u32]` and to a
mutable slice view of `Vec<&mut u32>`. They read an arbitrary valid element
before and after the call, proving either the prefix update or the unchanged
suffix, and preserve the container's element identities. Each container form
has `false`, wrong-prefix, and wrong-suffix controls on nonempty inputs.
Trusted `main` functions supply runtime examples only: empty, zero-prefix,
partial-prefix, and complete traversal. Neither the library proof nor the
client proof functions are trusted.

Run with a Prusti build from this branch:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py \
  --suite iterator_prefix_resources --backend Ferrum \
  --timeout 600 --solver-timeout-ms 30000 \
  --solver-check-timeout-ms 100 --solver-time-bounds
```

Repeat with `--backend Silicon` for paired qualification. `CARGO_TARGET_DIR`
selects the built Prusti binaries. The harness uses fresh fixture targets,
disables caching, checks the library and all generated helper bodies, and
runs one verifier with Ferrum branch parallelism disabled. This focused suite
does not rerun the full stdlib matrix. Current evidence is recorded in
`STDLIB_EXTERN_SPEC.md`.

The initial paired run passes all 12 checks on Silicon, including the untrusted
library (194.80 seconds), while Ferrum times out on that library. A subsequent
Ferrum run with reason-unknown reporting and an SMT log passes all 12 checks;
its library takes 254.02 seconds. Generated programs differ only by comments
and a bijective renaming of specification-helper UUIDs. The cause of the timing
variation is not established; the earlier timeout remains failed evidence in
`target/stdlib-im5-early-cross312`. No backend or encoder change was required.

All 12 checks per backend pass with rebuilt build 291 at the bounds above.
The source/artifact and exact-negative-diagnostic audit is in
`target/stdlib-im5-early-cross314/qualification.json`. The standalone whole-loop
positive and nonempty partial-traversal false control are
`mut_native_resource_early_{pass,false}` in the iterator fixture. These results
close the remaining combined focused proof gate; full stdlib matrix and
final-pin merge qualification remain deferred.
