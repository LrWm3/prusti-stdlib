# Mutable iterator reference snapshots

These historical probes record the steps leading to the current native
iterator model. The `mut_native_*` regressions in `../src/bin` now exercise a
logical borrowed tail for `core::slice::IterMut`, with bundled contracts for
yielding elements and restoring their source. The older `mutable_ref_snapshot`
and `expiry_ref_heap` probes below define their own contracts for the same
methods; their historical commands apply to revisions before bundling, since
those definitions now conflict with the bundled contracts. Pure mutable-reference
snapshots still cannot read referent values without permission. Whole-prefix
value proofs now use explicit `body_restore_prefix!` summaries; the unannotated
loop probes below retain their baseline limitations. Resources for mutable
references inside concrete or generic elements remain incomplete.

## IM0: native loop values and references inside elements

The `im0_*` binaries reproduce the two follow-up gaps from
[the plan](../../../../../STDLIB_EXTERN_SPEC.md#mutable-iterator-follow-up-plan).
They are valid Rust programs. Except for `im0_shared_ref_false`, they describe
correct behavior that should eventually verify; a failure or internal error
is evidence of a limitation, not a successful negative test.

On 2026-10-28, Prusti `22ea24a6b` with PCG `a4eb802`, Rust
`nightly-2026-08-15`, Viper `v-2026-09-01-0834`, Z3 `4.16.0`, and Java
`21.0.2` produced these results with fresh targets, the cache disabled,
overflow and panic checks enabled, a 180-second verifier timeout, and one
parallel verifier per process:

| Probe | Observed result | Meaning |
|---|---|---|
| `im0_native_unrolled`, `im0_native_unrolled_token` | Verify | Two native `next` calls update both scalar or non-`Copy` elements and establish the final postcondition. |
| `im0_native_loop_scalar`, `im0_native_loop_token` | Postcondition might not hold | The symbolic loop restores ownership, but a length invariant alone does not establish that every scalar or non-`Copy` element was updated. |
| `im0_shared_ref_index` | Verifies | Shared references in a slice can be read in this concrete example. |
| `im0_shared_ref_false` | Assertion might not hold | The shared-reference example does not prove an arbitrary false assertion. |
| `im0_mut_ref_index`, `im0_mut_ref_next`, `im0_holder_next` | Unsupported mutable-reference dereference in pure code | The assertion reads a referent through a mutable reference obtained from a slice snapshot; its correct heap-dependent interpretation is not encoded. |
| `im0_mut_ref_index_write`, `im0_mut_ref_local`, `im0_mut_ref_next_write`, `im0_holder_index_write`, `im0_holder_next_write`, `im0_optional_index_write`, `im0_optional_next` | Internal error reporting `call.precondition:insufficient.permission` | Valid writes or reborrows need a referent permission absent from the generated program. The error also lacks an appropriate source-level backtranslation. |
| `im0_shared_ref_next` | Verifies after call-wand selection fix | Shared-reference single- and two-yield regressions now live in the main iterator suite. |

The reference failures are independent of the loop proof: the write-only
index case contains no iterator and no referent-value assertion. Its generated
method requires the outer reference predicate and the slice predicate, but not
the predicate for the `u32` behind the element reference. Index unfolding
exposes the reference slot; `make_concrete_UInt_u32` then requires the missing
referent predicate. An `Option<&mut u32>` yielded by the iterator reaches the
same kind of missing permission. A direct nested dereference in a specification
fails earlier during pure encoding, so it cannot on its own diagnose whether
the executable write has sufficient permission. Write-only indexing and
iterator variants for mutable references, holders, and optional references
all reach the missing-permission error. Shared-reference iteration required a
separate fix to select the correct call wand, described below.

The single-yield and two-yield shared-reference cases now verify in the main
suite, including reads of the restored slice. Their incorrect-value and
`false` counterparts are rejected. The two-yield regression is
`mut_native_shared_ref_two_yields_pass`; its runtime examples use both repeated
and distinct referents, and its verified function does not require distinct
referents. The former `im3_shared_ref_two_yields` repro was promoted there.

The encoder now associates each emitted restoration obligation with its PCG
edge through a checked signature-to-call projection map. The constructor's
resource-free nested shared lifetime no longer reapplies the outer lifetime's
wand. The same selection retains individual wands across loop boundaries;
phase-local deduplication includes the obligation ID and path condition.
Unknown substitutions and partial edge coverage are rejected. This fixes the
diagnosed duplicate application without claiming support for arbitrary nested
mutable references or reference-containing collection resources.

The former `im3_branch_call_loop` and `im3_retained_branch` repros now verify
and have been promoted to `call_wand_branch_loop_*` and
`call_wand_retained_branch_*` in the borrow suite. They cover symbolic loop
lengths, both branch choices, alternating choices, and incorrect-value/false
controls. Retained call wands keep their edge guards, and unchanged reference
slots keep their shallow snapshots. At each invariant boundary, internal body
choices are reset only if the executable CFG places them after that boundary
and no live boundary condition needs them. Entry and exit history remains
available for restoration. Additional regressions cover `continue`, early
`break`, nested loops, and reference slots assigned or mutably borrowed during
or before a loop. These changes address call lifecycle and reference identity;
they do not provide the missing inner referent resources described above.

The scalar loop's generated invariant retains the iterator tail permission,
the source reference's shallow snapshot, and a wand from the tail resource to
the original slice resource. The constructor's inner expiry wand includes a
content equality using `old[lhs]`; packaging the outer loop wand keeps only
the resource predicate. The unrolled case applies the individual expiry wands
without that loop abstraction. This identifies where the value relationship
is lost, but does not imply that a length-only invariant should prove the
desired final values.

After building Prusti, reproduce a case and save its Viper program with:

```sh
case_name=im0_native_loop_scalar
im0_target=$(mktemp -d)
RUSTUP_TOOLCHAIN=nightly-2026-08-15 CARGO_TARGET_DIR="$im0_target" \
  PRUSTI_ENABLE_CACHE=false PRUSTI_NO_VERIFY_DEPS=true \
  PRUSTI_CHECK_OVERFLOWS=true PRUSTI_CHECK_PANICS=true \
  PRUSTI_DUMP_VIPER_PROGRAM=true PRUSTI_LOG_DIR="$PWD/target/im0-$case_name" \
  PRUSTI_EXTRA_VERIFIER_ARGS='--timeout=180 --numberOfParallelVerifiers 1' \
  ./x.py exec target/debug/cargo-prusti \
  --manifest-path prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml \
  --bin "$case_name"
```

Choose another `im0_*` binary for the corresponding probe. Keep each target
fresh, set `JAVA_HOME` to the recorded Java installation, and inspect the
diagnostic and dumped `viper_program/program-check.vpr` rather than treating
the exit code alone as proof of a particular failure.

## IM1: prefix restoration with a wand

[prefix_wand.vpr](prefix_wand.vpr) models a borrowed tail as a sequence of
distinct *logical* element locations, each with an owned value field. Its
abstract `values` function is a heap-dependent snapshot observation: its
only assumed properties are the sequence length and equality of each entry
to the corresponding owned field. It grants no permissions and assumes no
law relating values across updates. Logical location distinctness is not a
claim that Rust pointer addresses differ for zero-sized elements.

The Viper body of `take_first` is verified: it splits the first field from
the tail and packages an expiry wand. That wand reassembles the original
owner and relates its restored snapshot to the yielded value and tail
snapshot at `old[lhs]`. The loop carries a second wand relating the source to
the completed prefix and the remaining tail. On each backedge, its package
script imports the yielded field, applies the single-step wand, then applies
the previous summary. This makes the prefix relation inductive while the
source remains inaccessible during iteration.

`fill_prefix` proves that the first `min(stop, len)` elements become `7` and
the remaining suffix retains its entry values. `increment_prefix` proves the
same framing and adds one to each processed value, with an explicit
`u32`-compatible overflow bound on entry. Both use arbitrary symbolic finite
lengths. Concrete one- and two-element witnesses exercise empty/full and
mixed prefix/suffix outcomes without assuming the result.

On 2026-10-28, the pinned Silicon/Z3/Java configuration verified the model;
the negative clients rejected a wrong prefix, wrong suffix, wrong increment,
an overflow-causing call, and `false` at reachable states. Reproduce those six
checks with:

```sh
./x.py exec python3 scripts/test_stdlib_prefix_wand.py
```

The runner checks each negative diagnostic, rejects timeouts and internal
errors, and saves logs under `target/stdlib-im1-prefix-logs`. The abstract
snapshot observation and the logical-location model are explicit assumptions:
this is not yet a proof that Prusti can express the invariant in Rust or that
its whole-slice encoding implements the same summary. The existing untrusted
[`mut_region_full_pass`](../src/bin/mut_region_full_pass.rs) and
[`mut_region_full_wrong`](../src/bin/mut_region_full_wrong.rs) cases separately
exercise composition of Rust expiry pledges for two yields. On the current
encoder, the positive case verified in an isolated 129.10-second run and the
wrong-value case produced its expected diagnostic in 128.62 seconds. An
earlier concurrent positive run timed out and is not included as evidence.

## IM2: expressing a restoration summary in a loop

`im2_before_expiry_invariant` tries to use the existing `before_expiry`
operator directly in an ordinary `body_invariant!`. On the IM0 baseline,
it emitted `old[lhs]` outside a wand and produced a Viper consistency error.
The encoder now rejects this use with a specific unsupported-feature
diagnostic: `before_expiry` is available in expiry pledges, including their
specification closures, rather than an ordinary loop invariant. This keeps
the future heap from being read without a restoration obligation; it does
not yet provide an annotation for the IM1 summary.

`im2_owner_snapshot_loop` saves only the source reference's ghost identity,
uses a reflexive identity invariant, and otherwise performs the previously
verified ownership-only mutation loop. It produces an internal
`labelled.state.not.reached` error on the current baseline. Its pre-loop
package refers to `old[bb_8]` before that loop-head label has been reached.
The separate `im2_source_read_invariant` tries to read the saved owner's
contents while its tail is borrowed; it currently encounters the same
earlier label error. Once that is fixed, this read must still require
permission; it is not a valid way to assert the summary unconditionally.

As a diagnostic experiment, replacing the first unreached label in the dumped
identity-only program with its package-entry label exposed a further missing
`p_Param` permission for the original slice. The generated loop has separate
restoration obligations for the source's lifetime projection and its
dereferenced place, both dependent on the iterator tail. Their ownership and
coupling must be accounted for together; the label substitution alone is not
a fix and was not applied to the encoder. The shared-reference iterator
probe in IM0 also exhibits repeated use of a constructor wand, which needs
separate lifetime/coupling analysis before the reference-element gate.

A separate identity probe saves `(&*values) as *const [u32]` in a ghost
snapshot; it carries no indirect mutable-reference predicates. The loop
encoder now retains snapshots of unchanged ghost locals across loop
boundaries, including scalars, sequences, and raw pointers. The regular
`mut_native_owner_identity_pass` test checks that this saved snapshot matches
the restored owner's pointer after the loop;
`mut_native_owner_identity_wrong` rejects a false self-inequality. The
`ghost_snapshot_loop` and `ghost_snapshot_reassigned` tests check scalar and
sequence snapshots, including ghost locals assigned inside a loop; the latter
are not frozen to their entry values. The saved pointer is only an identity:
it does not permit reading the borrowed contents or imply physical address
distinctness. The
regular `mut_native_borrowed_owner_read` regression confirms that reading the
saved mutable owner's contents while the iterator is live is rejected for
insufficient permission. A direct invariant borrowing `values` while its
iterator is live is still rejected by Rust.

The regular `mut_native_tail_suffix` and `mut_native_tail_early_break` tests
prove that the live iterator tail is the entry sequence's suffix at the
processed count, for symbolic lengths and an arbitrary early-break bound.
Their negative clients reject incorrect exit-count claims. The
`mut_native_tail_value` pair proves an untouched element's value on a
nonempty early-break path and rejects a wrong value. Separate restoration
clients now establish the processed prefix and the restored owner's contents.

The `mut_native_prefix_ghost` controls maintain a growing ghost sequence and
verify that its length matches the number of yielded elements. The
`mut_native_prefix_values` controls additionally prove every completed value
is `7`, using both `usize` and mathematical `Int` indices; wrong values and
`false` are rejected. Earlier `im2_prefix_ghost_quantified` probes exposed a
failure to preserve these quantified invariants across append. The sequence
lookup model now gives its existing guarded equality an additional trigger on
native indexing, allowing the solver to instantiate the old-prefix property.
The `body_restore_prefix!` restoration clients connect those ghost values to
the owner after iterator expiry.

The generated constructor package can already prove its initial value
relation using the native `p_Slice` and `p_Param` predicates. The
[constructor-wand runner](../../../../../scripts/test_stdlib_iterator_constructor_wand.py)
inserts that checked assertion into the emitted proof script and runs both
an equality and its deliberately false opposite. It adds no assumptions, and
checks only initialization; the Rust loop annotation now carries the relation
on the selected loop wand. After building Prusti, reproduce the original
constructor experiment with a fresh target directory:

```sh
CONSTRUCTOR_PROBE_TARGET="$(mktemp -d)"
PRUSTI_DUMP_VIPER_PROGRAM=true PRUSTI_LOG_DIR="$PWD/target/stdlib-im2-constructor-input" \
  PRUSTI_ENABLE_CACHE=false PRUSTI_NO_VERIFY_DEPS=true CARGO_TARGET_DIR="$CONSTRUCTOR_PROBE_TARGET" \
  ./x.py exec target/debug/cargo-prusti --manifest-path prusti-tests/tests/cargo_verify/stdlib_iterator_spec/Cargo.toml --bin mut_native_tail_suffix_pass
./x.py exec python3 scripts/test_stdlib_iterator_constructor_wand.py \
  target/stdlib-im2-constructor-input/viper_program/program-check.vpr
```

The runner checks the expected generated fixture shape and stops if it has
changed, rather than treating a different package as evidence.

The [loop-wand runner](../../../../../scripts/test_stdlib_iterator_loop_wand.py)
uses the same dumped program. It strengthens the existing generated loop
wand with a processed-prefix relation and the tail-at-expiry suffix, then
checks the final owner contents. The constructor and `next` pledges establish
the packages; no new assumptions are added. The runner also checks that `false`
and an incorrect final value fail. This generated-Viper experiment predates
the Rust restoration annotation. Its `--prefix-sequence` variant
adds ghost assignments for a sequence of completed snapshots and proves the
planned `source = prefix ++ tail_at_expiry` relation. Run it after the
preceding dump:

```sh
./x.py exec python3 scripts/test_stdlib_iterator_loop_wand.py \
  target/stdlib-im2-constructor-input/viper_program/program-check.vpr
# Check the same proof with an explicit ghost prefix sequence.
./x.py exec python3 scripts/test_stdlib_iterator_loop_wand.py \
  target/stdlib-im2-constructor-input/viper_program/program-check.vpr --prefix-sequence
```

`im2_quantified_expiry_valid` and `im2_quantified_expiry_wrong` check a
two-state pledge inside a quantifier. The valid identity pledge currently
fails with insufficient permission to read a mutable reference, both before
and after the context check. The wrong pledge reports that its pledge
postcondition might not hold. These do not form a completed positive/negative
gate: the valid case must first verify. The generated valid program also
does not wrap the captured mutable-reference heap read in `old[lhs]`, so
propagating the expiry context and checking its heap selection are separate
work from permitting the marker in an expiry closure. When a closure captures
the mutable reference itself rather than its referent, the encoder now selects
the expiry heap for reads inside the quantified body. The regular
`quantified_expiry_heap` and `quantified_expiry_no_permission` regressions
check valid use, a wrong result, and missing permission. A quantifier placed
inside `before_expiry` remains unsupported because its body would otherwise
lose that temporal context; `quantified_expiry_outer_context` checks the
explicit diagnostic.

The `mutable_ref_snapshot` probe attempts to model the remaining mutable
references and relate each reference to the corresponding source element:

```sh
./x.py exec target/debug/cargo-prusti --manifest-path prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/Cargo.toml --bin mutable_ref_snapshot
```

Ordinary Rust compilation succeeds. Verification reports an unsupported
mutable-reference dereference in pure code. This restriction is intentional:
a mutable reference's pure snapshot does not constrain its referent value, so
reading that value without the appropriate heap state would be unsound. The
probe illustrates why reference values cannot be recovered directly from a
pure mutable-reference snapshot. The test-only iterator fixture now uses
address-only observations to relate yielded references to the source without
performing that dereference.

This is a snapshot and borrowing limitation, affecting the optional `IterMut`
owner-restoration milestone. A possible next step is an identity-only model
for yielded references combined with a heap-aware expiry rule that transfers
each final value back to its original slice position. The identity-only part
is now prototyped; the expiry rule remains open for the raw-backed iterator.
Such a model must also
account for multiple simultaneously live yielded references and exhaustion;
removing the pure dereference restriction alone would not establish those
properties and would be unsound.

The `expiry_ref_heap` probe attempts a single-element version of that expiry
rule using `before_expiry` to read a saved reference. On the pinned toolchain,
verification reports an insufficient-permission error at the address
observation in the postcondition. The iterator's lifetime
projection has no encoded predicate for its raw-pointer-backed elements, so
the saved reference's address alone does not provide permission to read it.
Reproduce with the command above, replacing `mutable_ref_snapshot` with
`expiry_ref_heap`. A complete solution needs a lifetime resource for the
iterator's remaining elements and a way to transfer and reassemble the
individual element permissions as yields expire.

The `region_next` probe replaces the raw-pointer-backed iterator with a
struct containing an explicit `&mut [u32]`. Its `next` has no functional
guarantees and its body always returns `None`; the caller must still handle
an unconstrained `Some`. This probe now verifies with two possible results
live at once. The call contract returns the post-call tail resource while a
wand retains the relationship needed to restore the original resource when
the results and tail expire. Additional tests cover both expiry orders and
optional tail state. Run the probe with `--bin region_next`.

This establishes the lifetime plumbing for an explicitly borrowed tail, not a
functional iterator: the probe never actually splits or advances the slice.
The raw-pointer-backed standard iterator still needs its own remaining-element
resource and a way to relate yielded elements to the original slice.

The `take_mut_slice` probe isolates borrow transfer when a safe implementation
moves a borrowed tail out of an `Option` using `mem::replace`. It now verifies:
the encoder avoids repeating graph edges already restored before the wand is
packaged. The regression suite also checks a caller that drops the slot before
using the returned borrow and rejects a false claim in the returned-`Some` path.
Run the probe with `--bin take_mut_slice`.

The `region_advance` probe implements `next` by taking and splitting that
tail. It now verifies, including a caller that holds two possible results and
restores the original slice's length. The package builder follows the input's
original lifetime projection through the split, and captures an index at the
same saved heap state as its slice. Run it with `--bin region_advance`.
The `mut_region_functional_pass` regression goes further: the untrusted
implementation proves its remaining length and `Some`/`None` behavior, and
the caller proves two yields and exhaustion. It uses `mut_slice_len`, a
metadata-only observation that does not read the mutable slice's elements.
The `mut_region_value_pass` regression uses `mut_slice_contents`, which
requires slice permission, in an expiry pledge and proves that writing through
one yielded reference updates the original slice. Its incorrect-value case is
rejected. The `mut_region_reassembly_pass` case and its reverse-order
counterpart use `after_call` to retain each call's post-call tail snapshot and
prove the final values of two simultaneously live yields. The untrusted `next` contract
applies to an arbitrary nonempty tail; incorrect values and a false assertion
are rejected. Generic two-yield tests also restore non-`Copy` elements in both
use orders and reject an incorrect value. `mut_region_full_pass` uses a single
generic `next` contract with separately stated yielding and exhaustion
pledges, and proves two mutations followed by an exhausted call. Its generic
counterpart checks a non-`Copy` element type.

`region_full_reassembly` combines that expiry pledge with the `Some`/`None`
contract in a single `next`. The implementation verifies on its own, but the
two-call client timed out with the 90-second verifier limit on 2026-10-28.
Run it with `--bin region_full_reassembly`. The separately stated pledges in
`mut_region_full_pass` establish the corresponding combined behavior for a
slice-backed iterator. Restoring ownership after an arbitrary-length mutation
loop now verifies in `mut_region_loop_pass`; proving the final values of its
entire updated prefix and modeling the standard library's raw-pointer-backed
`IterMut` remain open.

`region_full_loop` attempts an arbitrary-length mutation loop using this
contract and a length invariant. Its original unpositioned permission failure
exposed several encoder omissions: singleton loop wands were discarded at the
invariant, the original reference's shallow value was lost, and lifetime
projection labels were ignored. The encoder now preserves those wands and
reference values, captures the latter on loop-entry edges, and selects the
saved heap for each lifetime projection. When composing an expiry wand inside
a package, it checks the wand's inputs before applying it so `old[lhs]` can
read resources imported from the enclosing heap.

The generic `mut_region_loop_pass` regression writes through each yield,
destroys the iterator, then writes and reads the original slice again. The
corresponding false-assertion regression is rejected. These checks establish
resource restoration across an unbounded loop. The original `region_full_loop`
probe still lacks an invariant describing the updated prefix, so its
whole-slice value postcondition is not an acceptance test for this fix. Run
that probe with `--bin region_full_loop`.

Before the edge-replay fix, the wand package script in `take_mut_slice`
converted the slot's representation a second time and consumed its permission;
the enclosing method could not return that permission. The standalone
`slot_unframed.vpr` isolates this effect and fails with an insufficient-permission postcondition, while
`slot_frame.vpr` verifies when the slot is included on both sides of the wand.
The latter also makes the slot a requirement for applying the wand, so a
Prusti fix must account for that dependency and prove the relevant call and
expiry paths; the standalone example is not itself an encoder fix. Run these
files with the Silicon command below, replacing its final filename. Adding the
same frame to both sides of the earlier generated `take_mut_slice` Viper
program also made that method verify. The implemented fix instead avoids the
repeated conversion, so it does not require callers to retain the slot.

The standalone `spatial_iter.vpr` experiment models elements as distinct
*logical* locations with quantified permissions. It verifies splitting off
two simultaneously live elements, restoring their owner, and an unbounded
loop that updates every element. `spatial_iter_wrong.vpr` changes the loop's
postcondition to an incorrect value and is rejected. Run them with the pinned
Viper distribution, for example:

```sh
Z3_EXE="$PWD/viper_tools/z3/bin/z3" "$JAVA_HOME/bin/java" -Xss16m \
  -cp viper_tools/backends/viperserver.jar viper.silicon.SiliconRunner \
  --numberOfParallelVerifiers 1 --timeout 90 \
  prusti-tests/tests/cargo_verify/stdlib_iterator_spec/known_limits/spatial_iter.vpr
```

Replace the last filename with `spatial_iter_wrong.vpr` for the negative
check. Both ran on 2026-10-28 with Viper `v-2026-09-01-0834`, Z3 `4.16.0`,
and Java `21.0.2`. The distinctness premise is required: the predicate alone
does not assert ownership when locations are not distinct. These locations
are ghost identities, not Rust pointer addresses, which may coincide for
zero-sized elements. A separate positive case permits distinct logical
locations to share a physical address, and a negative assertion checks that
this case is not contradictory. The experiment does not yet connect those identities
to Rust slices, encode empty `next`, or establish a bundled `IterMut` model.

The integration boundary is the slice's whole-owner predicate and its
`index` framing operations in `prusti-encoder/src/encoders/ty/kinds/arraylike.rs`.
An iterator resource must be created from that ownership at `iter_mut`, split
when `next` yields, and reassembled when the iterator and its yielded borrows
expire. The current lifetime projection ignores raw pointers, so the pointer
fields alone do not perform any of those transfers. Logical element identity
must also stay separate from physical address observations: raw-pointer
construction, raw-pointer equality, and the mutable-reference address helpers
currently use Viper `Ref` identities directly. A change to only one of those
paths could make zero-sized element comparisons inconsistent.
