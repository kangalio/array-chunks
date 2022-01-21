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
