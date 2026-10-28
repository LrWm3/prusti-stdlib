use prusti_contracts::*;

fn visit(values: &mut [()]) {
    let owner = ghost! { (&*values) as *const [()] };
    let prefix = ghost! { Seq::<()>::new() };
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix); // expected-unsupported: cannot encode method body `visit`: body_restore_prefix is not yet supported
        match it.next() { Some(_) => {}, None => break }
    }
}

fn main() {}
