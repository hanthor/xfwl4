use std::ffi::c_int;

use crate::{ffi, image_type, CreateFailed, Fixed, GradientStop, Point};

image_type! {
    /// Conical gradient image
    #[derive(Debug)]
    ConicalGradient
}

impl ConicalGradient<'static> {
    /// Create a new conical gradient image usable as the src in blit operations
    pub fn new(
        center: impl Into<Point>,
        angle: impl Into<Fixed>,
        stops: &[GradientStop],
    ) -> Result<Self, CreateFailed> {
        let center = center.into();
        let angle = angle.into();
        let ptr = unsafe {
            ffi::pixman_image_create_conical_gradient(
                center.as_ptr(),
                angle.into_raw(),
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
