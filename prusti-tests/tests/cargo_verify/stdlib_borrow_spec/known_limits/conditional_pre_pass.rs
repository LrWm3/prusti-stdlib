use stdlib_borrow_known_limits::conditional_pre;

fn main() {
    let marker = 1usize;
    let result = conditional_pre(&marker, 1);
    assert!(result == 7);
}
