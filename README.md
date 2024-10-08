# Ring Buffer

A growable ring buffer implemented in Rust.

A ring buffer is a data type that stores an ordered array of elements which is efficient to insert/remove from both ends. The implementation is based on the description [here](https://en.wikipedia.org/wiki/Circular_buffer) and takes inspiration from the implementation of `Vec` described in the [Rustonomicon](https://doc.rust-lang.org/nomicon/vec/vec.html).
