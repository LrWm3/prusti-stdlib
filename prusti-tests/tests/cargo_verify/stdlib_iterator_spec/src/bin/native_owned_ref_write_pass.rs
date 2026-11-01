use prusti_contracts::*;

fn write(value: &mut u32) {
    let mut values = Vec::new();
    values.push(&mut *value);
    let mut iterator = values.into_iter();
    if let Some(element) = iterator.next() {
        *element = 7;
    }
    drop(iterator);
    prusti_assert!(*value == 7);
}

fn main() {
    let mut value = 1;
    write(&mut value);
}
