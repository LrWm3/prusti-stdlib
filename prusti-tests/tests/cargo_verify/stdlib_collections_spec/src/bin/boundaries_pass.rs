use collection_specs::{make, view, Owned};
use prusti_contracts::*;

fn append_boundaries() {
    let mut dest = make(0);
    let mut source = make(8);
    collection_specs::concatenate(&mut dest, &mut source);
    assert!(source.is_empty());
    assert!(dest.len() == 8);
    assert!(dest[6].value == 6);
    dest.append(&mut source);
    assert!(source.is_empty());
    assert!(dest.len() == 8);
    assert!(dest[6].value == 6);
    source.append(&mut Vec::new());
    assert!(source.is_empty());
}

fn insert_remove_boundaries() {
    let mut values = make(0);
    let item = collection_specs::insert_remove(&mut values, 0, Owned { value: 11 });
    assert!(item.value == 11);
    assert!(values.is_empty());
    values.insert(0, Owned { value: 3 });
    values.insert(1, Owned { value: 5 });
    values.insert(0, Owned { value: 2 });
    assert!(values.len() == 3);
    assert!(values[0].value == 2);
    assert!(values[1].value == 3);
    assert!(values[2].value == 5);
    let item = values.remove(2);
    assert!(item.value == 5);
    let item = values.remove(0);
    assert!(item.value == 2);
    assert!(values[0].value == 3);
    let item = values.swap_remove(0);
    assert!(item.value == 3);
    assert!(values.is_empty());
}

fn order_and_truncation() {
    let mut stable = make(8);
    // Instantiate the builder's observations before changing their indices.
    assert!(stable[2].value == 2);
    assert!(stable[4].value == 4);
    let removed = stable.remove(1);
    assert!(removed.value == 1);
    assert!(stable[1].value == 2);
    let before = ghost! { view(&stable) };
    stable.truncate(usize::MAX);
    prusti_assert!(view(&stable) == *before);
    stable.truncate(7);
    prusti_assert!(view(&stable) == *before);
    stable.truncate(4);
    assert!(stable.len() == 4);
    assert!(stable[3].value == 4);
    stable.truncate(0);
    assert!(stable.is_empty());
    stable.clear();
    assert!(stable.is_empty());
}

fn swap_removal_boundaries() {
    let mut swapped = make(8);
    // Expose the old observations used by the two mutations below.
    assert!(swapped[2].value == 2);
    assert!(swapped[6].value == 6);
    assert!(swapped[7].value == 7);
    let exchanged = swapped.swap_remove(1);
    assert!(exchanged.value == 1);
    assert!(swapped[1].value == 7);
    assert!(swapped[2].value == 2);
    let last = swapped.swap_remove(6);
    assert!(last.value == 6);
    assert!(swapped.len() == 6);
    swapped.clear();
    assert!(swapped.is_empty());
}

fn split_boundaries() {
    let values = make(8);
    let (all, empty) = values.split_at(8);
    assert!(all.len() == 8);
    assert!(empty.is_empty());
    let (empty, all) = values.split_at(0);
    assert!(empty.is_empty());
    assert!(all[7].value == 7);
}

fn half_open_boundaries() {
    let values = make(8);
    assert!(values[8..8].is_empty());
    assert!(values[8..].is_empty());
    assert!(values[..0].is_empty());
    assert!(matches!(values.get(8), None));
    assert!(matches!(values.get(7..3), None));
    assert!(matches!(values.get(0..9), None));
    assert!(values.get(8..8).unwrap().is_empty());
    assert!(values.get(8..).unwrap().is_empty());
    assert!(values.get(..0).unwrap().is_empty());
    assert!(values.get(..).unwrap().len() == 8);
}

fn inclusive_boundaries() {
    let values = make(8);
    assert!(values.get(..=7).unwrap().len() == 8);
    assert!(matches!(values.get(..=usize::MAX), None));
    assert!(values.get(3..=3).unwrap()[0].value == 3);
    assert!(values.get(4..=3).unwrap().is_empty());
    assert!(matches!(values.get(5..=3), None));
    assert!(matches!(values.get(0..=usize::MAX), None));
}

fn empty_slice_boundaries() {
    let empty = make(0);
    assert!(empty[..].is_empty());
    assert!(empty.get(0..0).unwrap().is_empty());
    assert!(matches!(empty.get(..=0), None));
    assert!(matches!(empty.get(0..=0), None));
    let (a, b) = empty.split_at(0);
    assert!(a.is_empty());
    assert!(b.is_empty());
}

fn swap_boundaries() {
    let mut values = [
        Owned { value: 2 },
        Owned { value: 3 },
        Owned { value: 5 },
        Owned { value: 7 },
        Owned { value: 11 },
    ];
    values.swap(0, 4);
    assert!(values[0].value == 11);
    assert!(values[4].value == 2);
    assert!(values[2].value == 5);
    values.swap(2, 2);
    assert!(values[2].value == 5);
}

fn main() {
    append_boundaries();
    insert_remove_boundaries();
    order_and_truncation();
    swap_removal_boundaries();
    split_boundaries();
    half_open_boundaries();
    inclusive_boundaries();
    empty_slice_boundaries();
    swap_boundaries();
}
