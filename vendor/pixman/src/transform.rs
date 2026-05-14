use std::mem::MaybeUninit;

use paste::paste;
use thiserror::Error;

use crate::{ffi, FVector, Fixed, Vector};

macro_rules! impl_transform {
    ($(#[$attr:meta])* $name:ident, ffi => $ffi:path, impl => $impl:ident, vector_t => $vector_t:path$(,)?) => {
        $(#[$attr])*
        pub struct $name($ffi);

        impl $name {
            /// Transform the provided bounds
            pub fn transform_bounds(&self, mut b: $crate::Box16) -> Option<$crate::Box16> {
                let res = unsafe { paste!($crate::ffi::[<$impl _bounds>](self.as_ptr(), &mut b)) };
                if res == 1 {
                    Some(b)
                } else {
                    None
                }
            }

            /// Initialize an identity transform
            #[inline]
            pub fn identity() -> Self {
                let mut transform = MaybeUninit::uninit();
                unsafe {
                    paste!($crate::ffi::[<$impl _init_identity>](transform.as_mut_ptr()));
                }
                Self(unsafe { transform.assume_init() })
            }

            /// Invert this transform
            pub fn invert(&self) -> Option<Self> {
                let mut transform = MaybeUninit::uninit();
                let res = unsafe { paste!($crate::ffi::[<$impl _invert>](transform.as_mut_ptr(), self.as_ptr())) };

                if res == 1 {
                    Some(Self(unsafe { transform.assume_init() }))
                } else {
                    None
                }
            }

            /// Multiply this transform with the provided transform
            pub fn multiply(&self, other: &$name) -> Self {
                let mut transform = MaybeUninit::uninit();
                unsafe {
                    paste!($crate::ffi::[<$impl _multiply>](transform.as_mut_ptr(), self.as_ptr(), other.as_ptr()));
                };
                Self(unsafe { transform.assume_init() })
            }

            /// Transform the given point
            pub fn transform_point(&self, mut vector: $vector_t) -> Option<$vector_t> {
                let res = unsafe { paste!($crate::ffi::[<$impl _point>](self.as_ptr(), vector.as_mut_ptr())) };
                if res == 1 {
                    Some(vector)
                } else {
                    None
                }
            }

            /// Transform the given point
            pub fn transform_point_3d(&self, mut vector: $vector_t) -> $vector_t {
                unsafe { paste!($crate::ffi::[<$impl _point_3d>](self.as_ptr(), vector.as_mut_ptr())) };
                vector
            }

            #[inline]
            pub(crate) fn as_ptr(&self) -> *const $ffi {
                &self.0 as *const $ffi
            }

            #[inline]
            pub(crate) fn as_mut_ptr(&mut self) -> *mut $ffi {
                &mut self.0 as *mut $ffi
            }
        }

        impl From<$ffi> for $name {
            #[inline]
            fn from(value: $ffi) -> Self {
                Self(value)
            }
        }

        impl From<$name> for $ffi {
            #[inline]
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

impl_transform! {
    /// Fixed-point transform
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    Transform,
    ffi => crate::ffi::pixman_transform_t,
    impl => pixman_transform,
    vector_t => Vector,
}

impl_transform! {
    /// Floating-point transform
    #[derive(Debug, Clone, Copy)]
    #[repr(transparent)]
    FTransform,
    ffi => crate::ffi::pixman_f_transform_t,
    impl => pixman_f_transform,
    vector_t => FVector,
}

impl Transform {
    /// Initialize a transform from the provided matrix
    #[inline]
    pub fn new<T: Into<Fixed> + Copy>(matrix: [[T; 3]; 3]) -> Self {
        let matrix = [
            [
                matrix[0][0].into().into_raw(),
                matrix[0][1].into().into_raw(),
                matrix[0][2].into().into_raw(),
            ],
            [
                matrix[1][0].into().into_raw(),
                matrix[1][1].into().into_raw(),
                matrix[1][2].into().into_raw(),
            ],
            [
                matrix[2][0].into().into_raw(),
                matrix[2][1].into().into_raw(),
                matrix[2][2].into().into_raw(),
            ],
        ];
        Self(ffi::pixman_transform { matrix })
    }

    /// Initialize a transform from a rotation
    #[inline]
    pub fn from_rotation(cos: impl Into<Fixed>, sin: impl Into<Fixed>) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_transform_init_rotate(
                transform.as_mut_ptr(),
                cos.into().into_raw(),
                sin.into().into_raw(),
            );
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Initialize a transform from a scale
    #[inline]
    pub fn from_scale(sx: impl Into<Fixed>, sy: impl Into<Fixed>) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_transform_init_scale(
                transform.as_mut_ptr(),
                sx.into().into_raw(),
                sy.into().into_raw(),
            );
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Initialize a transform from a translation
    #[inline]
    pub fn from_translation(tx: impl Into<Fixed>, ty: impl Into<Fixed>) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_transform_init_translate(
                transform.as_mut_ptr(),
                tx.into().into_raw(),
                ty.into().into_raw(),
            );
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Whether this transform represents an identity transform
    pub fn is_identity(&self) -> bool {
        unsafe { ffi::pixman_transform_is_identity(self.as_ptr()) == 1 }
    }

    /// TODO: Docs
    pub fn is_int_translate(&self) -> bool {
        unsafe { ffi::pixman_transform_is_int_translate(self.as_ptr()) == 1 }
    }

    /// Whether this transform represents an inverse transform
    pub fn is_inverse(&self, other: &Transform) -> bool {
        unsafe { ffi::pixman_transform_is_inverse(self.as_ptr(), other.as_ptr()) == 1 }
    }

    /// Whether this transform contains a scale transform
    pub fn is_scale(&self) -> bool {
        unsafe { ffi::pixman_transform_is_scale(self.as_ptr()) == 1 }
    }

    /// Add a rotation to this transform
    pub fn rotate(
        mut self,
        c: impl Into<Fixed>,
        s: impl Into<Fixed>,
        reverse: bool,
    ) -> Option<Self> {
        let c = c.into().into_raw();
        let s = s.into().into_raw();
        let res = if reverse {
            unsafe { ffi::pixman_transform_rotate(std::ptr::null_mut(), self.as_mut_ptr(), c, s) }
        } else {
            unsafe { ffi::pixman_transform_rotate(self.as_mut_ptr(), std::ptr::null_mut(), c, s) }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Add a scale to this transform
    pub fn scale(
        mut self,
        sx: impl Into<Fixed>,
        sy: impl Into<Fixed>,
        reverse: bool,
    ) -> Option<Self> {
        let sx = sx.into().into_raw();
        let sy = sy.into().into_raw();
        let res = if reverse {
            unsafe { ffi::pixman_transform_scale(std::ptr::null_mut(), self.as_mut_ptr(), sx, sy) }
        } else {
            unsafe { ffi::pixman_transform_scale(self.as_mut_ptr(), std::ptr::null_mut(), sx, sy) }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Add a translation to this transform
    pub fn translate(
        mut self,
        tx: impl Into<Fixed>,
        ty: impl Into<Fixed>,
        reverse: bool,
    ) -> Option<Self> {
        let tx = tx.into().into_raw();
        let ty = ty.into().into_raw();
        let res = if reverse {
            unsafe {
                ffi::pixman_transform_translate(std::ptr::null_mut(), self.as_mut_ptr(), tx, ty)
            }
        } else {
            unsafe {
                ffi::pixman_transform_translate(self.as_mut_ptr(), std::ptr::null_mut(), tx, ty)
            }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Access the current transform matrix
    #[inline]
    pub fn matrix(&self) -> [[Fixed; 3]; 3] {
        [
            [
                Fixed::from_raw(self.0.matrix[0][0]),
                Fixed::from_raw(self.0.matrix[0][1]),
                Fixed::from_raw(self.0.matrix[0][2]),
            ],
            [
                Fixed::from_raw(self.0.matrix[1][0]),
                Fixed::from_raw(self.0.matrix[1][1]),
                Fixed::from_raw(self.0.matrix[1][2]),
            ],
            [
                Fixed::from_raw(self.0.matrix[2][0]),
                Fixed::from_raw(self.0.matrix[2][1]),
                Fixed::from_raw(self.0.matrix[2][2]),
            ],
        ]
    }
}

impl<T: Into<Fixed> + Copy> From<[[T; 3]; 3]> for Transform {
    #[inline]
    fn from(value: [[T; 3]; 3]) -> Self {
        Transform::new(value)
    }
}

/// Failed to init Transform from FTransform
#[derive(Debug, Error)]
#[error("Failed to init Transform from FTransform")]
pub struct TransformConvertError;

impl TryFrom<FTransform> for Transform {
    type Error = TransformConvertError;

    fn try_from(value: FTransform) -> Result<Self, Self::Error> {
        let mut transform = MaybeUninit::uninit();
        let res = unsafe {
            ffi::pixman_transform_from_pixman_f_transform(transform.as_mut_ptr(), value.as_ptr())
        };
        if res != 1 {
            Err(TransformConvertError)
        } else {
            Ok(Self(unsafe { transform.assume_init() }))
        }
    }
}

impl FTransform {
    /// Initialize a transform from the provided matrix
    #[inline]
    pub fn new(m: [[f64; 3]; 3]) -> Self {
        let m = [
            [m[0][0], m[0][1], m[0][2]],
            [m[1][0], m[1][1], m[1][2]],
            [m[2][0], m[2][1], m[2][2]],
        ];
        Self(ffi::pixman_f_transform_t { m })
    }

    /// Initialize a transform from a rotation
    #[inline]
    pub fn from_rotation(cos: f64, sin: f64) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_f_transform_init_rotate(transform.as_mut_ptr(), cos, sin);
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Initialize a transform from a scale
    #[inline]
    pub fn from_scale(sx: f64, sy: f64) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_f_transform_init_scale(transform.as_mut_ptr(), sx, sy);
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Initialize a transform from a translation
    #[inline]
    pub fn from_translation(tx: f64, ty: f64) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_f_transform_init_translate(transform.as_mut_ptr(), tx, ty);
        }
        Self(unsafe { transform.assume_init() })
    }

    /// Add a rotation to this transform
    pub fn rotate(mut self, c: f64, s: f64, reverse: bool) -> Option<Self> {
        let res = if reverse {
            unsafe { ffi::pixman_f_transform_rotate(std::ptr::null_mut(), self.as_mut_ptr(), c, s) }
        } else {
            unsafe { ffi::pixman_f_transform_rotate(self.as_mut_ptr(), std::ptr::null_mut(), c, s) }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Add a scale to this transform
    pub fn scale(mut self, sx: f64, sy: f64, reverse: bool) -> Option<Self> {
        let res = if reverse {
            unsafe {
                ffi::pixman_f_transform_scale(std::ptr::null_mut(), self.as_mut_ptr(), sx, sy)
            }
        } else {
            unsafe {
                ffi::pixman_f_transform_scale(self.as_mut_ptr(), std::ptr::null_mut(), sx, sy)
            }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Add a translation to this transform
    pub fn translate(mut self, tx: f64, ty: f64, reverse: bool) -> Option<Self> {
        let res = if reverse {
            unsafe {
                ffi::pixman_f_transform_translate(std::ptr::null_mut(), self.as_mut_ptr(), tx, ty)
            }
        } else {
            unsafe {
                ffi::pixman_f_transform_translate(self.as_mut_ptr(), std::ptr::null_mut(), tx, ty)
            }
        };

        if res == 1 {
            Some(self)
        } else {
            None
        }
    }

    /// Access the current transform matrix
    #[inline]
    pub fn matrix(&self) -> [[f64; 3]; 3] {
        self.0.m
    }
}

impl From<[[f64; 3]; 3]> for FTransform {
    #[inline]
    fn from(value: [[f64; 3]; 3]) -> Self {
        FTransform::new(value)
    }
}

impl From<Transform> for FTransform {
    fn from(value: Transform) -> Self {
        let mut transform = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_f_transform_from_pixman_transform(transform.as_mut_ptr(), value.as_ptr());
        }
        Self(unsafe { transform.assume_init() })
    }
}
