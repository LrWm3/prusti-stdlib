# Key model probes (P3-A)

Run after building Prusti:

```sh
./x.py exec python3 scripts/test_stdlib_extern_spec.py --suite hash
```

The separately verified `no_std` library defines a generic projection with an
associated model type. Its `equivalent` method is specified to agree with
snapshot equality of projected keys. A generic logical-map insertion helper
exports its membership and lookup guarantees across crates. Implementations
cover `u32` and a custom key whose verified `PartialEq` compares only `id`.
Clients prove reflexivity, symmetry, and transitivity for the custom equality,
its agreement with the projection, the corresponding integer relationship,
and equal-key compatibility with the custom hash observation. They also check
that equal custom keys may have different whole snapshots, and that different
projected keys remain separate in a logical map even when their hash
observations collide.

The custom key's `Hash` method is explicitly trusted: its reviewed body writes
the same byte for every key, and a runtime-only check exercises a collision with
`DefaultHasher`. Its constant `hash_model` is a proof fixture, not a generic
contract for arbitrary hashers. Negative clients reject whole-snapshot equality
for equal keys, equality of different colliding projections, arbitrary claims
about an unconstrained projection, and claims based on bare `Eq`.
The guarded pure-method probes verify a concrete implementation and reject
assuming its result for every implementation satisfying the trait's precondition.

Crucially, `Projection` alone only constrains its own `equivalent` method. It
does not prove that every implementation agrees with Rust `Eq` or satisfies
hash compatibility and storage stability. The negative bridge test checks that
the verifier does not infer that relationship.

The bundled `KeyProjection` and `HashKeyModel` separate a logical projection
from an explicit trusted opt-in to the key laws. The integer implementation
and this fixture's custom key are reviewed implementations; the custom key's
lawful constant hash remains a trusted assumption. The fixture also verifies
the concrete equality/projection relationship. The bundled proof-only map and
set observations describe projected keys, not stored key representatives.
Conditional constructor contracts establish empty observations when key-law
evidence is present. Clients with just a projection cannot infer those
guarantees, and incorrect nonempty claims are rejected.

Conditional `len` and `insert` contracts also model map and set cardinality,
new entries, replacements, returned old values, and unaffected map entries.
The cross-crate clients cover generic keys, integer keys, and equal or
colliding custom keys. Negative clients reject incorrect old values, framing,
duplicate cardinality, collision claims, and length facts without evidence.
The runtime-only insertion check exercises the custom key without injecting
proof facts. Conditional map `contains_key`/`get` and set `contains` contracts
retain their original borrowed-key signatures but currently require the sealed
same-type capability. Negative clients check missing evidence, wrong lookup
values and membership, and an unmodeled heterogeneous query. Removal,
clearing, and mutable values remain open.
The heterogeneous negative fixture deliberately provides `Borrow<u32>` for
the custom key without claiming that the two hash implementations agree; it
is not evidence that this borrowed form satisfies Rust's `Borrow` contract.
