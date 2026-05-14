use crate::{ffi, Fixed};

/// A single vector
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Vector(ffi::pixman_vector_t);

impl Vector {
    /// Initialize a vector from the provided values
    #[inline]
    pub fn new<T: Into<Fixed> + Copy>(vector: [T; 3]) -> Self {
        Self(ffi::pixman_vector {
            vector: [
                vector[0].into().into_raw(),
                vector[1].into().into_raw(),
                vector[2].into().into_raw(),
            ],
        })
    }

    /// Access the x component of this vector
    pub fn x(&self) -> Fixed {
        Fixed::from_raw(self.0.vector[0])
    }

    /// Access the y component of this vector
    pub fn y(&self) -> Fixed {
        Fixed::from_raw(self.0.vector[1])
    }

    /// Access the z component of this vector
    pub fn z(&self) -> Fixed {
        Fixed::from_raw(self.0.vector[2])
    }

    #[inline]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut ffi::pixman_vector_t {
        &mut self.0 as *mut ffi::pixman_vector_t
    }
}

impl<T: Into<Fixed> + Copy> From<[T; 3]> for Vector {
    #[inline]
    fn from(value: [T; 3]) -> Self {
        Self::new(value)
    }
}

impl From<ffi::pixman_vector_t> for Vector {
    #[inline]
    fn from(value: ffi::pixman_vector_t) -> Self {
        Self(value)
    }
}

impl From<Vector> for ffi::pixman_vector_t {
    #[inline]
    fn from(value: Vector) -> Self {
        value.0
    }
}

/// Floating-point vector
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct FVector(ffi::pixman_f_vector_t);

impl FVector {
    /// Initialize the vector from the specified values
    #[inline]
    pub fn new(v: [f64; 3]) -> Self {
        Self(ffi::pixman_f_vector_t {
            v: [v[0], v[1], v[2]],
        })
    }

    /// Access the x component of this vector
    pub fn x(&self) -> f64 {
        self.0.v[0]
    }

    /// Access the y component of this vector
    pub fn y(&self) -> f64 {
        self.0.v[1]
    }

    /// Access the z component of this vector
    pub fn z(&self) -> f64 {
        self.0.v[2]
    }

    #[inline]
    pub(crate) fn as_mut_ptr(&mut self) -> *mut ffi::pixman_f_vector_t {
        &mut self.0 as *mut ffi::pixman_f_vector_t
    }
}

impl From<[f64; 3]> for FVector {
    #[inline]
    fn from(value: [f64; 3]) -> Self {
        Self::new(value)
    }
}

impl From<ffi::pixman_f_vector_t> for FVector {
    #[inline]
    fn from(value: ffi::pixman_f_vector_t) -> Self {
        Self(value)
    }
}

impl From<FVector> for ffi::pixman_f_vector_t {
    #[inline]
    fn from(value: FVector) -> Self {
        value.0
    }
}
