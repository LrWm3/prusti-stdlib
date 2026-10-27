# Standard-library external-contract milestone

This fixture checks real `Option::unwrap`, `core::mem::replace`, and
`std::mem::replace` calls using the contracts bundled in `prusti_contracts`.
`pass` consumes those contracts directly; `cross_crate_pass` consumes verified
generic wrappers exported by `stdlib_specs`. Neither client contains external
declarations, trusted functions, or assumptions. `core_only` checks generic
clients in a `#![no_std]` library with only the `core` contracts feature enabled.

The unannotated `target_api` crate supplies small traits for testing resolution
and refinement independently of any collection model. Their external contracts
live in `stdlib_specs`, so positive generic clients depend on exported metadata.
`Identity` preserves an associated item or a shared borrow, `Relay<T>` preserves
a trait type argument, and `Bounded` permits a result at most ten. Its `Zero`
implementation has a stronger contract. `lifetimes_pass` declares additional
valid external signatures locally. Negative declaration probes deliberately
place invalid external specs in their client crate to check diagnostics.

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
checks. It explicitly verifies the `stdlib_specs` wrapper bodies and the
`core_only` library as separate packages. Verifier calls have a 90-second timeout;
each test command has a 300-second outer timeout.

| Case | Expected result |
|---|---|
| Ordinary Rust check | All client binaries compile without verification. |
| `rust-core-only`, `core-only` | The core-only library compiles normally and verifies without requiring std or alloc features. |
| `spec-library` | The separately compiled generic wrapper bodies verify using the bundled contracts. |
| Runtime positive clients | Concrete assertions agree with the actual standard-library and target API behavior. |
| `pass` | Generic unwrap and replacement postconditions, a non-`Copy` token with no equality traits, both replacement effects, and bounded arithmetic verify. |
| `traits_pass` | Generic trait arguments, associated items (owned and shared-reference values), explicit borrow lifetimes, concrete external impl contracts, and a verified local refinement work across crates. |
| `refinement_lifetime_pass` | Refined implementation contracts preserve impl and method lifetimes and const generics across crates. |
| `refinement_lifetime_generic_wrong`, `refinement_lifetime_stronger_pre`, `refinement_const_wrong` | Reject a concrete-only guarantee in an unconstrained generic caller, a strengthened implementation precondition, and an incorrect const-generic result. |
| `refinement_mutable_pass` | Verify a concrete finite iterator model and consume its two-state trait contract through a generic cross-crate helper. |
| `refinement_mutable_frame_pass` | Verify a mutable trait method alongside a separate shared-reference frame condition. |
| `refinement_mutable_wrong_post`, `refinement_mutable_wrong_state` | Reject a weakened mutable implementation postcondition and an incorrect generic state claim. |
| `cross_crate_pass` | Generic wrappers preserve non-`Copy` payloads, return the old destination, and install the new value across crate boundaries. |
| `lifetimes_pass` | Renamed early/late-bound lifetimes, existing outlives bounds, type aliases, and nested higher-ranked signatures are accepted. |
| `conditional_pass`, `iterator_capability_pass` | Conditional postconditions are selected with caller evidence for trusted free functions and the external `Iterator::next` trait method, including old-state and cross-crate generic capability probes. The sealed iterator capability is available only to supported implementations. |
| `iterator_capability_unbound`, `iterator_capability_wrong`, `iterator_capability_lifetime` | Reject an iterator claim without model evidence, an incorrect pre-call state, and a trait guarantee available only for a `'static` reference. |
| `conditional_unbound`, `conditional_lifetime`, `conditional_generic_lifetime`, `conditional_nested_lifetime`, `conditional_lifetime_bound` | Reject conditional guarantees without sufficient evidence, including a hidden `'static` requirement on a reference-containing type and mismatched generic lifetime slots. |
| `conditional_pre_wrong`, `conditional_state_wrong` | Reject a conditional postcondition when its refinement has a conditional precondition, and reject an incorrect pre-call state. |
| `conditional_poly_pass`, `conditional_poly_unbound`, `conditional_poly_static`, `conditional_sized_static`, `conditional_equal_lifetimes` | Prove a universally applicable reference implementation with its non-region `Copy` bound, and reject missing bounds, hidden `'static` requirements (including alongside a satisfied `Sized` bound), and invented equality between independent lifetimes. |
| `cross_crate_wrong_value` | Reject a wrong returned value from an imported generic wrapper. |
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
| `duplicate_imported` | Reject a local declaration that duplicates bundled `Option::unwrap`. |
| `duplicate_bundled_replace` | Reject a local std declaration that duplicates bundled `core::mem::replace`. |
| `duplicate_imported_trait` | Reject a local declaration that duplicates a trait contract exported by `stdlib_specs`. |
| `signature_generics` | Reject a monomorphic declaration for a generic target. |
| `signature_specialized` | Reject specialization of a target's type parameter while retaining the same parameter count. |
| `signature_coercion` | Reject a parameter type mismatch hidden by a mutable-to-shared reference coercion. |
| `signature_return_coercion` | Reject a return type mismatch hidden by that coercion. |
| `bounds_stronger_trait` | Reject an added `T: Copy` requirement on `mem::replace`. |
| `bounds_stronger_associated` | Reject an added `Copy` requirement on a trait's associated item. |
| `bounds_stronger_supertrait` | Reject an added `Copy` supertrait on an externally specified trait. |
| `bounds_stronger_lifetime` | Reject an added `T: 'static` requirement on `mem::replace`. |
| `bounds_stronger_outlives` | Reject an added reverse outlives requirement between early-bound lifetimes. |
| `lifetime_merged` | Reject merging two independent input lifetimes. |
| `lifetime_static_input` | Reject restricting an arbitrary borrow to `'static`. |
| `lifetime_shortened_output` | Reject shortening a target's `'static` result to the input lifetime. |
| `lifetime_early_bound` | Reject replacing early-bound outlives parameters with one late-bound lifetime. |
| `lifetime_nested` | Reject changing a callback's higher-ranked lifetime requirements. |

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
`?Sized` caller, by-value receivers, and a type parameter following an early-bound
lifetime. The trait rewriter preserves supertraits
and permits unsized `Self`, adding `Sized` only for a by-value receiver.

The lifetime comparison normalizes associated types while preserving regions,
then anonymizes each binder independently. Early-bound lifetime parameters must
occupy the target's generic positions. Concrete trait implementations are checked
against the trait's public call signature. The bundled reference-equality specs
now make those impl lifetimes explicit. This exposed a spec-encoding bug: removing
lifetime slots shifted subsequent type/const indices. The encoder now retains
the full argument list when instantiating spec bodies.

Both signature and predicate checks are conservative. They may reject valid
declarations requiring different early-bound parameter layouts or more complex
outlives equivalences. They do not silently accept changed lifetime relationships.

Conditional postcondition selection is currently limited to trusted free impure
functions and directly called external trait methods with no conditional
preconditions. Region-bearing bounds must hold for all independently quantified
lifetimes, with no unproved region requirements or lifetime leaks. Unresolved
inference variables, other associated functions, external free functions and
impl methods, and pure calls remain excluded; conditional expiry pledges are
unsupported.

These checks establish the P1-B external-contract foundation, not unrestricted
collection or reference-containing generic support. Review trusted contracts
against the pinned Rust implementation; passing clients alone cannot establish
their soundness.
