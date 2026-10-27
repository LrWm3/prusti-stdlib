# Minimal functional Vec gate (P1-D / M1)

Run from the repository root on the pinned toolchain, after building Prusti:

```sh
./x.py build --all
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite vec
```

The runner uses a fresh Cargo target directory, disables verification caching,
and enables overflow and panic checking. It verifies the `no_std` `vec_specs`
library explicitly before verifying its separately compiled clients. It also
compiles every client with ordinary Rust and executes every positive binary.
CI runs the same command. Logs, timings, and positive Viper programs remain in
`target/stdlib-vec-spec-logs/`.

Bundled external contracts cover real `Vec::new`, `len`, `is_empty`, `push`,
`pop`, `as_slice`, and `Index<I>::index`. They preserve the pinned API's generic
allocator and index parameters. A supporting `usize: SliceIndex<[T]>` contract
for `get` supplies scalar bounds and element observations; range indices are a
later gate. `vec_contents` and the exported `view` observe the same slice-backed
sequence of element snapshots, without consuming elements or requiring `Copy`,
`Clone`, `PartialEq`, or `Eq`.

- `symbolic_pass` proves append preservation at an arbitrary existing index,
  the moved element at the new last index, and the new length. It also proves a
  generic push/pop round trip through exported wrappers and a nonempty pop's
  returned element, length, and retained prefix. Ordinary runs instantiate these
  generic clients with an owned struct lacking copy/clone/equality traits.
- `loop_pass` constructs a Vec of arbitrary symbolic finite length. Its loop
  invariant describes every element in the entire constructed prefix. There is
  no fixed bound or unrolling. Clients check empty and eight-element results,
  indexed access, pop, and restoration after a cross-crate round trip.
- `basic_pass` checks empty/new behavior, shared Vec/slice indexing, and both
  empty and nonempty pop branches.
- `trait_dispatch_pass` isolates selection of a pure implementation of an
  otherwise impure trait. `trait_purity_leak` checks that the implementation's
  purity does not escape to an abstract trait-bound caller.
- `old_body_pass` isolates entry-state observations inside body assertions and
  loop invariants. `old_body_changed` rejects identifying a changed parameter
  with its entry value. These retain the encoder blocker exposed by the loop.
- The other negative cases reject wrong elements, prefixes, lengths and pop
  results, invalid indexing, length overflow, and duplicate bundled contracts.
  Each must produce exactly the diagnostic at its annotated source line.

The supported element domain is owned values with stable value snapshots.
Reference-containing elements, interior mutability, nested collection models,
mutable views, and iterators are outside this gate.

These functional contracts describe normal returns from trusted library code.
`push` requires a length below `usize::MAX`; its sequence arithmetic uses
mathematical integers. That precondition does not guarantee a successful buffer
allocation or exclude Rust's capacity-overflow failure. Allocator failure,
capacity growth, unwinding, and arbitrary destructors remain execution
assumptions outside M1. Enabling panic checks verifies the client obligations
expressed by the contracts, including indexing and unwrap bounds; it does not
verify the library's allocator or unsafe implementation.
