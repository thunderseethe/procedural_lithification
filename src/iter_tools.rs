//! Custom implementation itertools::Itertools::all_equal
//! The itertools implementation is prohibitively slow and unconditionally touches the whole iterator.
//! This is particularly critical to chunk generation where all_equal is in a hot path.

#[inline]
pub fn all_equal<I>(mut iter: I) -> bool
where
    I: Iterator,
    I::Item: PartialEq,
{
    iter.next()
        .map(|head| iter.all(|ele| head == ele))
        .unwrap_or(true)
}