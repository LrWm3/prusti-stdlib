use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let owner = ghost! { (&*values) as *const [u32] };
    let prefix = ghost! { Seq::<u32>::new() };
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix); // expected-unsupported: cannot encode method body `visit`: body_restore_prefix is not yet supported
        body_invariant!(true);
        match it.next() { Some(value) => *value = 7, None => break }
    }
}

fn main() {}
