#![doc = include_str!("../README.md")]
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

    // Used to cause double-free
    // https://discord.com/channels/273534239310479360/933858432954536027/934038098520719361
    #[test]
    fn non_fused_source_iterator_1() {
        let mut toggle = false;
        let iter = std::iter::from_fn(|| {
            toggle = !toggle;
            if toggle {
                Some(Box::new("hi"))
            } else {
                None
            }
        });

        let mut chunks = iter.array_chunks::<2>();
        chunks.next();
        chunks.next();
    }

    #[test]
    fn non_fused_source_iterator_2() {
        let mut toggle = false;
        let mut i = 0;
        let iter = std::iter::from_fn(|| {
            toggle = !toggle;
            if toggle {
                i += 1;
                Some(i)
            } else {
                None
            }
        });

        let mut chunks = iter.array_chunks::<2>();

        assert_eq!(chunks.next(), None); // reading Some(1) and None
        assert_eq!(chunks.remainder(), &[1]);
        assert_eq!(chunks.next(), Some([1, 2])); // reading Some(2)
        assert_eq!(chunks.remainder(), &[]);
        assert_eq!(chunks.next(), None); // reading None
        assert_eq!(chunks.remainder(), &[]);

        assert_eq!(chunks.next(), None); // reading Some(3) and None
        assert_eq!(chunks.remainder(), &[3]);
        assert_eq!(chunks.next(), Some([3, 4])); // reading Some(4)
        assert_eq!(chunks.remainder(), &[]);
        assert_eq!(chunks.next(), None); // reading None
        assert_eq!(chunks.remainder(), &[]);
    }

    #[test]
    fn memory_leak() {
        let mut chunks = [Box::new(123), Box::new(234)]
            .into_iter()
            .array_chunks::<3>();

        assert_eq!(chunks.next(), None);
        // The two elements are now in the half-initialized buffer. Let's check if the ArrayChunks
        // Drop impl deinitializes them properly
        drop(chunks);
    }

    #[test]
    fn debug() {
        let chunks = (0..7).array_chunks::<3>();
        assert_eq!(
            format!("{:?}", chunks),
            "ArrayChunks { iter: 0..7, buf: [\
                core::mem::maybe_uninit::MaybeUninit<i32>, \
                core::mem::maybe_uninit::MaybeUninit<i32>, \
                core::mem::maybe_uninit::MaybeUninit<i32>\
            ], num_init: 0 }"
        );
    }

    #[test]
    fn size_hint() {
        let mut chunks = (0..7).array_chunks::<3>();

        assert_eq!(chunks.size_hint(), (2, Some(2)));
        assert_eq!(chunks.next(), Some([0, 1, 2]));
        assert_eq!(chunks.size_hint(), (1, Some(1)));

        let mut chunks = (0..).array_chunks::<3>();

        assert_eq!(chunks.size_hint(), (usize::MAX / 3, None));
        assert_eq!(chunks.next(), Some([0, 1, 2]));
        assert_eq!(chunks.size_hint(), (usize::MAX / 3, None));
    }

    #[test]
    fn clone() {
        #[derive(Clone)]
        struct NonFusedIterator(bool);
        impl Iterator for NonFusedIterator {
            type Item = Box<u32>;

            fn next(&mut self) -> Option<Self::Item> {
                self.0 = !self.0;
                if self.0 {
                    Some(Box::new(123))
                } else {
                    None
                }
            }
        }
        let iter = NonFusedIterator(false);

        let mut chunks = iter.array_chunks();
        assert_eq!(chunks.next(), None);

        let mut chunks2 = chunks.clone();
        assert_eq!(chunks.next(), Some([Box::new(123), Box::new(123)]));
        assert_eq!(chunks2.next(), Some([Box::new(123), Box::new(123)]));
    }

    // TODO: try to get this working. The problem is std::panic::catch_unwind requires the closure
    // to be UnwindSafe which it isn't (for some reason) due to capturing &mut ArrayChunks.
    /* #[test]
    fn panicky_next() {
        // This iterator yields 1, 2... 7, then panics. After the panic, 9, 10... is yielded
        let panics_at_eight = (1..).inspect(|&i| {
            if i == 8 {
                std::panic::resume_unwind(Box::new("panic payload"));
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
