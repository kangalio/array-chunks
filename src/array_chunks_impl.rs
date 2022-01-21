//! Actual implementation of the iterator. This module should be kept as small as possible to
//! minimize the amount of code that could possibly violate this type's invariants and cause UB

use core::mem::MaybeUninit;

/// Iterator adapter like [`slice::array_chunks`] but for any iterator
#[derive(Debug)]
pub struct ArrayChunks<I, T, const N: usize> {
    iter: I,
    buf: [MaybeUninit<T>; N],
    num_init: usize,
}

impl<I, T, const N: usize> ArrayChunks<I, T, N> {
    /// Creates a new [`ArrayChunks`] iterator adapter from the given source iterator
    pub fn new(iter: I) -> Self {
        Self {
            iter,
            buf: MaybeUninit::uninit_array(),
            num_init: 0,
        }
    }

    /// If this iterator is exhausted, the remaining items that did not fit in a chunk are returned.
    /// Otherwise, an empty slice is returned
    pub fn remainder(&self) -> &[T] {
        // SAFETY: the Iterator::next() implementation ensures buf[..num_init] is in an initialized
        // state at any point in time
        unsafe { MaybeUninit::slice_assume_init_ref(&self.buf[..self.num_init]) }
    }
}

impl<I, T, const N: usize> Iterator for ArrayChunks<I, T, N>
where
    I: Iterator<Item = T>,
{
    type Item = [T; N];

    fn next(&mut self) -> Option<Self::Item> {
        for slot in &mut self.buf[self.num_init..] {
            *slot = MaybeUninit::new(self.iter.next()?);
            self.num_init += 1;
        }
        // SAFETY: array_assume_init: at this point, we have completely iterated through
        // self.buf and set each item to an instance of MaybeUninit::new(). Therefore, the
        // entire array is in an initialized state, as array_assume_init requires.
        // SAFETY: std::ptr::read: self.num_init is set to zero immediately after this, so the
        // items from buf we're cloning out will never be read again. Therefore, those items
        // won't be duplicated.
        let chunk = unsafe { MaybeUninit::array_assume_init(core::ptr::read(&self.buf)) };
        self.num_init = 0;
        Some(chunk)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (min_items, max_items) = self.iter.size_hint();
        let min_chunks = (min_items.saturating_add(self.num_init)) / N;
        let max_chunks =
            max_items.and_then(|max_items| Some(max_items.checked_add(self.num_init)? / N));
        (min_chunks, max_chunks)
    }
}

impl<I, T, const N: usize> Drop for ArrayChunks<I, T, N> {
    fn drop(&mut self) {
        for item in &self.buf[..self.num_init] {
            // SAFETY: the Iterator::next() implementation ensures buf[..num_init] is in an
            // initialized state at any point in time
            unsafe { drop(item.assume_init_read()) }
        }
    }
}
