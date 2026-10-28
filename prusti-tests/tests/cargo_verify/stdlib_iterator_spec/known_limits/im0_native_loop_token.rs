use prusti_contracts::*;

struct Token { value: u32 }

#[ensures(forall(|i: usize| i < values.len() ==> values[i].value == 7))]
fn visit(values: &mut [Token]) {
    let len = values.len();
    let mut it = values.iter_mut();
    loop {
        body_invariant!(mut_slice_len(&*mut_iter_tail(&it)) <= len);
        match it.next() {
            Some(value) => value.value = 7,
            None => break,
        }
    }
    drop(it);
}

#[trusted]
fn main() { visit(&mut [Token { value: 1 }, Token { value: 2 }]); }
