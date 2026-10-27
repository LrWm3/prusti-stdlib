# Expanded Vec and shared slice gate (P2-A)

Run from the repository root on the pinned toolchain, after building Prusti:

```sh
./x.py build --all
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite collections
```

The runner uses a fresh Cargo target directory, disables verification caching,
and enables overflow and panic checking. It explicitly verifies the `no_std`
`collection_specs` library, compiles every client with ordinary Rust, executes
the positive binaries, and verifies the separately compiled clients. Each
negative case must produce exactly its annotated diagnostic at the expected
source line. CI includes the same command. Logs, timings, and positive Viper
programs remain in `target/stdlib-collections-spec-logs/`.

The bundled contracts extend the shared sequence model to `Vec::clear`,
`truncate`, `append`, `insert`, `remove`, and `swap_remove`. `append` relates both
final vectors to both entry sequences. `remove` shifts the suffix; `swap_remove`
replaces the selected element with the previous last element. Generic allocator
parameters are preserved. No element `Copy`, `Clone`, or equality bounds are
introduced.

Shared slice contracts cover `get`, indexing, `split_at`, and `is_empty`, with
`Vec::deref` connecting implicit shared coercions to the same model. Supported
indices are `usize`, `start..end`, `start..`, `..end`, `..`, `..=end`, and freshly
constructed `start..=end`. Checked lookup describes both `Some` and `None`;
indexing requires `Some`. Reference snapshot equality preserves slice length
metadata as well as element observations. `slice::swap` exchanges the selected
element snapshots and preserves all other elements, including when the two
indices coincide.

`RangeInclusive` has hidden iterator state: identical endpoints do not imply
identical selected slices after iteration. Its subsequence contract therefore
applies when the whole range snapshot equals `RangeInclusive::new(start, end)`.
The literal range syntax satisfies this condition. An adjacent reversed fresh
range such as `4..=3` is valid and empty; `5..=3` is invalid. An inclusive end of
`usize::MAX` is invalid even for a slice whose length is `usize::MAX`. Existing
iterator state remains outside P2-A, and a negative probe rejects inferring a
singleton solely from equal range endpoints.

- `mutations_pass` proves generic append observations at arbitrary indices,
  insertion and removal with quantified prefix/suffix preservation,
  `swap_remove` replacement and framing, and truncation of arbitrary symbolic
  lengths. The contracts are also consumed through an exported append wrapper.
- `slices_pass` proves generic range observations, checked scalar and range
  lookup branches, inclusive indexing, a cross-crate split wrapper, and swap
  framing and restoration after swapping twice.
- `boundaries_pass` exercises empty/end cases, empty sources and destinations,
  insertion/removal round trips through an exported wrapper, stable versus
  unstable removal order, truncation above/equal to/below the length, reversed
  and overflowing ranges, empty splits, and equal/distinct swap indices.
- The library's verified loop builds an arbitrary symbolic finite sequence of
  owned structs without copy/clone/equality traits. Concrete client executions
  include lengths five and eight; they impose no bound on the generic proofs.
- Negative clients reject incorrect lengths, elements, source state, ordering,
  framing, invalid bounds, and append length overflow.

The supported element domain remains owned values with stable value snapshots.
These are normal-return contracts for trusted library implementations; length
preconditions do not guarantee allocation success or exclude capacity-overflow
failure. Allocation, unwinding, and arbitrary destructors remain execution
assumptions. Mutable collection views and owner restoration after borrow expiry
belong to P2-B; this suite's mutable slice swaps use direct slice arguments and
arrays. Reference-containing elements, interior mutability, nested collection
models, and iteration are not established by this gate.
