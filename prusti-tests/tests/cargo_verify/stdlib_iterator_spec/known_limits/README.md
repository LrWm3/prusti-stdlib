# Mutable iterator reference snapshots

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
slice-backed iterator. Proving an arbitrary-length mutation loop and modeling
the standard library's raw-pointer-backed `IterMut` remain open.

`region_full_loop` attempts an arbitrary-length mutation loop using this
contract and a length invariant. On 2026-10-28 it failed with an
insufficient-permission obligation lacking a Viper source position, which
Prusti reports as an internal error. This probe does not yet provide an
invariant describing the updated prefix, so correcting the diagnostic alone
would not establish the loop postcondition. Replacing that postcondition with
`true` still produces the same unpositioned permission error. Run the original
probe with `--bin region_full_loop`.

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
