# Standard-library external-contract milestone

This fixture checks real `Option::unwrap` and `std::mem::replace` calls using
contracts exported by the separate `stdlib_specs` crate. The client contains no
external declarations, trusted functions, or assumptions. The contracts are
prototype specifications for this fixture; they are not installed as bundled
standard-library contracts.

After following the repository setup instructions and building with
`./x.py build --all`, run from the repository root:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py
```

Use `--release` after building release Prusti binaries. The runner pins the
fixture's compiler to the repository's `rust-toolchain`, overriding a surrounding
`RUSTUP_TOOLCHAIN`. As with other `x.py` commands, set `JAVA_HOME` if Java is not in
a location detected by the setup helper.

The test is wired into the `all-tests` CI job separately from the legacy Cargo
harness, which is currently disabled. It uses a fresh target directory on each
invocation, exports dependency contracts without verifying dependency bodies,
disables the verification cache, and explicitly enables overflow and panic
checks. Verifier calls have a 90-second timeout; each test command has a
300-second outer timeout.

| Case | Expected result |
|---|---|
| Ordinary Rust check | All client binaries compile without verification. |
| Runtime positive client | Concrete assertions agree with the actual standard-library behavior. |
| `pass` | Generic unwrap and replacement postconditions, a non-`Copy` token with no equality traits, both replacement effects, and bounded arithmetic verify. |
| `false_assert` | Reject the false assertion, even with both external contracts loaded. |
| `overflow` | Reject addition without the necessary overflow precondition. |
| `unwrap_none` | Reject unwrapping a known `None`. |
| `unwrap_unknown` | Reject a generic unwrap without a `Some` precondition. |
| `unwrap_wrong_value` | Reject the wrong payload value. |
| `replace_wrong_return` | Reject a claim that replacement returns the new value. |
| `replace_wrong_destination` | Reject a claim that the destination retains its old value. |
| `pledge_wrong_post` | Reject an invalid shared-reference pledge, guarding against dropping expiry contracts when there are no indirect permissions. |

Each negative case has one `expected-error` annotation. Passing requires the
expected verification diagnostic at that source line. A compiler error, internal
error, unsupported feature, timeout, unexpected additional diagnostic, or
successful verification fails the test. Logs and a timing/result summary remain
in `target/stdlib-extern-spec-logs/` after the temporary build directory is removed.

The initial experiment exposed a permission-encoding bug: generic replacement
returned the destination permission inside `true --* P`, then its postcondition
tried to read the destination without that permission. The encoder now returns
those permissions directly when the wand's left side has no resources and there
are no pledges. Wands carrying actual resources or expiry conditions remain.

These checks establish the small external-contract milestone, not unrestricted
collection or reference-containing generic support. Review trusted contracts
against the pinned Rust implementation; passing clients alone cannot establish
their soundness.
