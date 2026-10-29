# Current stdlib integration build checkpoint

The current integration combines committed stdlib snapshot
`708b0063c1ebc76521e89291d4fab16e3b63bcc3` with adapter `1b994e389` and pins
Ferrum `f686fde563e0e971d20a46eb8dd14e083a11e801`. The separate working copy
preserves uncommitted changes in the active stdlib checkout. The pcg submodule
matches `c18a51899e37e0fa67f9b5f17f7d518ef2f5518c`.

Both locked nightly-2026-08-15 builds pass: `cargo build -p prusti
--no-default-features --features ferrum` and `cargo build -p prusti --features
ferrum`. The dual build uses the configured JDK17 library path. Ferrum's exact
code checkpoint passes all 826 library tests with real stdio and linked Z3;
its production optimization settings remain unchanged.

Fresh verification of the 602 current client binaries across eight stdlib suites
is pending. The harness now explicitly selects `--backend Ferrum` or
`--backend Silicon` after clearing inherited Prusti options, keeps backend/run
logs separate, saves results incrementally and records source/artifact hashes.
Its two timeout-cleanup regressions pass. The JVM integration bundle SHA256 is
`6d11d2b73efe97b97026050f13d966c5ec599cf70d43022346382e065a49cf7c`; it differs
from the standalone pinned Silicon reference used for Ferrum compatibility tests.
No performance claim is made from these build checks.

The remaining record below is historical and must not be read as fresh
verification of the current integration.

# Native iterator integration checkpoint

The fixtures come from Prusti `40333f884e3e0a359c1724bf77bc0be5c53039c9`,
`prusti-tests/tests/cargo_verify/stdlib_iterator_spec`. Ferrum is
`418ab77896b7e7b1b5718eb7aa3ac1086e524783`. Verification caches were disabled and
dependency verification was disabled in both configurations. The JVM bundle
identifies Silicon `6b540aa4`; it is not the standalone pinned reference jar.

| Fixture | Native / JVM | Viper dump comparison |
| --- | --- | --- |
| `borrowed_direct_pass` | accept / accept | equal without standalone comments |
| `borrowed_direct_wrong_state` | reject / reject | equal without standalone comments |
| `exact_length_pass` | accept / accept | other differences remain |
| `exact_length_wrong` | reject / reject | other differences remain |
| `mut_direct_pass` | accept / accept | other differences remain |
| `mut_element_array_pass` | accept / accept | byte-identical |
| `mut_element_array_wrong` | reject / reject | byte-identical |
| `mut_element_option_pass` | accept / accept | equal without standalone comments |

The five positive and three negative outcomes agree. Separate encoding runs
produce non-comment differences for three cases, including a branch guard in
`mut_direct_pass`; these have not been normalized away or proved equivalent. A
shared-dependency rerun made `mut_element_option_pass` byte-identical, but did not
eliminate the mutable-direct branch-guard difference.

At the same Ferrum revision, both `mut_element_option_pass` and `mut_direct_pass`
overflowed the 128 MiB verifier stack with all three native crates at optimization
level 0, and both verified with all three at level 1. The latter retains debug
assertions and unwinding. This supports the development profile mitigation, not
a general stack bound or a performance ratio.

The raw local fixture, binary/hash, command, dump and outcome records are in
`/tmp/ferrum-rust-matrix`, especially `dump-comparison.json`, `dump-results.json`,
`shared-dump-results.json` and `latest-opt-results.json`. The initial warm-service
smoke ran four independent service starts and eight requests with caching off;
all completed and emitted the opt-in phase events. It ran on a loaded host and
does not establish a speedup or a detailed solver-time split.

The adapter now pins Ferrum `f4a070c1bb34a733af09eb78d86ef3ec69d34435` and passes
`--logLevel ERROR` to both native Ferrum and JVM Silicon. This checkpoint adds
owned CFG and loop guards, composed expression inputs and returns, generic
production/consumption receivers, and wand/Applying stages. Distinct evaluation,
quantified and loop-phase collectors retain their identities across worker
transfers. The shared wand completion helpers are included in this revision.
Registry dependency versions, including libc 0.2.191, remain unchanged.

Both locked checks pass on nightly-2026-08-15: native-only `prusti-server` with
`--no-default-features --features ferrum`, and `prusti --features ferrum` with
native and JVM adapters together. Ferrum's production-profile all-target check
with `native-z3` passes. Its workspace test run passes 690 regular tests (142
ignored), including 309 Silicon library tests (nine ignored), using test-only
optimization overrides. An explicit real-Z3 pass adds 60 CLI continuation tests,
11 warning tests, mask-alias replay and the cooperative collector-restoration
regression. The mask fixtures also pass linked Z3 4.16 with one and two workers.

The continuation documentation records source-disabled branches, borrowed API
boundaries, quantified/join cardinality limits and the internal active-logger
fallback. These are build and backend compatibility checks. The iterator matrix
above remains the earlier recorded checkpoint; this pin change does not claim a
fresh full Rust iterator matrix, complete Viper compatibility or a measured
verification speedup.
