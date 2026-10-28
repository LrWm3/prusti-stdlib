use prusti_contracts::*;

fn visit(first: &mut [u32], second: &mut [u32]) {
    let owner = ghost! { (&*first) as *const [u32] };
    let prefix = ghost! { Seq::<u32>::new() };
    let saved = first.iter_mut();
    let mut active = second.iter_mut();
    loop {
        body_restore_prefix!(&saved, *owner, *prefix); // expected-unsupported: cannot encode method body `visit`: body_restore_prefix requires a unique matching iterator restoration edge
        match active.next() { Some(value) => *value = 7, None => break }
    }
    drop(saved);
}

fn main() {}
