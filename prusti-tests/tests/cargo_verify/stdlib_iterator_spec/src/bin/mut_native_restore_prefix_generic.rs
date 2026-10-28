use prusti_contracts::*;

fn visit<T>(values: &mut [T]) {
    let owner = ghost! { (&*values) as *const [T] };
    let prefix = ghost! { Seq::<T>::new() };
    let mut it = values.iter_mut();
    loop {
        body_restore_prefix!(&it, *owner, *prefix); // expected-unsupported: cannot encode method body `visit`: body_restore_prefix is not yet supported
        match it.next() { Some(_) => {}, None => break }
    }
}

fn main() {}
