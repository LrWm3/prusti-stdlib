fn take<'a>(slot: &mut Option<&'a mut [u32]>) -> Option<&'a mut [u32]> {
    core::mem::replace(slot, None)
}

fn main() {}
