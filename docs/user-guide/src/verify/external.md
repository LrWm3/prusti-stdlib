# External specifications

Since the Rust standard library and external libraries do not specify contracts for their functions, Prusti allows specifying the contract for functions separately from where they are implemented. Such a specification looks like a regular `impl`, with the exception that there are no bodies in the implementation functions, and that the `impl` has an `#[extern_spec]` attribute.

The standard library type `std::option::Option` could be specified as follows:

```rust,noplaypen,ignore
use prusti_contracts::*;

#[extern_spec]
impl<T> std::option::Option<T> {
    #[trusted]
    #[pure]
    #[ensures(matches!(*self, Some(_)) == result)]
    pub fn is_some(&self) -> bool;

    #[trusted]
    #[pure]
    #[ensures(self.is_some() == !result)]
    pub fn is_none(&self) -> bool;

    // ...
}
```

Prusti does not verify the functions of an external specification against their implementation, so every function in an external specification must be marked [`#[trusted]`](trusted.md). It is possible to specify multiple `#[extern_spec]` implementations for the same type, but it is an error to externally specify the same function multiple times.

Prusti already bundles contracts for `Option::unwrap` and `core::mem::replace`
(also available through `std::mem::replace`). Do not redeclare these contracts.
`unwrap` requires `Some` and returns its payload; `replace` returns the old
destination and installs the moved source. Both contracts use snapshot equality,
so owned elements need no `Copy`, `Clone`, or equality traits.

An external declaration must preserve the target's parameter and return types
and forward its type and const parameters unchanged. It must not add
unconditional bounds that the target does not guarantee: for example, a
declaration of `mem::replace` cannot add `T: Copy`. Bounds already implied by the
target, such as `Clone` when the target requires `Copy`, are accepted. Prusti
reports an invalid specification if it cannot establish a required bound; this
check can conservatively reject complex outlives implications.

Lifetime relationships must also match. Renaming late-bound lifetimes, including
those in higher-ranked function types, is supported. Merging independent borrows,
requiring `'static`, or shortening the result lifetime is rejected even when Rust
can coerce the generated call. Early-bound lifetime parameters must occur in the
same generic positions as the target; use explicit lifetimes in reference impl
headers. This is a conservative structural check, not a solver for all equivalent
outlives constraints. Concrete trait specifications are compared against the
trait's public call signature with the implementation's arguments substituted.

The `extern_spec` attribute accepts an optional argument to provide the module path to the function being specified. For example, to specify `std::mem::swap`, the argument is `std::mem`:

```rust,noplaypen,ignore
use prusti_contracts::*;

#[extern_spec(std::mem)]
#[trusted]
#[ensures(*a === *old(&*b) && *b === *old(&*a))]
fn swap<T>(a: &mut T, b: &mut T);
```
