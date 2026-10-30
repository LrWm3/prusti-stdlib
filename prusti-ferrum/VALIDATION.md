# Current stdlib integration checkpoint

The final integration branch `codex/stage3-cleanup-fix` is based on Prusti
commit `3f623122a0b1473c44d15c4d1a11200addb8a030` and pins Ferrum
`5cbb29be28b45194e555d2e173ed39b1a64a80de`. It retains stdlib snapshot
`708b0063c1ebc76521e89291d4fab16e3b63bcc3` and pcg
`c18a51899e37e0fa67f9b5f17f7d518ef2f5518c`. The earlier matrix, map fix,
and final integration are distinct checkpoints:

- Initial matrix: Prusti integration `13674e5e2e46eff8f6140bece1a988e0db22893e`
  on `codex/stage3-verification`, with Ferrum
  `f686fde563e0e971d20a46eb8dd14e083a11e801`. The source-hash and coverage
  audit accounted for all 602 fixtures on each backend. The native-only and
  dual build artifacts and client logs are retained separately by backend.
- Map correction: Prusti `0ad015f11ac31ba76cbd942ee0c375e21b86b2a5` on
  `codex/stage3-map-fix` fixes the `MapContains` encoding. The initial matrix
  contained 21 affected client pairs. The complete corrected native hash
  suite passed all 51 checks: 40 clients and 11 controls. This does not retroactively replace
  the initial matrix outcomes.
- Final cleanup integration: `3f623122a0b1473c44d15c4d1a11200addb8a030` pins
  Ferrum `5cbb29be28b45194e555d2e173ed39b1a64a80de`, including the correction to repeated escaping of nested
  cleanup-error strings. Locked production native-only and dual builds and
  both launchers passed on nightly-2026-08-15. The real-Z3 `prusti-ferrum`
  library suite passed 17 tests with no failures; Ferrum optimization overrides
  applied only to the test build. An uninstrumented 60-second control returned
  its verifier-timeout diagnostic without reaching the outer watchdog; it is
  not an original-budget parity or performance result.

The initial matrix produced the following client annotation matches; controls
are counted separately. A mismatch includes timeouts, shared upstream or encoder
limitations, and source-version differences, so it is not automatically a
Ferrum defect.

| Suite | Clients per backend | Native matched | JVM matched | Native controls passed / total | JVM controls passed / total |
| --- | ---: | ---: | ---: | ---: | ---: |
| extern | 78 | 76 | 76 | 19 / 19 | 19 / 19 |
| sequence | 15 | 15 | 15 | 7 / 7 | 7 / 7 |
| vec | 14 | 14 | 14 | 7 / 7 | 7 / 7 |
| collections | 22 | 21 | 21 | 5 / 5 | 5 / 5 |
| borrow | 109 | 88 | 90 | 52 / 53 | 52 / 53 |
| iterator | 310 | 269 | 275 | 117 / 118 | 118 / 118 |
| hash | 40 | 19 | 40 | 10 / 11 | 11 / 11 |
| resources | 14 | 14 | 14 | 9 / 9 | 9 / 9 |
| **Total** | **602** | **516** | **545** | **226 / 229** | **228 / 229** |

The paired initial classification was 511 cases meeting both expectations,
35 shared failures, one shared stale-unsupported expectation supported by a
false-assertion control, 32 timeout-inconclusive pairs, two confirmed
source-version differences, and 21 map-encoding differences addressed by the
later map correction. The native borrow, iterator and hash specification
library controls failed, as did the JVM borrow library control; all runtime
and remaining controls passed. The JVM integration bundle SHA-256 is
`6d11d2b73efe97b97026050f13d966c5ec599cf70d43022346382e065a49cf7c`;
it differs from the standalone pinned Silicon reference. Matching case
outcomes alone do not establish identical Viper input.

Final-integration serial retries used fresh targets, the same 180-second
verifier limit and 300-second command limit, and retained the initial results.
Of 16 native iterator client retries, four met expectations, nine reached the
verifier limit and three completed with shared Prusti failures. Of 12 JVM
iterator retries, eight met expectations, two reached the verifier limit and
two completed with shared encoder failures. No retry hit the outer watchdog or
unexpectedly accepted a negative fixture. Five original/retry Viper dumps were
identical, 12 differed and 11 were unavailable; matching Rust fixtures alone do
not establish equivalent inputs. The native iterator specification-library
retry passed; its initial native run had reached the verifier limit.

The borrow retry covered the 12 backend attempts that initially timed out
across 10 cases. Native met expectations in five of seven; `open_from_pass`
and `vec_lookup_inclusive_pass` again reached the verifier limit. JVM met
expectations in three of five; `inclusive_exhausted_pass` again reached the
verifier limit, and `vec_lookup_inclusive_pass` reported the shared unsupported
restoration obligation at line 13:23 without timing out. Native reported the
same diagnostic alongside its timeout, as in its initial run. No borrow retry
hit the outer watchdog or unexpectedly accepted a negative fixture. All
remaining timeouts are inconclusive; a shared encoder rejection is not a
successful proof. No performance comparison follows from these runs.

See Ferrum's [Prusti validation record](https://github.com/LrWm3/ferrum/blob/989f58ced5b8d1081e74c6cdbfcfe1e6d8a62cf1/porting/prusti-validation.md)
for source comparison and reproducible evidence. The initial matrix summary,
coverage audit, per-case logs and source/artifact hashes are retained under
`/tmp/ferrum-prusti-stage3`; final retry records remain separate there.

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
