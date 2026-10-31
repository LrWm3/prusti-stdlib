# Generic iterator resource preservation

The untrusted `no_std` helper `preserve_all<T>` traverses a mutable slice using
`IterMut` and a prefix restoration summary. Its contract preserves shallow
element identities and the whole resource-state sequence. A per-element
postcondition with a pure sequence-lookup trigger exposes the same state frame
to concrete callers. The helper has no `Copy` or equality bound on `T`.

The positive clients check the generic contract and read an arbitrary valid
referent before and after calls with `&mut [&mut u32]` and `&mut [&mut Owned]`,
where `Owned(u32)` does not implement `Copy`. Negative clients assert
`false`, unequal final states on nonempty input, and a changed concrete referent
value. The concrete wrong-value case requires a valid index, so its rejection
cannot rely on an empty slice. Trusted `main` functions only provide runtime
checks; the helper and client proof functions remain untrusted.

Run the focused suite with a Prusti build from this branch:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py \
  --suite iterator_generic_resources --backend Ferrum \
  --timeout 600 --solver-timeout-ms 30000 \
  --solver-check-timeout-ms 100 --solver-time-bounds
```

`CARGO_TARGET_DIR` selects the built Prusti binaries. The harness uses fresh
fixture targets, disables verification caching, checks all generated helper
bodies, and runs one verifier with Ferrum branch parallelism disabled. Use
`--backend Silicon` for the corresponding focused specification and negative
checks. This suite does not rerun the full stdlib matrix.

The initial aggregate-only contract verified
its library but did not establish the concrete caller's value assertion on
either backend. With the explicit per-element postcondition, all seven
original checks pass on both backends at the repository paths; the original
failure remains recorded in the plan. The additional non-`Copy` pair also
passes its four focused checks on both backends, with the helper unchanged.
The source/artifact audit is in
`target/stdlib-im5-generic-cross263/qualification.json`. This establishes the
listed generic/cross-crate proofs; early-break and Vec support remain separate
open work.
