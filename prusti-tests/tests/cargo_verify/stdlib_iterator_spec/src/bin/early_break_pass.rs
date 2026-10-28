use iterator_specs::prefix_owned;

struct Token(u32);
fn main() {
    let empty: Vec<Token> = Vec::new();
    assert_eq!(prefix_owned(empty, 0, 1), 0);
    let mut values = Vec::new();
    values.push(Token(1));
    values.push(Token(2));
    assert_eq!(prefix_owned(values, 2, 1), 1);
}
