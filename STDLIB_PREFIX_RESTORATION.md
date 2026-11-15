# R3: original resource-prefix proof

Baseline: R2 compiler `ecec87c3fe84379b607cb30e81951fa544ce7979`, report
`b24809d11`. Integration remains `stdlib/ownership-inference-m3`; experiments use
`stdlib/ownership-prefix-r3` in the existing isolated repair worktree.
Artifacts: `../prusti-stdlib/target/stdlib-ownership-prefix-r3/`.

The original 29 `mut_native_resource_*` clients are frozen byte-for-byte from
`c2631cfb0`, checked against `STDLIB_RESTORATION_ACCEPTANCE.json`. This family
supplements, and does not replace, the unchanged 790-client restoration target.
The original early-break positive and exact false control both have a recorded
600-second qualification limit; preserve it with a separate 720-second outer
watchdog. Other cases retain their recorded 180/600-second schedule.

| Current evidence | Result |
|---|---|
| R2 Ferrum / Silicon gates | Each 454/454; each 70/70 focused |
| Original prefix positive on R2 | Compilation/specification failure |
| Original exact false control on R2 | Wrong diagnostic |
| Prefix proof, inference, full restoration | Unqualified |

`original-pair-baseline-01` cannot reach verification: the quantifier parser rejects
`move` capture and the branch lacks `ResourceState`, resource observations,
mutable-iterator observations and `body_restore_prefix!`. These are prerequisites,
not a new solver timeout. Neither result counts as a passing negative control.

## Ownership invariant before restoration repairs

The physical owners are the slice's reference slots and the separate scalar
referents. A reference-slot snapshot identifies its historical target; it does
not describe or grant permission to that target's current value. The slice's
entry sequence fixes the original slot values and ordering. Entry resource states
capture the initial referent values under their permissions.

After creating the iterator, its saved post-call history identifies the original
owner and the remaining tail. Selecting one slot transfers its resource and the
corresponding referent resources to the yield, retaining the remainder's frame.
After `**value = 7`, expiry must restore the slot and its referent resources before
that yield joins the processed prefix. The prefix stores immutable slot and
resource-state values; these are not permissions. The remaining tail preserves
its original suffix states. At the loop head a bounded checked summary must
account for prefix, any outstanding yield, and tail without accumulating an
unbounded list of call histories.

On early exit, discharging the iterator obligation rejoins the processed prefix
and untouched suffix into the original physical owner. Only then may the original
owner be read to establish unchanged slots, updated referents equal to 7, and the
unchanged suffix. Reference replacement must retain obligations for displaced
and replacement targets. Distinct logical indices do not imply distinct physical
addresses, including for generic or zero-sized elements.

## Ordered experiments

1. Restore `move` quantifier capture from the later parser, preserving it on both
   body and trigger closures. A small native positive/negative pair isolates this
   syntax needed by the original proof. Preserve the existing trigger validation
   and quantifier well-definedness checks; add no ownership or solver assumptions.
2. Specify and qualify permission-checked resource observations and the minimum
   indexed-resource representation. Use concrete scalar/reference/sequence cases
   first, with wrong-state and missing-permission controls, then generic and ZST
   controls. An observation must never manufacture a permission.
3. Restore mutable iterator operations and manually supplied bounded prefix
   summaries one checked rule at a time. Keep original callee/caller expiry
   obligations and exact value/frame controls. Do not import the newer restoration
   encoder wholesale or claim inference repairs missing ownership resources.
4. Run the unchanged full Rust early-break pair, targeted prefix/suffix/replacement
   controls and three cold native/CLI repeats. Compare proof structure and costs
   under matching inputs; under 60 seconds remains a stretch target.
5. Proceed to bounded value inference and complete restoration qualification only
   after the required proof and controls pass. Silicon is reserved for major
   feature completion and final qualification. Stop before D1–D6; no automation.

Keep one integration candidate, one worker and Ferrum branch parallelism disabled.
After two failed hypotheses for a symptom, review the model before any successor
patch. Raw Viper edits are diagnostic only. Preserve exact programs, source and
binary hashes, settings, logs and every unsuccessful result. No timeouts, skipped
cases or wrong diagnostics satisfy acceptance.

## Quantifier prerequisite evidence

The small native controls reproduce the capture syntax failure on R2. The narrow
parser backport from `d4d4bc72bb` passes all 37 specification tests and repairs the
universal positive and exact negative. Its initial existential positive still
fails. `quantifier-witness-diagnostic-01` distinguishes the causes: removing
`move` has the same failure, while adding the checked `marker(bound) == 4`
assertion proves the unchanged existential with its explicit trigger.

The positive control therefore includes that checked ground instance; its two
original quantified assertions and the negative are unchanged. This is a proof
annotation, not an encoder inference result or weakened obligation. The original
unannotated source/results and both diagnostic variants remain preserved. The
original prefix sources and 29-case target are untouched. Automatic witness
selection remains outside this parser repair and may be considered in R4.
