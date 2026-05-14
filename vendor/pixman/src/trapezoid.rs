use crate::{ffi, Fixed, Line};

/// A trapezoid
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Trapezoid(ffi::pixman_trapezoid_t);

impl Trapezoid {
    /// Initialize a trapezoid from the provided values
    #[inline]
    pub fn new(
        top: impl Into<Fixed>,
        bottom: impl Into<Fixed>,
        left: impl Into<Line>,
        right: impl Into<Line>,
    ) -> Self {
        Self(ffi::pixman_trapezoid_t {
            top: top.into().into_raw(),
            bottom: bottom.into().into_raw(),
            left: left.into().into(),
            right: right.into().into(),
        })
    }

    /// Access the top component of the trapezoid
    #[inline]
    pub fn top(&self) -> Fixed {
        Fixed::from(self.0.top)
    }

    /// Access the bottom component of the trapezoid
    #[inline]
    pub fn bottom(&self) -> Fixed {
        Fixed::from(self.0.bottom)
    }

    /// Access the left component of the trapezoid
    #[inline]
    pub fn left(&self) -> Line {
        Line::from(self.0.left)
    }

    /// Access the right component of the trapezoid
    #[inline]
    pub fn right(&self) -> Line {
        Line::from(self.0.right)
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const ffi::pixman_trapezoid_t {
        &self.0
    }
}

impl From<ffi::pixman_trapezoid_t> for Trapezoid {
    #[inline]
    fn from(value: ffi::pixman_trapezoid_t) -> Self {
        Self(value)
    }
}

impl From<Trapezoid> for ffi::pixman_trapezoid_t {
    #[inline]
    fn from(value: Trapezoid) -> Self {
        value.0
    }
}
