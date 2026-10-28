use prusti_contracts::*;

fn visit(values: &mut [u32]) {
    let owner = ghost! { (&*values) as *const [u32] };
    let mut completed = ghost! { Seq::<u32>::new() };
    prusti_for!(iterator, for value in &mut *values => {
        body_restore_prefix!(&iterator, *owner, *completed); // expected-unsupported: cannot encode method body `visit`: body_restore_prefix requires the annotation before iterator advancement
        *value = 7;
        completed = ghost! { (*completed).append(Seq::single(7u32)) };
    });
    prusti_assert!(forall(|i: usize| i < values.len() ==> values[i] == 7));
}

fn main() {}
