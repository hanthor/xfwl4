use crate::{ffi, Point};

/// A triangle
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Triangle(ffi::pixman_triangle_t);

impl Triangle {
    /// Initialize the triangle from the provided values
    pub fn new(p1: impl Into<Point>, p2: impl Into<Point>, p3: impl Into<Point>) -> Self {
        Self(ffi::pixman_triangle_t {
            p1: p1.into().into(),
            p2: p2.into().into(),
            p3: p3.into().into(),
        })
    }

    /// Access the first point of this triangle
    #[inline]
    pub fn p1(&self) -> Point {
        self.0.p1.into()
    }

    /// Access the second point of this triangle
    #[inline]
    pub fn p2(&self) -> Point {
        self.0.p2.into()
    }

    /// Access the third point of this triangle
    #[inline]
    pub fn p3(&self) -> Point {
        self.0.p3.into()
    }
}

impl<P: Into<Point> + Copy> From<[P; 3]> for Triangle {
    #[inline]
    fn from(value: [P; 3]) -> Self {
        Self::new(value[0], value[1], value[2])
    }
}

impl From<ffi::pixman_triangle_t> for Triangle {
    #[inline]
    fn from(value: ffi::pixman_triangle_t) -> Self {
        Self(value)
    }
}

impl From<Triangle> for ffi::pixman_triangle_t {
    #[inline]
    fn from(value: Triangle) -> Self {
        value.0
    }
}
