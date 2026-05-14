use std::ffi::c_int;

use crate::{ffi, image_type, CreateFailed, Fixed, GradientStop, Point};

image_type! {
    /// Radial gradient image
    #[derive(Debug)]
    RadialGradient
}

impl RadialGradient<'static> {
    /// Create a new radial gradient image usable as the src in blit operations
    pub fn new(
        inner: impl Into<Point>,
        outer: impl Into<Point>,
        inner_radius: impl Into<Fixed>,
        outer_radius: impl Into<Fixed>,
        stops: &[GradientStop],
    ) -> Result<Self, CreateFailed> {
        let inner = inner.into();
        let outer = outer.into();
        let inner_radius = inner_radius.into();
        let other_radius = outer_radius.into();
        let ptr = unsafe {
            ffi::pixman_image_create_radial_gradient(
                inner.as_ptr(),
                outer.as_ptr(),
                inner_radius.into_raw(),
                other_radius.into_raw(),
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
