use std::ffi::c_int;

use crate::{ffi, image_type, CreateFailed, GradientStop, Point};

image_type! {
    /// Linear gradient image
    #[derive(Debug)]
    LinearGradient
}

impl LinearGradient<'static> {
    /// Create a new linear gradient image usable as the src in blit operations
    pub fn new(
        p1: impl Into<Point>,
        p2: impl Into<Point>,
        stops: &[GradientStop],
    ) -> Result<Self, CreateFailed> {
        let p1: Point = p1.into();
        let p2: Point = p2.into();
        let ptr = unsafe {
            ffi::pixman_image_create_linear_gradient(
                p1.as_ptr(),
                p2.as_ptr(),
                stops.as_ptr() as *const _,
                stops.len() as c_int,
            )
        };
        if ptr.is_null() {
            Err(CreateFailed)
        } else {
            Ok(unsafe { Self::from_ptr(ptr) })
        }
    }
}
