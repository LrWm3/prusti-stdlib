This suite checks combined iterator and referent-state contracts across crate
boundaries. The separate `no_std` library is verified, including its symbolic
native `IterMut` loop and generated resource conversion helpers. Clients use
only its exported contract to prove unchanged reference identities and that
every referent becomes 7. Negative clients reject an incorrect final value
and `false` after the call. Runtime checks cover empty and nonempty slices.

The `native_resource_pair_cross_*` clients exercise a second untrusted helper,
`copy_pair_values`. It copies both referent values from an exemplar into every
element of a slice of `Pair<'a, 'b>`, preserving element and exemplar reference
identities. Its contract exports the final resource state with an explicit
element-identity trigger. The positive client proves values 7 and 8 at an
arbitrary valid index; separate controls reject `false`, a wrong first value,
and a wrong second value. The exemplar identity invariant is required
separately from its referent-state invariant.

Run after building this branch with `./x.py build --all --locked --features prusti/ferrum`:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py \
  --suite iterator_resources --backend Ferrum --timeout 600 \
  --solver-timeout-ms 30000 --solver-check-timeout-ms 100 --solver-time-bounds
```

Repeat with `--backend Silicon` for specification changes, negative tests,
unexpected backend differences, and before merging. The runner uses one
verifier worker and disables Ferrum branch parallelism.

The expanded suite passes all 11 checks on both backends with the rebuilt
build-229 driver and the bounds above. Results, source hashes, and the audit
are under `target/stdlib-im5-pair-cross248`. Both untrusted library bodies and
every generated conversion helper are checked. This establishes the listed
cross-crate proofs; the standalone wrapper controls under `known_limits`
retain their separate unresolved results.

This suite is separate from the broader iterator library so the combined
resource proofs can be checked within their own program budget. The plan in
`STDLIB_EXTERN_SPEC.md` records acceptance results and outstanding generic,
early-break, wrapper controls, and Vec cases.
