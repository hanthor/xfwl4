use crate::{ffi, image_type, Color, CreateFailed};

image_type! {
    /// Solid color image
    #[derive(Debug)]
    Solid
}

impl Solid<'static> {
    /// Create a new solid color image usable as the src in blit operations
    pub fn new(color: impl Into<Color>) -> Result<Self, CreateFailed> {
        let color = color.into();
        let ptr = unsafe { ffi::pixman_image_create_solid_fill(color.as_ptr()) };

        if ptr.is_null() {
            Err(CreateFailed)
        } else {
            Ok(unsafe { Self::from_ptr(ptr) })
        }
    }
}
