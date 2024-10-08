//! A growable ring buffer implementation.
//!
//! https://en.wikipedia.org/wiki/Circular_buffer#:~:text=In%20computer%20science%2C%20a%20circular,easily%20to%20buffering%20data%20streams.
//! https://stackoverflow.com/questions/49072494/how-does-the-vecdeque-ring-buffer-work-internally
//! https://doc.rust-lang.org/nomicon/vec/vec-push-pop.html
//!
//!
//!
#![allow(dead_code)]
use std::{alloc::Layout, ops::Index, ptr::NonNull};

/// A ring buffer.
/// Contains a pointer to the buffer, the allocated capacity, the current length of the buffer and
/// the head index.
#[derive(Debug)]
pub struct RingBuffer<T> {
    ptr: NonNull<T>,
    capacity: usize,
    head: usize,
    len: usize,
}

impl<T> RingBuffer<T> {
    pub fn new() -> Self {
        assert!(std::mem::size_of::<T>() != 0, "ZSTs are not supported.");
        RingBuffer {
            ptr: NonNull::dangling(),
            capacity: 0,
            head: 0,
            len: 0,
        }
    }

    /// Return true if the buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Return the number of elements in the buffer.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Get the item at position `index` in the buffer.
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<&T> {
        if index >= self.len {
            return None;
        }

        let idx = (self.head + index) % self.capacity;
        unsafe { Some(&*self.ptr.as_ptr().add(idx)) }
    }

    /// Insert an item at the front of the buffer.
    pub fn push_front(&mut self, item: T) {
        if self.is_full() {
            self.grow();
        }

        let index = self.head.wrapping_sub(1) % self.capacity;

        unsafe {
            self.ptr.as_ptr().add(index).write(item);
        }

        self.head = index;
        self.len += 1;
    }

    /// Insert an item at the end of the buffer.
    pub fn push_back(&mut self, item: T) {
        if self.is_full() {
            self.grow();
        }

        let index = (self.head + self.len) % self.capacity;

        unsafe {
            self.ptr.as_ptr().add(index).write(item);
        }
        self.len += 1;
    }

    /*
     * Private methods.
     */

    /// Return true if the buffer is full and needs to be grown before elements can be added.
    fn is_full(&self) -> bool {
        self.len == self.capacity
    }

    /// Grow the buffer by doubling its capacity.
    fn grow(&mut self) {
        let new_cap = if self.capacity == 0 {
            1
        } else {
            self.capacity * 2
        };

        // Safe unwrap because we know that `new_cap` is <= usize::MAX
        let new_layout = Layout::array::<T>(new_cap).unwrap();

        assert!(
            new_layout.size() <= isize::MAX as usize,
            "capacity overflow"
        );

        let new_ptr = if self.capacity == 0 {
            // Allocate a new buffer
            unsafe { std::alloc::alloc(new_layout) }
        } else {
            let old_layout = Layout::array::<T>(self.capacity).unwrap();
            let old_ptr = self.ptr.as_ptr() as *mut u8;
            unsafe { std::alloc::realloc(old_ptr, old_layout, new_layout.size()) }
        };

        // Abort if the allocation fails
        self.ptr = match NonNull::new(new_ptr as *mut T) {
            Some(ptr) => ptr,
            None => std::alloc::handle_alloc_error(new_layout),
        };

        // If the buffer wrapped around the end of the allocated memory, we need to update it now
        // that we have re-allocated.
        if self.head != 0 {
            if (self.capacity - self.head) >= self.head {
                // If the portion from the start of the buffer is smaller than the rest then we
                // move it to the end.
                //
                //        H                 C
                // [o, o, o, o, o, o, o, o]
                //
                //           H                        H+L
                // -> [., ., o, o, o, o, o, o, o, o., ., ., ., ., ., .]
                unsafe {
                    self.ptr
                        .as_ptr()
                        .copy_to(self.ptr.as_ptr().add(self.capacity), self.head);
                }
            } else {
                // If the portion from the start of the buffer is smaller than the rest then we
                // move it to the end.
                //
                //                    H     C
                // [o, o, o, o, o, o, o, o]
                //
                //                      H+L                      H    C
                // -> [o, o, o, o, o, o, ., ., ., ., ., ., ., ., o, o]
                unsafe {
                    self.ptr.as_ptr().add(self.head).copy_to(
                        self.ptr.as_ptr().add(new_cap - self.head),
                        self.capacity - self.head,
                    );
                }
                self.head = new_cap - self.head;
            }
        }
        self.capacity = new_cap;
    }
}

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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_new() {
        let rb = RingBuffer::<i32>::new();
        assert_eq!(rb.capacity, 0);
        assert_eq!(rb.head, 0);
        assert_eq!(rb.len, 0);
    }

    #[test]
    fn test_grow() {
        let mut rb = RingBuffer::<i32>::new();
        rb.grow();
        assert_eq!(rb.capacity, 1);
        rb.grow();
        assert_eq!(rb.capacity, 2);
        rb.grow();
        assert_eq!(rb.capacity, 4);
        assert!(!rb.is_full());
        assert!(rb.is_empty());
    }

    #[test]
    fn test_push_back() {
        let mut rb = RingBuffer::<i32>::new();

        rb.push_back(1);
        assert_eq!(rb.len, 1);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert!(rb.get(1).is_none());

        rb.push_back(2);
        assert_eq!(rb.len, 2);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert_eq!(rb[1], 2);

        rb.push_back(3);
        assert_eq!(rb.len, 3);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert_eq!(rb[1], 2);
        assert_eq!(rb[2], 3);
    }

    #[test]
    fn test_push_front() {
        let mut rb = RingBuffer::<i32>::new();

        rb.push_front(1);
        assert_eq!(rb.len, 1);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 1);
        assert!(rb.get(1).is_none());

        rb.push_front(2);
        assert_eq!(rb.len, 2);
        assert_eq!(rb.head, 1);
        assert_eq!(rb[0], 2);
        assert_eq!(rb[1], 1);

        //     H         H               H  (see `grow` for why this happens)
        // [1, 2] -> [1, 2, ., .] -> [., 2, 1, .] -> [3, 2, 1, .]
        rb.push_front(3);
        assert_eq!(rb.len, 3);
        assert_eq!(rb.head, 0);
        assert_eq!(rb[0], 3);
        assert_eq!(rb[1], 2);
        assert_eq!(rb[2], 1);
    }
}
