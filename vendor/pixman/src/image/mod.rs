use std::os::raw::c_int;

use pixman_sys as ffi;
use thiserror::Error;

use crate::{
    repeat::Repeat, Dither, Filter, Fixed, OperationFailed, Region16, Region32, Transform,
};

mod bits;
mod conical_gradient;
mod linear_gradient;
mod radial_gradient;
mod solid;

pub use bits::*;
pub use conical_gradient::ConicalGradient;
pub use linear_gradient::LinearGradient;
pub use radial_gradient::RadialGradient;
pub use solid::Solid;

/// Resource creation failed
#[derive(Debug, Error)]
#[error("Resource creation failed")]
pub struct CreateFailed;

/// A reference to a raw image
#[derive(Debug)]
pub struct ImageRef(*mut ffi::pixman_image_t);

// SAFETY: See `Image`.
#[cfg(feature = "sync")]
unsafe impl Send for ImageRef {}
#[cfg(feature = "sync")]
unsafe impl Sync for ImageRef {}

impl ImageRef {
    /// Set the repeat operation for this image
    pub fn set_repeat(&mut self, repeat: Repeat) {
        unsafe {
            ffi::pixman_image_set_repeat(self.0, repeat.into());
        }
    }

    /// Apply the specified transform during sampling from this image
    pub fn set_transform(
        &mut self,
        transform: impl Into<Transform>,
    ) -> Result<(), OperationFailed> {
        let transform = transform.into();
        let res = unsafe { ffi::pixman_image_set_transform(self.0, transform.as_ptr()) };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Clear a previously applied transform
    pub fn clear_transform(&mut self) -> Result<(), OperationFailed> {
        let res = unsafe { ffi::pixman_image_set_transform(self.0, std::ptr::null()) };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Apply a clip region used during composition
    pub fn set_clip_region(&mut self, region: Option<&Region16>) -> Result<(), OperationFailed> {
        let region = if let Some(region) = region {
            region.as_ptr()
        } else {
            std::ptr::null()
        };
        let res = unsafe { ffi::pixman_image_set_clip_region(self.0, region) };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Apply a clip region used during composition
    pub fn set_clip_region32(&mut self, region: Option<&Region32>) -> Result<(), OperationFailed> {
        let region = if let Some(region) = region {
            region.as_ptr()
        } else {
            std::ptr::null()
        };
        let res = unsafe { ffi::pixman_image_set_clip_region32(self.0, region) };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Set the dither operation used during composition
    pub fn set_dither(&mut self, dither: Dither) {
        unsafe {
            ffi::pixman_image_set_dither(self.0, dither.into());
        }
    }

    /// Set the dither offset
    pub fn set_dither_offset(&mut self, offset_x: c_int, offset_y: c_int) {
        unsafe { ffi::pixman_image_set_dither_offset(self.0, offset_x, offset_y) }
    }

    /// Set the filter operation used during composition
    pub fn set_filter(
        &mut self,
        filter: Filter,
        filter_params: &[Fixed],
    ) -> Result<(), OperationFailed> {
        let res = unsafe {
            ffi::pixman_image_set_filter(
                self.0,
                filter.into(),
                filter_params.as_ptr() as *const _,
                filter_params.len() as i32,
            )
        };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Set whether the source clip was set by a client
    pub fn set_has_client_clip(&mut self, client_clip: bool) {
        let client_clip = if client_clip { 1 } else { 0 };
        unsafe {
            ffi::pixman_image_set_has_client_clip(self.0, client_clip);
        }
    }

    // TODO: pixman_image_set_indexed
    //pub fn set_indexed(&mut self)

    /// Set whether the clip applies when the image is used as a source
    pub fn set_source_clipping(&mut self, source_clipping: bool) {
        let source_clipping = if source_clipping { 1 } else { 0 };
        unsafe {
            ffi::pixman_image_set_source_clipping(self.0, source_clipping);
        }
    }

    /// Whether the image has component alpha or unified alpha
    pub fn component_alpha(&self) -> bool {
        unsafe { ffi::pixman_image_get_component_alpha(self.0) == 1 }
    }

    /// Set whether the image has component alpha or unified alpha
    pub fn set_component_alpha(&mut self, component_alpha: bool) {
        let component_alpha = if component_alpha { 1 } else { 0 };
        unsafe { ffi::pixman_image_set_component_alpha(self.0, component_alpha) }
    }
}

impl ImageRef {
    /// Create a reference to a raw image
    ///
    /// # Safety
    ///
    /// The pointer is expected to be valid and have a ref-count of at least one.
    /// Ownership of the pointer is transferred and unref will be called on drop.
    pub unsafe fn from_ptr(ptr: *mut ffi::pixman_image_t) -> Self {
        assert!(!ptr.is_null());
        ImageRef(ptr)
    }

    /// Access the raw image pointer
    pub fn as_ptr(&self) -> *mut ffi::pixman_image_t {
        self.0
    }
}

impl Drop for ImageRef {
    fn drop(&mut self) {
        #[cfg(feature = "sync")]
        let _lock = crate::REF_COUNT_LOCK.lock().unwrap();
        unsafe {
            ffi::pixman_image_unref(self.0);
        }
    }
}

macro_rules! image_type {
    ($(#[$attr:meta])* $name:ident) => {
        $(#[$attr])*
        pub struct $name<'alpha> {
            image: $crate::ImageRef,
            _phantom: std::marker::PhantomData<&'alpha ()>,
        }

        impl<'a> $name<'a> {
            /// Set an alpha map that will be used when this image is
            /// used as a src in a blit operation
            pub fn set_alpha_map<'alpha: 'a>(
                self,
                alpha_map: &'alpha crate::Image<'_, 'static>,
                x: i16,
                y: i16,
            ) -> $name<'alpha> {
                #[cfg(feature = "sync")]
                let _lock = $crate::REF_COUNT_LOCK.lock().unwrap();
                unsafe {
                    $crate::ffi::pixman_image_set_alpha_map(
                        self.as_ptr(),
                        alpha_map.as_ptr(),
                        x,
                        y,
                    );
                }
                $name {
                    image: self.image,
                    _phantom: std::marker::PhantomData,
                }
            }

            /// Clear a previously set alpha map
            pub fn clear_alpha_map(self) -> $name<'static> {
                #[cfg(feature = "sync")]
                let _lock = $crate::REF_COUNT_LOCK.lock().unwrap();
                unsafe {
                    $crate::ffi::pixman_image_set_alpha_map(
                        self.as_ptr(),
                        std::ptr::null_mut(),
                        0,
                        0,
                    );
                }
                $name {
                    image: self.image,
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<'alpha> $name<'alpha> {
            /// Initialize the image from a raw pointer
            ///
            /// # Safety
            ///
            /// The pointer is expected to be valid and have a ref-count of at least one.
            /// Ownership of the pointer is transferred and unref will be called on drop.
            pub unsafe fn from_ptr(ptr: *mut ffi::pixman_image_t) -> Self {
                Self {
                    image: $crate::ImageRef::from_ptr(ptr),
                    _phantom: std::marker::PhantomData,
                }
            }
        }

        impl<'alpha> std::ops::Deref for $name<'alpha> {
            type Target = $crate::ImageRef;

            fn deref(&self) -> &Self::Target {
                &self.image
            }
        }

        impl<'alpha> std::ops::DerefMut for $name<'alpha> {
            fn deref_mut(&mut self) -> &mut Self::Target {
                &mut self.image
            }
        }
    };
}

pub(crate) use image_type;
