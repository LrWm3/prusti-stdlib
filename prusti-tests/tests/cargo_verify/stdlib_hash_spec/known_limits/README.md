# Conditional expiry limitation

Run after building Prusti:

```sh
./x.py exec target/debug/cargo-prusti --manifest-path prusti-tests/tests/cargo_verify/stdlib_hash_spec/known_limits/Cargo.toml --bin conditional_expiry
```

Ordinary Rust compilation succeeds. Verification currently fails with
`unexpected nested spec type: after_expiry` because `refine_spec` only accepts
preconditions, postconditions, and `pure`. Even accepting the syntax would not
by itself make the expiry pledge available: the wand encoder must select and
apply conditional pledges only when the caller proves their bounds. This is a
minimal version of the missing mechanism needed for modeled `HashMap::get_mut`.
