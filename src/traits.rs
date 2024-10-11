// Implementations of some common traits for the RingBuffer.
use crate::RingBuffer;
use std::{
    cmp::{Eq, PartialEq},
    fmt::Display,
    ops::Index,
};

/// Safety: `NonNull` provides `Send` if `T` is `Send`.
unsafe impl<T: Send> Send for RingBuffer<T> {}

/// Safety: `NonNull` provides `Sync` if `T` is `Sync`.
unsafe impl<T: Sync> Sync for RingBuffer<T> {}

impl<T> Default for RingBuffer<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Allow raw indexing into the buffer.
///
/// Warning: this will panic if the index is out of bounds.
impl<T> Index<usize> for RingBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.get(index).expect("Index out of bounds.")
    }
}

impl<T: Display> Display for RingBuffer<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[")?;
        for (i, item) in self.iter().enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            Display::fmt(item, f)?;
        }
        f.write_str("]")
    }
}

impl<T: PartialEq> PartialEq for RingBuffer<T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().zip(other.iter()).all(|(a, b)| a == b)
    }
}

impl<T: Eq> Eq for RingBuffer<T> {}
