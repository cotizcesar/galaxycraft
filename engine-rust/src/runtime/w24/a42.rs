#![allow(unused_imports)]
use super::super::*;

// ===========================================================================
// Merged from src/wave24/agent42.rs
// ===========================================================================
/// Shared scalar-deleting-destructor shape: run the destructor, then free when bit 0 of `flags` is set.
pub fn w24a42_scalar_delete(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    dtor();
    if flags & 1 != 0 {
        free();
    }
}

/// `0x007c42a0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_42a0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c42d0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_42d0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4300`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4300(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4330`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4330(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4360`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4360(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4390`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4390(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c43f0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_43f0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c45a0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_45a0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c45d0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_45d0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4600`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4600(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4bf0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4bf0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4c20`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4c20(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4c50`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4c50(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4cb0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4cb0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

/// `0x007c4ce0`: scalar deleting destructor; calls the member destructor, then the deallocator when `flags & 1`.
pub fn w24a42_del_4ce0(flags: u8, dtor: impl FnOnce(), free: impl FnOnce()) {
    w24a42_scalar_delete(flags, dtor, free);
}

#[cfg(test)]
mod tests_wave24_agent42 {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn deleting_destructors() {
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_42a0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_42a0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_42a0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_42d0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_42d0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_42d0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4300(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4300(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4300(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4330(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4330(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4330(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4360(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4360(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4360(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4390(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4390(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4390(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_43f0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_43f0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_43f0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_45a0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_45a0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_45a0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_45d0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_45d0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_45d0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4600(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4600(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4600(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4bf0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4bf0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4bf0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4c20(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4c20(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4c20(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4c50(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4c50(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4c50(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4cb0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4cb0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4cb0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
        {
            let (d, f) = (Cell::new(0), Cell::new(0));
            w24a42_del_4ce0(0, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (1, 0));
            w24a42_del_4ce0(3, || d.set(d.get() + 1), || f.set(f.get() + 1));
            assert_eq!((d.get(), f.get()), (2, 1));
            w24a42_del_4ce0(2, || (), || f.set(f.get() + 1));
            assert_eq!(f.get(), 1);
        }
    }
}
