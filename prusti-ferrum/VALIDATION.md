# Current upgrade candidate 0e048a4 (qualification pending)

Ferrum `0e048a420c081f4c442dd04277bb016346de4e10` includes the permission-map
repair in `85fc9d8`, stable allocation ordering, preserved heap-dependent
snapshot triggers, and two subsequent Viper-text frontend repairs. The
[upstream follow-up](https://github.com/LrWm3/ferrum/blob/0f2a563e5e112a7cca59e42f09d63470ceb6b137/porting/stdlib-followup-review.md)
rechecks our checkpoint 349 in fresh requests: `vec_lookup_range_pass`,
`mut_element_replace_pass`, and `mut_element_replace_return_pass` verify at
`85fc9d8`; the replacement-false and inclusive-frame negatives reach exactly
their expected assertions. The inclusive negative also retains the exact Viper
bytes from the earlier unstable comparison. Those targeted results support the
upgrade; they do not qualify the full matrix or establish general solver
stability. The upstream benchmark's strict dependency-event check failed after
rebuilding an invalidated cache, so its wall times are not accepted performance
measurements.

The later frontend changes avoid repeated nested ADT receiver checking
(`09a28fb`) and share immutable type-substitution maps (`0e048a4`). They repair
raw Viper replay overhead. Prusti's injected-AST adapter bypasses that frontend;
these changes do not establish faster Rust verification through the adapter.

This branch has been rebuilt with all three Ferrum dependencies pinned to
`0e048a4`, using `./x.py build --all --locked --features prusti/ferrum` and
nightly-2026-08-15. Build 352 includes the committed encoder and PCG `269cb9a`,
without either pending encoder/PCG experiment. Its source is frozen at
`/tmp/prusti-stdlib-probe352`, based on `4927ecfb0` plus a dependency-only patch.
Executables are under `/tmp/prusti-stdlib-matrix352-bin`; the driver's SHA-256 is
`fad7742d27b936ffe9c9cf9ba9862eb421c0c70ac004080715e29360345c78f0`.
`target/stdlib-matrix-20261031-1236/build352.json` records all six hashes.
All 18 adapter tests are compiled but have not yet executed. The standalone
CLI was built from the same Ferrum pin into an external target directory;
Rust validation uses the rebuilt Prusti binaries.

The serial gate runs ordinary and real-Z3 adapter tests with one CPU and one
test thread, then paired raw diagnostics and Rust checks. Rust coverage includes
concrete replacement, optional/nested elements, the full Pair prefix positive
and wrong-second negative, native framing, iterator models, and borrow/helper
controls. Every generated body remains checked. Negative controls must match
exact expected diagnostics. Resource clients retain the 600-second program,
30-second assertion and 100-millisecond check bounds, one verifier, and branch
parallelism disabled.

The handoff models had a local named `current` colliding with a let-bound name
inside the wand. That local has been renamed before verification; the original
models are retained. Syntax rejection supplies no permission-proof result.
The corrected positive, false-assertion and missing-resource models are part
of the paired diagnostics in this gate.

Builds 353 and 354 rebase the independent nested-typing and opaque-loop-coupling
candidates onto the same final Ferrum pin. Their separate paired gates follow
build 352. Earlier unstarted dispatchers 346/349/350/351 were stopped while
waiting, and their artifacts remain preserved. None produced proof results.
The broader concrete reservation in build 348 also remains deferred and
unverified.

The frozen full matrix at `afdf10c` continues unchanged, before these focused
gates use the single worker. Its old-backend failures are baseline evidence,
not regressions on current Ferrum. The committed pin remains `afdf10c` until
this branch's gate is reviewed, and a full matrix on the eventual accepted
source and build is still required before merge.

# Performance upgrade to 2df948c deferred after qualification

The user authorized upgrading Ferrum after the generic-loop checkpoint
`1587799a8`. Candidate `2df948c9014de33b7baa13599eaf9d5078906b28` was rebuilt
and tested, but is **not adopted**: a previously accepted negative control
fails at the wrong obligation. The dependency manifest and lockfile have
been restored exactly to `afdf10cf59106c38d09df32939f1aa9c77d0b989`.
The candidate changed only the three Ferrum Git revisions in those files.
PCG remains at `c18a51899e37e0fa67f9b5f17f7d518ef2f5518c`.

This revision includes structural-term hash caching, reduced path-condition
copying, stable emission ordering, and the now-default selective snapshot
summary triggers. The experimental `groundPermissionSummaries` and
`separateMethodSolver` options remain disabled. Use one verifier worker and
keep branch parallelism off. The upstream
[trigger report](https://github.com/LrWm3/ferrum/blob/2df948c9014de33b7baa13599eaf9d5078906b28/porting/performance-summary-triggers.md)
and [sample of this branch](https://github.com/LrWm3/ferrum/blob/2df948c9014de33b7baa13599eaf9d5078906b28/porting/performance-trigger-stdlib-sample.md)
motivate this candidate; they do not qualify the current full stdlib matrix.

Prusti was rebuilt from this `main` checkout with
`./x.py build --all --locked --features prusti/ferrum`, using
nightly-2026-08-15. The six frozen executables are under
`/tmp/prusti-im5-checkpoint255`; provenance is in
`/tmp/prusti-im5-upgrade255/build.json`. The rebuilt driver's SHA-256 is
`3a7f358ab4da6d1ccf265472c5693210897a976ea868d4f0d322e426371e461e`.
The prior checkpoint 229 remains available for comparison.

All seven adapter unit tests and all eleven real-Z3 integration tests pass.
The integration tests run sequentially with single-CPU affinity so Ferrum's
default worker count is one. They cover opaque/revealed function behavior,
body checking, wands, collections, ADTs, source positions and counterexamples.
Logs are `/tmp/prusti-im5-upgrade255-adapter-{unit,z3}.log`.

The checks compare the old and candidate binaries on unchanged iterator and
borrow fixtures and validate recent combined resource proofs at their
established bounds. They require exact negative diagnostics, full helper-body
checking, fresh targets and disabled outer caching. Selected source and
artifact hashes are under `target/stdlib-im5-upgrade255`; pre-existing failures
remain recorded. No full-matrix or general performance claim is made.

The matching smoke runs are complete: the candidate passes all ten client
expectations and seven Rust/runtime controls. The old pin passes nine clients
and all controls, reproducing its existing 180-second timeout in
`range_wrong_frame`'s generated `resource_unpack_aggregate_3379c9af52ac4b15`
helper. The candidate reaches that control's intended assertion rejection in
8.36 seconds. The timeout remains an unresolved baseline observation, not a
negative pass or a completed performance measurement.

The nested iterator positive takes 9.90 seconds on the old pin and 5.12 on
the candidate with byte-identical Viper input; the array positive also has
identical input (6.94/6.87 seconds). These are single observations on a shared
host. The full cross-crate resource suite passes all eleven checks on the
candidate, including both untrusted library bodies and every conversion
helper. Its library takes 368.25 seconds versus 342.97 in the prior accepted
run, so these observations do not establish a uniform improvement.

Results and source comparisons are under
`target/stdlib-im5-upgrade255/{baseline,candidate}` and
`target/stdlib-im5-upgrade255/smoke-comparison.json`. The source manifest covers
549 fixture, contract and harness files, excluding generated targets.
The previous cross-crate evidence is retained at
`target/stdlib-im5-pair-cross248`; unchanged client source hashes and all
eleven outcomes match the new run.

The standalone prefix run passes both positive proofs, the generic wrong-state
control and the Pair false-assertion control, but does **not** pass
`mut_native_resource_pair_prefix_wrong_second`. It reports loop restoration at
line 18 after 460.82 seconds, instead of the expected assertion at line 44.
This is an unresolved qualification failure, not a successful negative test.
The run checks all generated bodies with 600-second program, 30-second
assertion and 100-millisecond check bounds. A fresh comparison using the old
frozen binary and unchanged current source reaches the exact expected
assertion rejection in 268.87 seconds at the same bounds. Its results are in
`baseline/prefix-wrong-second`; the candidate mismatch remains in
`candidate/prefix`. All 549 source-manifest hashes still match. This is an
observed qualification regression; emitted-program variability and solver
behavior have not been isolated as causes. The fresh Silicon check was
interrupted when the user directed work back to IM; it has no proof outcome.

Upstream's subsequent, documentation-only
[timeout investigation](https://github.com/LrWm3/ferrum/blob/26282c2/porting/timeout-followup.md)
also records incomplete-quantifier failures with selective triggers on a
different linked-list fixture and remaining allocation-order variability.
Neither finding establishes the cause of this Prusti control's failure. Do
not adopt the new pin merely from the faster smoke results.

The old and candidate frozen binaries, candidate dependency patch, logs and
source manifest remain available for follow-up. A build of earlier performance
revision `4cda819` was also interrupted at the user's direction; it was not
qualified. Both dependency files are restored to the old pin. Cancellation
details are in `target/stdlib-im5-upgrade255/interrupted.json`.
Routine IM work resumes on the old pin with focused checks, without the full
verification matrix. When revisiting the upgrade, compare selective triggers
on/off on the full failing program,
retaining every result and exact diagnostics; alternatively qualify an earlier
performance revision before the changed trigger default. Any selected
revision/configuration needs fresh current iterator/borrow and heavy resource
qualification. The full stdlib matrix and dual-backend validation remain
required before merging a later upgrade. No archived known-limit control was
promoted and no timeout or wrong-obligation rejection counts as a pass.

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

The iterator, borrow, and cross-crate resource adoption suites have been checked
on both backends. The unchanged Silicon run uses frozen checkpoint 226; the
corrected Ferrum run uses checkpoint 229. Both use one verifier worker, with
Ferrum branch parallelism disabled. The original native iterator results are
retained separately after that run was stopped to apply the annotation fix.

All six suite/backend runs completed, covering 432 client pairs and 609 checks
per backend. Counts below are matches against the original fixture expectations;
compilation, runtime, and specification-library controls are separate. The
iterator and borrow suites use the standard 180-second program limit and
default assertion resource bounds. The cross-crate resource suite uses a
600-second program limit and 30-second solver time bounds. All runs disable
verification caching, use fresh targets, and check overflows and panics.

| Suite | Backend | Positives | Negatives | All clients | Controls |
| --- | --- | ---: | ---: | ---: | ---: |
| iterator | Ferrum | 102 / 119 | 182 / 201 | 284 / 320 | 121 / 121 |
| iterator | Silicon | 104 / 119 | 185 / 201 | 289 / 320 | 121 / 121 |
| borrow | Ferrum | 41 / 51 | 51 / 58 | 92 / 109 | 52 / 53 |
| borrow | Silicon | 41 / 51 | 50 / 58 | 91 / 109 | 52 / 53 |
| iterator_resources | Ferrum | 1 / 1 | 2 / 2 | 3 / 3 | 3 / 3 |
| iterator_resources | Silicon | 1 / 1 | 2 / 2 | 3 / 3 | 3 / 3 |
| **Total** | **Ferrum** | **144 / 171** | **235 / 261** | **379 / 432** | **176 / 177** |
| **Total** | **Silicon** | **146 / 171** | **237 / 261** | **383 / 432** | **176 / 177** |

In the iterator suite, Ferrum's 36 mismatches comprise 23 unsupported cases, seven timeouts, three
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

The borrow comparison has 90 matches on both backends, 15 shared mismatches,
and four timeout-inconclusive pairs. Both specification-library controls fail
the existing unsupported partial-restoration obligation in `replace_lookup`.
One shared mismatch is the previously stale `call_wand_shifted_projection`
unsupported expectation: both backends accept its correct value assertion.
That original outcome remains in this baseline; follow-up controls are separate.
There is no completed native-only borrow failure in this comparison.

The promoted cross-crate resource suite passes all six checks on both backends.
Its untrusted library body and generated helpers verify in 57.23 seconds on
Ferrum and 79.90 seconds on Silicon. The positive and two exact negative clients
take 1.48/1.48/1.95 seconds on Ferrum and 6.86/12.09/12.92 seconds on Silicon.
The Rust/runtime checks also pass. These timings are evidence for these runs,
not a controlled performance benchmark.

Across all 432 client pairs, 375 match on both backends, 39 are shared
mismatches (including the stale expectation), 16 are timeout-inconclusive, and
the two reference-replacement positives are completed native-only failures.
The baseline is not entirely passing and does not close the outstanding IM
gates. Complete results are under `target/stdlib-im5-dual229/ferrum` and
`target/stdlib-im5-dual226/silicon`. Before any fixture correction, the audit
confirmed all 463 fixture/contract source hashes and all six frozen executable
hashes for each build. The immutable records are
`/tmp/prusti-im5-dual226/source-manifest.json`,
`/tmp/prusti-im5-dual229/audit-baseline-complete.json`, and
`/tmp/prusti-im5-dual229/completion.json`.

After freezing that audit, the stale projection fixture was replaced by
`call_wand_shifted_projection_pass` and `call_wand_shifted_projection_wrong`.
The latter changes the asserted value from 3 to 4 and expects the panic
precondition error at that assertion. Both final repository fixtures pass their
intended outcomes on frozen build 229, with four checks passing per backend
(Rust compilation, runtime, positive, negative). Positive/negative times are
9.72/0.71 seconds on Ferrum and 13.63/4.93 seconds on Silicon. Evidence is in
`target/stdlib-im5-shift231/{ferrum,silicon}`. This changes only test expectations
and coverage; the original baseline results above remain unchanged.

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
