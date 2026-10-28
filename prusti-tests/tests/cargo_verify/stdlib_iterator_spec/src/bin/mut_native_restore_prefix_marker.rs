use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let owner = ghost! { (&*values) as *const [u32] };
    let prefix = ghost! { Seq::<u32>::new() };
    let mut it = values.iter_mut();
    prusti_assert!(loop_restore_prefix(&it, *owner, *prefix)); // expected-unsupported: cannot encode method body `visit`: loop_restore_prefix is only available in body_restore_prefix annotations
    let _ = it.next();
}

fn main() {}
