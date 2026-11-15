# M3 correctness and migration audit

This is an audit of a narrower restart candidate, not qualification of current
main or of the 790-client restoration inventory. The selected compiler is
`be1364c7c` / PCG `e54bf675468d`; its compiler source matches `b15a36e4a`.
The independently rebuilt source and binary records are preserved alongside this
report. Historical evidence is kept separately from the new results.

## Accepted repaired baseline

The final source `2c5484c004ba5051ab7856f2c5938c33b05081e0` is qualified on
Ferrum and Silicon: each matches 387/387 checks and every unchanged original audit
control (15/15), with 22/22 focused controls overall. The compiler was rebuilt from
the integration branch; the two runs share source manifests and six binary hashes.
See [final qualification](STDLIB_M3_QUALIFICATION.json) and
[repair evidence](STDLIB_M3_REPAIR_EVIDENCE.json). The R0 results below remain
historical evidence for rejecting M3 unchanged; they are not current failures.

The failed-call propagation change `3b9208f05` remains an explicit migration audit
item: no standalone reproducer or demonstrated failure in this fixed baseline gate
has been established. The later no-glue drop implementation was unnecessary for
its passing controls. Broader restoration, unsupported features and the complete
790-client target remain outside this qualified checkpoint. Work is stopped before
R2 and D1–D6, with no automation scheduled.

## Scope

The later-change inventory captures 141 commits through `e06edd300` affecting
encoder, interface, VIR, contracts, adapter or PCG. It records touched files and
regression paths. This inventory is not an assertion that every change is correct
or that its feature works on M3. Fifteen unchanged controls from six relevant
later fixes are executed separately using the frozen M3 compiler, after the
historical suite gate. They do not change the historical acceptance list.

## Correctness fixes to evaluate before extending the baseline

| Area | Change references | Evidence and import rule |
|---|---|---|
| Entry versus current heap in quantified contracts | `bbb6b0520` | Execute the original quantified mutation positive and wrong postcondition. Preserve snapshot-expression evaluation when selecting `old`; a passing incorrect postcondition is a correctness blocker. |
| Expiry observations outside pledges | `c17aaa266` | Execute pre/post/invariant and quantified-context rejections plus a valid quantified pledge. Distinguish missing rejection, internal failure and exact intended diagnostic. |
| Mutable-read permission diagnostics | `dcf7fcc83` | Execute the original missing-permission pledge. An earlier wrong diagnostic is not a successful negative control. |
| Inferred reference lifetimes | `e9dd7929c`, narrowed by `2247e831d` | Execute the nested inferred-lifetime positive/negative. Any future import must retain the normalization guard. |
| Quantified expiry heaps | `800d9350b` | Execute the original heap positive and incorrect pledge. This is distinct from ordinary entry-heap capture. |
| No-glue drop | `f9270bd87` | Execute borrowed-state preservation and its exact false control. Do not generalize the optimization to unknown types or destructors. |
| Failed call encoding | `3b9208f05` | Future import must propagate failure instead of fabricating a replacement terminator. No standalone source reproducer accompanies this commit; retain it as an explicit audit requirement. |

## Changes required by later ownership capabilities

Do not copy the newer ownership engine wholesale into the baseline. Preserve
these dependencies and corresponding tests when restoring their feature family:

- `df3ca87a3`, `31c781ba9`, `e286dec43`: PCG lifetime and associated-result
  dependencies. PCG pins carry semantic changes, not merely build compatibility.
- `aa520ad34`: mutable-slice observations must read the selected heap with the
  required permissions. The earlier `6da75a162` was reverted by `674448d2b`;
  importing the reverted attempt would lose the later design and its controls.
- `358c34d50`: call-return snapshots differ from entry and expiry snapshots;
  inference cannot reconstruct an omitted temporal relation.
- `d2a7eb008`, `ff235ce02`, `c6d93efb4`: restoration inputs, physical loop-edge
  identity, and pre-advance placement must be checked. Duplicate, overlapping,
  wrong-edge and zero-sized controls remain in the future acceptance target.
- `de983ae3b`, `606494f2d`: reference replacement must retain historical targets
  and dependencies; the storage's current reference cannot redirect loan expiry.
