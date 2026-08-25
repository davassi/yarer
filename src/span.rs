//! Byte offsets into the expression an error came from.

/// A half-open range of bytes in the source expression.
///
/// Offsets are in bytes because that is what the tokeniser produces. Turning
/// them into terminal columns is [`crate::error::Error::render`]'s job, and it
/// is not a cast: `×` is two bytes and one column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Offset of the first byte.
    pub start: usize,
    /// Offset one past the last byte.
    pub end: usize,
}

impl Span {
    /// Builds a span from a half-open byte range.
    #[must_use]
    pub fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }
}

/// A value carrying the span of the text it came from.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Spanned<T> {
    pub node: T,
    pub span: Span,
}

impl<T> Spanned<T> {
    pub(crate) fn new(node: T, span: Span) -> Spanned<T> {
        Spanned { node, span }
    }
}
