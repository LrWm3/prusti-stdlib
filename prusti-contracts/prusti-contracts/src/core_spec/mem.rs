use crate::*;

// The source is moved into the destination and the previous value is returned;
// neither value is dropped. Snapshot equality imposes no traits on T.
#[extern_spec(core::mem)]
#[trusted]
#[ensures(*dest === src)]
#[ensures(result === *old(&*dest))]
fn replace<T>(dest: &mut T, src: T) -> T;
