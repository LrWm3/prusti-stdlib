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
is now prototyped; the expiry rule remains open. Such a model must also
account for multiple simultaneously live yielded references and exhaustion;
removing the pure dereference restriction alone would not establish those
properties and would be unsound.

The `expiry_ref_heap` probe attempts a single-element version of that expiry
rule using `before_expiry` to read a saved reference. On the pinned toolchain,
verification reaches a Viper insufficient-permission error which is currently
reported as an internal backtranslation error. The iterator's lifetime
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
These examples do not yet prove the final element values or model the standard
library's raw-pointer-backed `IterMut`.

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
