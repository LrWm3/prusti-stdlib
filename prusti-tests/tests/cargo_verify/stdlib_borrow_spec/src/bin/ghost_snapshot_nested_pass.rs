use prusti_contracts::*;

fn visit() {
    let mut round = 0usize;
    loop {
        body_invariant!(round <= 2);
        if round == 2 { break; }
        let saved = ghost! { round };
        let mut inner = 0usize;
        loop {
            body_invariant!(inner <= 1);
            body_invariant!(*saved == *saved);
            if inner == 1 { break; }
            inner += 1;
        }
        prusti_assert!(*saved == round);
        round += 1;
    }
}

fn main() {}
