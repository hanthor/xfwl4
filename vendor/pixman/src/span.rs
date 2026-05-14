use crate::{ffi, Fixed};

/// A single span
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Span(ffi::pixman_span_fix_t);

impl Span {
    /// Initialize the span with from the provided components
    #[inline]
    pub fn new(l: impl Into<Fixed>, r: impl Into<Fixed>, y: impl Into<Fixed>) -> Self {
        Self(ffi::pixman_span_fix_t {
            l: l.into().into_raw(),
            r: r.into().into_raw(),
            y: y.into().into_raw(),
        })
    }

    /// Get the l component of this span
    #[inline]
    pub fn l(&self) -> Fixed {
        Fixed::from(self.0.l)
    }

    /// Get the r component of this span
    #[inline]
    pub fn r(&self) -> Fixed {
        Fixed::from(self.0.r)
    }

    /// Get the y component of this span
    #[inline]
    pub fn y(&self) -> Fixed {
        Fixed::from(self.0.y)
    }
}

impl<T: Into<Fixed> + Copy> From<[T; 3]> for Span {
    #[inline]
    fn from(value: [T; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<ffi::pixman_span_fix_t> for Span {
    #[inline]
    fn from(value: ffi::pixman_span_fix_t) -> Self {
        Self(value)
    }
}

impl From<Span> for ffi::pixman_span_fix_t {
    #[inline]
    fn from(value: Span) -> Self {
        value.0
    }
}
