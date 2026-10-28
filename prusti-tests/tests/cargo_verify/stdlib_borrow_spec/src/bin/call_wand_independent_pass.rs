use prusti_contracts::*;

#[ensures(*result.0 == old(*first) && *result.1 == old(*second))]
fn pair<'a, 'b>(first: &'a mut u32, second: &'b mut u32) -> (&'a mut u32, &'b mut u32) {
    (first, second)
}

fn visit(first: &mut u32, second: &mut u32, reverse: bool) {
    let (a, b) = pair(first, second);
    if reverse {
        *b = 11;
        *second = 13;
        *a = 17;
        *first = 19;
        prusti_assert!(*first == 19 && *second == 13);
    } else {
        *a = 11;
        *first = 13;
        *b = 17;
        *second = 19;
        prusti_assert!(*first == 13 && *second == 19);
    }
}

fn main() {
    visit(&mut 1, &mut 2, true);
    visit(&mut 1, &mut 2, false);
}
