/*!
This crate implements an `array_chunks` method for iterators. It behaves like
[`slice::array_chunks`] but works with any [`Iterator`] type.

Several nightly features related to [`std::mem::MaybeUninit`] are used, so this crate cannot be used
on stable. That's because this crate was written mainly to check viability of an `array_chunks`
method on [`Iterator`] for potential inclusion to the standard library.

This crate is `no_std` compatible.

```rust
use array_chunks::IteratorExt;

let mut chunks = (1..=7).array_chunks();

assert_eq!(chunks.next(), Some([1, 2, 3]));
assert_eq!(chunks.next(), Some([4, 5, 6]));
assert_eq!(chunks.next(), None);
assert_eq!(chunks.remainder(), &[7]);
```
*/

#![cfg_attr(not(any(test, doc)), no_std)]
#![feature(maybe_uninit_slice)]
#![feature(maybe_uninit_array_assume_init)]
#![feature(maybe_uninit_uninit_array)]
#![warn(missing_docs)]

mod array_chunks_impl;
pub use array_chunks_impl::ArrayChunks;

/// A simple [`Iterator`] extension trait that only provides the [`IteratorExt::array_chunks()`]
/// method, which internally calls [`ArrayChunks::new()`].
pub trait IteratorExt: Iterator + Sized {
    /// Returns an iterator of chunks, where each chunk contains N items from the source iterator
    fn array_chunks<const N: usize>(self) -> ArrayChunks<Self, Self::Item, N> {
        ArrayChunks::new(self)
    }
}
impl<I: Iterator> IteratorExt for I {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let mut chunks = (1..=7).array_chunks();

        assert_eq!(chunks.next(), Some([1, 2, 3]));
        assert_eq!(chunks.next(), Some([4, 5, 6]));
        assert_eq!(chunks.remainder(), &[]);
        assert_eq!(chunks.next(), None);
        assert_eq!(chunks.remainder(), &[7]);
    }

    #[test]
    fn empty() {
        let mut chunks = std::iter::empty::<()>().array_chunks::<3>();

        assert_eq!(chunks.next(), None);
        assert_eq!(chunks.remainder(), &[]);
    }

    #[test]
    fn no_remainder() {
        let mut chunks = [1, 2, 3].into_iter().array_chunks();

        assert_eq!(chunks.next(), Some([1, 2, 3]));
        assert_eq!(chunks.next(), None);
        assert_eq!(chunks.remainder(), &[]);
    }

    #[test]
    fn zero_size_arrays() {
        let mut chunks = [1, 2, 3].into_iter().array_chunks::<0>();

        assert_eq!(chunks.next(), Some([]));
        assert_eq!(chunks.next(), Some([]));
    }

    // To test memory leaks in miri
    #[test]
    fn string() {
        let vec = vec![
            String::from("apple"),
            String::from("banana"),
            String::from("cucumber"),
            String::from("date"),
            String::from("eggfruit"),
        ];
        let mut chunks = vec.iter().array_chunks();

        assert_eq!(chunks.next(), Some([&vec[0], &vec[1]]));
        assert_eq!(chunks.next(), Some([&vec[2], &vec[3]]));
        assert_eq!(chunks.next(), None);
        assert_eq!(chunks.remainder(), &[&vec[4]]);
        assert_eq!(chunks.next(), None);
    }

    // TODO: try to get this working. The problem is std::panic::catch_unwind requires the closure
    // to be UnwindSafe which it isn't (for some reason) due to capturing &mut ArrayChunks.
    /* #[test]
    fn panicky_next() {
        // This iterator yields 1, 2... 7, then panics. After the panic, 9, 10... is yielded
        let panics_at_eight = (1..).inspect(|&i| {
            if i == 8 {
                panic!();
            }
        });
        let mut chunks = panics_at_eight.array_chunks();

        assert_eq!(chunks.next(), Some([1, 2, 3, 4, 5]));
        assert!(std::panic::catch_unwind(|| {
            chunks.next();
        })
        .is_err());
        assert_eq!(chunks.remainder(), &[6, 7]);
        assert_eq!(chunks.next(), Some([42, 69, 1337, 666, 727]));
    } */
}
