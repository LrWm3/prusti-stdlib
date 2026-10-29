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

The adapter now pins Ferrum `afdf24ddba1516a7987a17d43e5090b8725b6980` and passes
`--logLevel ERROR` to both native Ferrum and JVM Silicon. This checkpoint adds
owned Let, strict-operator, field, domain/backend and CurrentPerm argument returns,
fixes mask-alias declaration replay between workers, and preserves warning
categories, occurrence timing and positions. It rejects options that disable the
global declarations required by branch replay. The lockfile updates libc from
0.2.176 to the Ferrum Unix logger's required 0.2.191; other registry versions stay
unchanged.

Both the native-only `prusti-server` check and the full `prusti --features ferrum`
check (native and JVM adapters together) pass on nightly-2026-08-15. Ferrum's
checkpoint passes 215 library tests (nine ignored), 18 real-Z3 CLI continuation
regressions and focused warning/configuration checks. The cooperative restoration
regression and one/two-worker mask fixtures through stdio and linked Z3 4.16 also
pass. These are build and backend compatibility checks. The iterator results above
remain the recorded earlier checkpoint; this pin change does not claim a fresh
full Rust iterator matrix or a measured verification speedup.
