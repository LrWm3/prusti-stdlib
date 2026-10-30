This suite checks combined iterator and referent-state contracts across crate
boundaries. The separate `no_std` library is verified, including its symbolic
native `IterMut` loop and generated resource conversion helpers. Clients use
only its exported contract to prove unchanged reference identities and that
every referent becomes 7. Negative clients reject an incorrect final value
and `false` after the call. Runtime checks cover empty and nonempty slices.

Run after building this branch with `./x.py build --all --locked --features prusti/ferrum`:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py \
  --suite iterator_resources --backend Ferrum --timeout 600 \
  --solver-timeout-ms 30000 --solver-time-bounds
```

Repeat with `--backend Silicon` for specification changes, negative tests,
unexpected backend differences, and before merging. The runner uses one
verifier worker and disables Ferrum branch parallelism.

This suite is separate from the broader iterator library so the combined
resource proofs can be checked within their own program budget. The plan in
`STDLIB_EXTERN_SPEC.md` records acceptance results and outstanding generic,
early-break, wrapper, and Vec cases.
