use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let owner = ghost! { (&*values) as *const [u32] };
    let prefix = ghost! { Seq::<u32>::new() };
    let mut it = values.iter_mut();
    body_restore_prefix!(&it, *owner, *prefix); // expected-incorrect: `body_restore_prefix!` annotations must be placed inside loop bodies
    let _ = it.next();
}

fn main() {}
