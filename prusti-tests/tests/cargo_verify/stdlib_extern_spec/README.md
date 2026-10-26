# Standard-library external-contract milestone

This fixture checks real `Option::unwrap` and `std::mem::replace` calls using
contracts exported by the separate `stdlib_specs` crate. The positive clients
contain no external declarations, trusted functions, or assumptions. The
contracts are prototype specifications for this fixture; they are not installed
as bundled standard-library contracts.

The unannotated `target_api` crate supplies small traits for testing resolution
and refinement independently of any collection model. Their external contracts
live in `stdlib_specs`, so positive generic clients depend on exported metadata.
`Identity` preserves an associated item or a shared borrow, `Relay<T>` preserves
a trait type argument, and `Bounded` permits a result at most ten. Its `Zero`
implementation has a stronger contract. Negative declaration probes deliberately
place external specs in their client crate to check diagnostics.

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
| Runtime positive clients | Concrete assertions agree with the actual standard-library and target API behavior. |
| `pass` | Generic unwrap and replacement postconditions, a non-`Copy` token with no equality traits, both replacement effects, and bounded arithmetic verify. |
| `traits_pass` | Generic trait arguments, associated items (owned and shared-reference values), explicit borrow lifetimes, concrete external impl contracts, and a verified local refinement work across crates. |
| `false_assert` | Reject the false assertion, even with both external contracts loaded. |
| `overflow` | Reject addition without the necessary overflow precondition. |
| `unwrap_none` | Reject unwrapping a known `None`. |
| `unwrap_unknown` | Reject a generic unwrap without a `Some` precondition. |
| `unwrap_wrong_value` | Reject the wrong payload value. |
| `replace_wrong_return` | Reject a claim that replacement returns the new value. |
| `replace_wrong_destination` | Reject a claim that the destination retains its old value. |
| `pledge_wrong_post` | Reject an invalid shared-reference pledge, guarding against dropping expiry contracts when there are no indirect permissions. |
| `trait_wrong_value` | Reject a wrong value returned through an associated-type trait method. |
| `trait_generic_refinement` | Reject use of a concrete implementation's stronger postcondition in a generic caller. |
| `refinement_stronger_pre` | Reject a local implementation that strengthens an imported trait's precondition. |
| `refinement_weaker_post` | Reject a local implementation that weakens an imported trait's postcondition. |
| `duplicate_local` | Reject two external declarations for the same target in one crate. |
| `duplicate_imported` | Reject a local declaration that duplicates an imported contract. |
| `signature_generics` | Reject a monomorphic declaration for a generic target. |
| `signature_specialized` | Reject specialization of a target's type parameter while retaining the same parameter count. |
| `signature_coercion` | Reject a parameter type mismatch hidden by a mutable-to-shared reference coercion. |
| `signature_return_coercion` | Reject a return type mismatch hidden by that coercion. |
| `bounds_stronger_trait` | Reject an added `T: Copy` requirement on `mem::replace`. |
| `bounds_stronger_associated` | Reject an added `Copy` requirement on a trait's associated item. |
| `bounds_stronger_supertrait` | Reject an added `Copy` supertrait on an externally specified trait. |
| `bounds_stronger_lifetime` | Reject an added `T: 'static` requirement on `mem::replace`. |

Each negative case has one `expected-error` (verification) or
`expected-incorrect` (invalid specification) annotation. Passing requires the
exact diagnostic at that source line. Signature cases also require an
`expected-note` explaining the mismatch. Unexpected compiler errors, internal
errors, unsupported features, timeouts, additional diagnostics, and successful
verification fail negative tests. Logs and a timing/result summary remain
in `target/stdlib-extern-spec-logs/` after the temporary build directory is removed.

The initial experiment exposed a permission-encoding bug: generic replacement
returned the destination permission inside `true --* P`, then its postcondition
tried to read the destination without that permission. The encoder now returns
those permissions directly when the wand's left side has no resources and there
are no pledges. Wands carrying actual resources or expiry conditions remain.

The resolution probes exposed another gap: a generated Rust call can compile
even when the external declaration changes the target signature through a
coercion. The resolver now compares normalized input and output types. It also
requires type and const arguments to remain unchanged and in declaration order,
preventing a specialized declaration from becoming an unconditional contract.
The resolver also checks declaration bounds against the target environment,
rejecting added requirements unless Rust can establish them from that
environment. The positive trait client covers a redundant `Clone` bound implied
by `Copy`, an existing `'static` bound, bounded trait arguments, a generic
`?Sized` caller, and by-value receivers. The trait rewriter preserves supertraits
and permits unsized `Self`, adding `Sized` only for a by-value receiver.

The predicate check is conservative and can reject valid implications it cannot
establish, particularly complex outlives relationships. Signature comparison
still erases regions; it does not establish general equivalence of lifetime
relationships between a declaration and its target.

These checks establish the small external-contract milestone, not unrestricted
collection or reference-containing generic support. Review trusted contracts
against the pinned Rust implementation; passing clients alone cannot establish
their soundness.
