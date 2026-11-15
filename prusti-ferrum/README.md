# Native Ferrum backend

This crate converts VIR directly into Ferrum's Silver AST and runs its native
frontend, including plugins and consistency checks. The workspace pins Ferrum
to a Git revision for reproducible builds.

On `stdlib/ownership-inference-m3`, the adapter is imported through Prusti
`a59ed917c` with the map-membership and opaque/reveal corrections. Ferrum is pinned
to `931a2d5` (the boolean printer fix atop `0e048a4`). R2 restores the VIR
`PredicatePerm` observation and its native/JVM conversions for checked resource
view selection; observing permission does not acquire it. Historical
results in `VALIDATION.md` are not qualification of this branch; its current gate
is recorded in `STDLIB_EXTERN_SPEC.md` at the repository root.

Build the Prusti driver without JNI or a JVM:

```sh
cargo +nightly-2026-08-15 build -p prusti --no-default-features --features ferrum
```

Select it with `PRUSTI_VIPER_BACKEND=Ferrum` and set `Z3_EXE` to a compatible
Z3 executable. The ordinary build keeps the JVM backend enabled; add
`--features ferrum` to build both routes. A requested backend that was not
compiled in returns a backend error.

The adapter preserves identifier positions for Rust diagnostic backtranslation.
It handles domains, ADTs, collections, quantified expressions, method control
flow, fold/unfold, wand proofs, refute statements and function decreases.
Unimplemented conversion cases and unsupported output modes return errors.
Backend setup/verification/shutdown errors are not cached. Warnings are retained
through failed startup and normal cleanup.

Mapped counterexample backtranslation and structured method/quantifier reporting
are available through the existing Prusti options. Counterexamples use raw
converter data, and selected method results share error deduplication with the
final result. Viper program dumps, consistency-check-only mode and CSV/CFG debug
output use the existing Prusti options. This is an integration in progress; broader
compatibility and performance measurements remain outstanding. The existing SMT
wrapper options are supported; analyzer cleanup errors retain the primary result
and are reported as backend errors.

The current Rust encoder has a pre-existing panic-path issue: its `Never`
postcondition makes a subsequent `exhale false` unreachable. Both the exact
Scala baseline and this native adapter accepted the existing `assert!(false)`
fixture. Checked arithmetic overflow, including mutable-reference increment,
is a useful negative integration case and is rejected at matching Rust spans.

Set `FERRUM_BENCH_PHASES=1` with INFO logging enabled to record nanosecond timings
for Rust-to-VIR encoding, VIR-to-backend conversion and the backend verification
call. Driver logs contain encoding; server logs contain conversion and verification
when using a remote server. Set the variable in both processes for all phases.
The JVM verification measurement includes its wrapper/polling path; the native
measurement covers `frontend.verify`. Neither separates solver startup, preamble,
SMT solving or result mapping. Keep caches disabled, record binary/input hashes,
and measure startup, first request and later requests separately before comparing.
The initial paired warm-service smoke completed eight requests with caching off;
it establishes that the measurement path works, not a performance improvement.
