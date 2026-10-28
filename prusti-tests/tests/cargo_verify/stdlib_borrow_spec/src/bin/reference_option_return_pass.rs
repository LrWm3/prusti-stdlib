use prusti_contracts::*;

struct Holder<'a> {
    value: &'a u32,
    available: bool,
}

impl<'a> Holder<'a> {
    #[ensures(!self.available)]
    #[ensures(match &result {
        Some(value) => old(self.available) && *value === self.value,
        None => !old(self.available),
    })]
    fn read(&mut self) -> Option<&'a u32> {
        if self.available {
            self.available = false;
            Some(self.value)
        } else {
            None
        }
    }
}

struct Pair<'a, 'b> {
    left: &'a u32,
    right: &'b u32,
}

fn left<'a, 'b>(pair: Pair<'a, 'b>) -> &'a u32 {
    pair.left
}

struct MixedPair<'a, 'b> {
    left: &'a u32,
    right: &'b mut u32,
}

fn left_from_mixed<'a, 'b>(pair: MixedPair<'a, 'b>) -> &'a u32 {
    pair.left
}

fn main() {
    let value = 7;
    let mut holder = Holder {
        value: &value,
        available: true,
    };
    let first = holder.read();
    prusti_assert!(!holder.available);
    assert!(matches!(first, Some(_)));
    assert!(matches!(holder.read(), None));

    let other = 8;
    let _ = left(Pair {
        left: &value,
        right: &other,
    });

    let mut mutable = 9;
    let _ = left_from_mixed(MixedPair {
        left: &value,
        right: &mut mutable,
    });
    mutable = 10;
}
