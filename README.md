# Ring Buffer

A growable ring buffer implemented in Rust.

A ring buffer is a data type that stores an ordered array of elements which is efficient to insert/remove from both ends. The implementation is based on the description [here](https://en.wikipedia.org/wiki/Circular_buffer) as well as from the implementation of `Vec` described in the [Rustonomicon](https://doc.rust-lang.org/nomicon/vec/vec.html). The API is a subset of the the standard library's `VecDeque`.

## Usage

```rust
use ring_buffer::RingBuffer;

let mut ring_buffer = RingBuffer::new();
ring_buffer.push_front(1);
ring_buffer.push_front(2);

assert_eq!(ring_buffer.pop_back(), Some(1));
```
