use prusti_contracts::{iter_model::ModeledIterator, *};

#[requires(expected == rows.len())]
#[ensures(result == expected)]
fn nested(rows: Vec<Vec<u8>>, expected: usize) -> usize {
    let mut outer_count = 0usize;
    for row in rows {
        body_invariant!(expected == old(expected));
        body_invariant!(Int::from(outer_count)
            + for_iterator::<std::vec::IntoIter<Vec<u8>>>().remaining().len()
            + Int::from(1) == Int::from(expected));
        let length = row.len();
        let mut inner_count = 0usize;
        for _value in row {
            body_invariant!(Int::from(inner_count)
                + for_iterator::<std::vec::IntoIter<u8>>().remaining().len()
                + Int::from(1) == Int::from(length));
            inner_count += 1;
        }
        prusti_assert!(inner_count == length);
        outer_count += 1;
    }
    outer_count
}

fn main() {}
