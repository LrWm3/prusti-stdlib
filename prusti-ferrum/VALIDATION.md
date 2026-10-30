# Stdlib encoder adoption at afdf10c

The `stdlib/opaque-resource-encoder` branch integrates the tested adapter through
Prusti `a59ed917c`, retaining the branch's newer iterator/resource encoder work.
All three Ferrum crates remain pinned to
`afdf10cf59106c38d09df32939f1aa9c77d0b989`; PCG remains
`c18a51899e37e0fa67f9b5f17f7d518ef2f5518c`.

Prusti `474b7ae1ddba6b82599de7f5b4c65c8527546893` was built from this branch with
`./x.py build --all --locked --features prusti/ferrum` on nightly-2026-08-15.
Its driver SHA-256 is
`68d24cd16b0f2f12afc6d8363b41fd629dee98f26d3b12656f70aff214e72ef3`.
The build record and frozen binaries are in `/tmp/prusti-im5-dual229` and
`/tmp/prusti-im5-checkpoint229`. These are fresh branch artifacts, not the
earlier benchmark binaries.

The adoption checks exposed missing conversion of VIR's opaque-function and
reveal-call annotations. Commit `474b7ae1d` preserves both using the existing
Ferrum metadata API, without changing the backend pin or encoder contracts.
A real-Z3 test passes five scenarios: transparent and revealed calls verify;
hidden-body use and a wrong revealed value fail; an incorrect opaque function
postcondition fails even when the function is unused. The three generic-array
positives that previously timed out on native conversion now pass on both
backends, with byte-identical Viper inputs:

| Client | Ferrum seconds | Silicon seconds |
| --- | ---: | ---: |
| `mut_generic_array_pass` | 106.65 | 107.86 |
| `mut_generic_array_state_pass` | 119.93 | 114.79 |
| `mut_generic_array_symbolic_pass` | 54.27 | 51.55 |

These are full generated-program checks, including observer and conversion
bodies, at the standard 180-second program limit. Timings include client
compilation and do not establish a performance ratio. Exact hashes and
annotation counts are in `/tmp/prusti-im5-dual229/annotation-comparison.json`.

The current iterator, borrow, and cross-crate resource suites are being checked
on both backends. The unchanged Silicon run uses frozen checkpoint 226; the
corrected Ferrum run uses checkpoint 229. Both use one verifier worker, with
Ferrum branch parallelism disabled. The original native iterator results are
retained separately after that run was stopped to apply the annotation fix.

Both iterator baselines are complete. Counts below are client expectation
matches; compilation, runtime, and specification-library controls are separate.

| Backend | Positives | Negatives | All clients | Controls |
| --- | ---: | ---: | ---: | ---: |
| Ferrum | 102 / 119 | 182 / 201 | 284 / 320 | 121 / 121 |
| Silicon | 104 / 119 | 185 / 201 | 289 / 320 | 121 / 121 |

Ferrum's 36 mismatches comprise 23 unsupported cases, seven timeouts, three
diagnostic mismatches, and three internal errors. Silicon's 31 comprise 20
unsupported cases, seven timeouts, three diagnostic mismatches, and one internal
error. The timeout category takes precedence when a log also contains an
unsupported diagnostic. An unsupported rejection does not satisfy a negative
test expecting an assertion failure. A timeout remains unresolved.

The paired classification is 282 cases matching on both backends, 24 shared
mismatches, 12 timeout-inconclusive pairs, and two completed native-only failures:
`mut_element_replace_pass` and `mut_element_replace_return_pass`. Both fail an
implicit permission postcondition natively and verify on Silicon. The return
variant's Viper inputs are byte-identical; the other pair differs only in
standalone comments. Neither run unexpectedly accepts an iterator negative.
These two cases retain the Silicon gate. The full whole-prefix referent-value
positive verifies on both, while its incorrect recorded-state control times
out on both; its wrong-final-value control times out only on Ferrum.

Complete iterator results are in `target/stdlib-im5-dual229/ferrum/iterator` and
`target/stdlib-im5-dual226/silicon/iterator`; borrow and cross-crate resource
results are still pending. The frozen source manifest and paired audit are in
`/tmp/prusti-im5-dual226/source-manifest.json` and
`/tmp/prusti-im5-dual229/audit.json`.

The full stdlib matrix at the final pin remains outstanding. Use Ferrum for
routine development, retain Silicon for changed specifications, negative tests
and unexpected differences, and require dual-backend validation before merging.

# Stage 4 backend integration checkpoint

Prusti `e76038c31a4cb1bb3e34745940b1f883094dd2cb` pins all three Ferrum
crates to `dc20a0fa9fdc5a6a290b24c7bcf92a5afd15bbb7`. On
nightly-2026-08-15, locked native-only and dual-backend production driver
builds and both launchers passed; all 17 real-Z3 adapter library tests passed
with test-only Ferrum optimization overrides. The first dual build could not
load `libjvm.so`; the retry passed with the JDK server library directory in
`LD_LIBRARY_PATH`. No full stdlib matrix or benchmark was run at this pin.
The earlier 602-fixture matrix below remains historical evidence for its
recorded revisions, not a result for the new backend. Source and artifact
hashes, commands, and logs are retained under `/tmp/prusti-stage4-preparation`.
The native and dual driver SHA-256 values are respectively
`1a14204ab0678fcc1cdc6b983714c4aac41d716c2169815a20cfaf77e219042d` and
`eeb0fe7fe668cb016a37866cc999856d5f9551fe6051fcfa47cb971a36dc72bf`.

# Stage 3 stdlib integration checkpoint

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