- `225eb7714`, `8519eac5b`, `400ef288d`: immediate resources, promised heaps,
  and exported call-result groups must remain distinct during restoration.
- `08f24392d`: captured snapshots may improve cost, but nested wand expressions
  that rely on antecedent resources still require their proper temporal scope.
- `2299992a3`, `7d995a1de`: triggers must retain captured values and correct heaps;
  syntactic simplification is not permission or value justification.

R1 must also preserve the adapter's map-membership operand correction
`0ad015f11`, opaque-function/reveal handling `474b7ae1d`, and the separately
validated Ferrum boolean printer fix `931a2d5`. These are adapter/backend
requirements, not evidence that ownership redesign is already justified.

## Restrictions that a reproduced historical gate does not remove

The M3 `mut_wrong_owner` case expects rejection of the true post-mutation owner
value, recording an incomplete restoration model. Its matched diagnostic is not
a proof of owner restoration and not a false-property soundness control.
Similarly, historically expected unsupported cases record the supported boundary.
The later early-break prefix/suffix example, reference-containing/generic element
restoration, and all unresolved combined-matrix cases remain unqualified.

Trusted external library implementations and the reviewed hash-key model laws
remain assumptions of the original milestone. The baseline gate verifies its
specification libraries and clients; it does not verify unsafe stdlib internals.

## Results and decision

The historical source reproduced all 291 expected checks: hash 51/51, extern
97/97, iterator 66/66 (historical 61 plus five additions), and borrow 77/77.
All four specification libraries verified. The 36 specification unit tests passed.
There were no historical-suite timeouts, skipped cases or wrong diagnostics.
Eight historical checks intentionally expect unsupported-feature rejections;
they remain limitations rather than feature-support claims.

The later-control audit matched 3/15: the valid quantified pledge, no-glue drop
positive and its exact false control. The remaining outcomes are one verification
failure, two unexpected acceptances, five internal errors, two wrong diagnostics,
and two unsupported cases. No timeout occurred. Exact case results, expectations,
source hashes and origins are in [the evidence](STDLIB_M3_BASELINE_EVIDENCE.json).

**M3 must not be adopted unchanged.** In `quantified_old_mutation_wrong`, entry
requires a one-element slice containing 1, the body writes 7, and the accepted
postcondition incorrectly claims the element equals its entry value. Its paired
valid `old(value) + 6` postcondition is rejected. This establishes an incorrect
acceptance in existing contract syntax, independently of iterator redesign.
The observed behavior agrees with the entry/captured-expression defect addressed
by `bbb6b0520`; that commit's isolated backport still needs qualification here.
Archived `stdlib-quantified-old-inline-borrow-logs` records both controls passing
after the historical fix, but that older evidence is not added to this gate.

`before_expiry_quantified_post_context` also verifies despite using `before_expiry`
outside a pledge. This is an accepted unsupported context, distinct from the
incorrect functional postcondition above. The related plain pre/post contexts
produce internal errors, and both invariant controls report invariant failures
instead of the intended scope diagnostic. `c17aaa266` is the relevant guard fix.

The nested inferred-lifetime pair and missing-read-permission control produce
internal errors. The two more expressive quantified-expiry controls are
unsupported. The no-glue drop pair already passes M3, so this audit supplies no
reason to import `f9270bd87` for that pair alone.

R0's reproduction and audit are complete; qualification for adopting a new
baseline is incomplete. Before R1 backend compatibility, import the old-state
capture and expiry-context correctness repairs individually, rebuild, run their
native positive/negative controls, and rerun affected historical families. Keep
other audit failures explicit until repaired or their supported boundary is
established. Do not start ownership redesign or inference on a purportedly
qualified baseline while the incorrect acceptance remains.

Fresh suite elapsed times were 318.39s hash, 344.46s extern, 377.82s iterator and
807.96s borrow. Their longest checks were 16.23s, 11.52s, 25.10s and 45.02s.
These are observations from this build and environment, not demonstrated
architectural speedups. All source/binary preservation checks passed; the six
pending main files remain unchanged. No job is running.
