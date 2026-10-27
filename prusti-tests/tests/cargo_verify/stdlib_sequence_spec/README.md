# Sequence observation gate (P1-C)

Run from the repository root, after building Prusti:

```sh
./x.py build --all
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite sequence
```

The runner uses a fresh Cargo target directory, disables the verification cache,
and enables overflow and panic checking. It explicitly verifies the `no_std`
`sequence_specs` library with the `core` and `alloc` contract features before
checking the separately compiled clients. The same command runs in CI.
Logs, timings, and emitted Viper remain in `target/stdlib-sequence-spec-logs/`.

`Seq::from_slice(&[T])` observes element snapshots without consuming elements.
`prusti_contracts::alloc_spec::vec::vec_contents(&Vec<T, A>)` is a transparent
pure helper defined through `as_slice`; its allocator parameter matches Vec's.
The Vec helper exists when the `prusti` and `alloc` features are enabled, and is
used in annotations rather than ordinary runtime code. The slice helper, like
other ghost constructors, produces an opaque zero-sized value outside Prusti.

The supported domain for this gate is owned values with stable value snapshots.
No `Copy`, `Clone`, `PartialEq`, or `Eq` element bound is added. In particular:

- `observation_pass` proves length and arbitrary-index agreement, a quantified
  observation with an explicit indexing trigger, exported observations, and
  access to a non-Copy struct after observing and moving its array.
- `preservation_pass` proves arbitrary indexed elements agree when observations
  agree, and preserves the previous Vec observation through `mem::replace`.
  The library verifies both replacement effects using bundled contracts.
- `triggers_pass` exercises captured values, multi-triggers and existential
  triggers. `explicit_marker` occurs only in annotations, never in a quantified
  body. The runner checks that it appears inside both kinds of Viper trigger.
- Negative clients reject false lengths, false elements, a false replacement
  claim, and `false` after observing a slice. Each must report exactly the
  expected diagnostic at the annotated source line.
- Malformed trigger probes cover incomplete coverage, a term without any bound
  variable, unsupported expression shape, shadowed bindings, and unnamed
  quantifier arguments. Parser unit tests also cover field/method names that
  resemble binders and empty trigger sets.
- `unsupported_generic_mutref` and `unsupported_nested_mutref` require precise
  unsupported-feature errors, with no compiler panic or internal error. They
  test limits of this branch; they do not claim to validate the upstream PR.

The trigger implementation adapts [PR #1567](https://github.com/prusti/prusti/pull/1567),
with syntax-aware variable validation. Calls, method calls and indexing are
accepted as trigger terms. Explicitly triggered binders must be named; local
bindings and macros inside triggers are rejected conservatively.

[PR #1573](https://github.com/prusti/prusti/pull/1573) was reviewed for its split
between shallow/deep snapshots and two-state trait specifications. It was not
imported: the existing owned-value snapshots and `old(&*dest)` machinery suffice
for this gate's replacement proofs. Generic deep snapshots and nested mutable
reference permissions remain separate work, along with reference-containing
collection elements, interior mutability, and nested collection models.

An exploratory nested-reference function body also produced a permission
backtranslation error. The retained reproducer is `known_limits/nested_mutref_body.rs`;
it is outside this gate's accepted domain and is not counted as a passing check.
The isolated specification-only probe above checks the snapshot diagnostic.

This fixture establishes P1-C. Vec push/pop and the full M1 collection milestone
are covered by the adjacent [P1-D gate](../stdlib_vec_spec/README.md).
