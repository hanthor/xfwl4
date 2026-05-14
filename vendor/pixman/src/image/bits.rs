use std::{ffi::c_int, marker::PhantomData, mem::MaybeUninit};

use crate::{
    ffi, Box32, Color, CreateFailed, Edge, Fixed, FormatCode, ImageRef, Operation, OperationFailed,
    Rectangle16, Region16, Trap, Trapezoid, Triangle,
};

/// Image holding some pixel data
#[derive(Debug)]
pub struct Image<'bits, 'alpha> {
    image: ImageRef,
    _bits: PhantomData<&'bits ()>,
    _alpha: PhantomData<&'alpha ()>,
}

// SAFETY: A reference to the image is only created by `set_alpha_map`.
// Which returns a type with a lifetime bound, so `&mut self` methods cannot
// be called while this additional reference is in use.
//
// Thus the only mutability allowed is reference counting, which is made
// thread-safe with the `REF_COUNT_LOCK` mutex, used when calling
// `pixman_image_unref`, or `pixman_image_set_alpha_map`.
#[cfg(feature = "sync")]
unsafe impl Send for Image<'_, '_> {}
#[cfg(feature = "sync")]
unsafe impl Sync for Image<'_, '_> {}

impl std::ops::Deref for Image<'_, '_> {
    type Target = ImageRef;

    fn deref(&self) -> &Self::Target {
        &self.image
    }
}

impl std::ops::DerefMut for Image<'_, '_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.image
    }
}

impl Image<'static, 'static> {
    /// Create a new image with the specified format and size
    pub fn new(
        format: FormatCode,
        width: usize,
        height: usize,
        clear: bool,
    ) -> Result<Self, CreateFailed> {
        let ptr = if clear {
            unsafe {
                ffi::pixman_image_create_bits(
                    format.into(),
                    width as c_int,
                    height as c_int,
                    std::ptr::null_mut(),
                    0,
                )
            }
        } else {
            unsafe {
                ffi::pixman_image_create_bits_no_clear(
                    format.into(),
                    width as c_int,
                    height as c_int,
                    std::ptr::null_mut(),
                    0,
                )
            }
        };

        if ptr.is_null() {
            Err(CreateFailed)
        } else {
            Ok(unsafe { Self::from_ptr(ptr) })
        }
    }
}

impl<'bits> Image<'bits, 'static> {
    /// Create an image from some pre-allocated pixel data
    pub fn from_slice_mut(
        format: FormatCode,
        width: usize,
        height: usize,
        bits: &'bits mut [u32],
        rowstride_bytes: usize,
        clear: bool,
    ) -> Result<Self, CreateFailed> {
        unsafe {
            Self::from_raw_mut(
                format,
                width,
                height,
                bits.as_mut_ptr(),
                rowstride_bytes,
                clear,
            )
        }
    }

    /// Create an image from some pre-allocated pixel data pointer
    ///
    /// # Safety
    ///
    /// The caller is responsible to make sure the pointer stays
    /// valid for the lifetime of the created image.
    pub unsafe fn from_raw_mut(
        format: FormatCode,
        width: usize,
        height: usize,
        bits: *mut u32,
        rowstride_bytes: usize,
        clear: bool,
    ) -> Result<Self, CreateFailed> {
        let ptr = if clear {
            unsafe {
                ffi::pixman_image_create_bits(
                    format.into(),
                    width as c_int,
                    height as c_int,
                    bits,
                    rowstride_bytes as c_int,
                )
            }
        } else {
            unsafe {
                ffi::pixman_image_create_bits_no_clear(
                    format.into(),
                    width as c_int,
                    height as c_int,
                    bits,
                    rowstride_bytes as c_int,
                )
            }
        };

        if ptr.is_null() {
            Err(CreateFailed)
        } else {
            Ok(unsafe { Self::from_ptr(ptr) })
        }
    }
}

impl<'bits, 'a> Image<'bits, 'a> {
    /// Set an alpha map that will be used when this image is
    /// used as a src in a blit operation
    pub fn set_alpha_map<'alpha: 'a>(
        self,
        alpha_map: &'alpha Image<'_, 'static>,
        x: i16,
        y: i16,
    ) -> Image<'bits, 'alpha> {
        #[cfg(feature = "sync")]
        let _lock = crate::REF_COUNT_LOCK.lock().unwrap();
        unsafe {
            ffi::pixman_image_set_alpha_map(self.as_ptr(), alpha_map.as_ptr(), x, y);
        }
        Image {
            image: self.image,
            _bits: self._bits,
            _alpha: PhantomData,
        }
    }

    /// Clear a previously set alpha map
    pub fn clear_alpha_map(self) -> Image<'bits, 'static> {
        #[cfg(feature = "sync")]
        let _lock = crate::REF_COUNT_LOCK.lock().unwrap();
        unsafe {
            ffi::pixman_image_set_alpha_map(self.as_ptr(), std::ptr::null_mut(), 0, 0);
        }
        Image {
            image: self.image,
            _bits: self._bits,
            _alpha: PhantomData,
        }
    }
}

impl Image<'_, '_> {
    /// Get the width of the image
    pub fn width(&self) -> usize {
        unsafe { ffi::pixman_image_get_width(self.as_ptr()) as usize }
    }

    /// Get the height of the image
    pub fn height(&self) -> usize {
        unsafe { ffi::pixman_image_get_height(self.as_ptr()) as usize }
    }

    /// Get the stride of the image
    pub fn stride(&self) -> usize {
        unsafe { ffi::pixman_image_get_stride(self.as_ptr()) as usize }
    }

    /// Get the depth of the image
    pub fn depth(&self) -> usize {
        unsafe { ffi::pixman_image_get_depth(self.as_ptr()) as usize }
    }

    /// Get the format of the image
    pub fn format(&self) -> FormatCode {
        let format = unsafe { ffi::pixman_image_get_format(self.as_ptr()) };
        FormatCode::from(format)
    }

    /// Access the underlying pixel data
    ///
    /// # Safety
    ///
    /// The pointer is valid for the lifetime of the image
    pub unsafe fn data(&self) -> *mut u32 {
        unsafe { ffi::pixman_image_get_data(self.as_ptr()) }
    }

    /// Fill this image with the specified boxes and color
    pub fn fill_boxes(
        &mut self,
        op: Operation,
        color: impl Into<Color>,
        boxes: &[Box32],
    ) -> Result<(), OperationFailed> {
        let color = color.into();
        let res = unsafe {
            ffi::pixman_image_fill_boxes(
                op.into(),
                self.as_ptr(),
                color.as_ptr(),
                boxes.len() as c_int,
                boxes.as_ptr(),
            )
        };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Fill this image with the specified rectangles and color
    pub fn fill_rectangles(
        &mut self,
        op: Operation,
        color: impl Into<Color>,
        rects: &[Rectangle16],
    ) -> Result<(), OperationFailed> {
        let color = color.into();
        let res = unsafe {
            ffi::pixman_image_fill_rectangles(
                op.into(),
                self.as_ptr(),
                color.as_ptr(),
                rects.len() as c_int,
                rects.as_ptr(),
            )
        };
        if res == 1 {
            Ok(())
        } else {
            Err(OperationFailed)
        }
    }

    /// Composite the specified src image into this image
    #[allow(clippy::too_many_arguments)]
    pub fn composite(
        &mut self,
        operation: Operation,
        src: &ImageRef,
        mask: Option<&ImageRef>,
        src_loc: (i16, i16),
        mask_loc: (i16, i16),
        dest_loc: (i16, i16),
        size: (u16, u16),
    ) {
        let mask_ptr = if let Some(mask) = mask {
            mask.as_ptr()
        } else {
            std::ptr::null_mut()
        };

        unsafe {
            ffi::pixman_image_composite(
                operation.into(),
                src.as_ptr(),
                mask_ptr,
                self.as_ptr(),
                src_loc.0,
                src_loc.1,
                mask_loc.0,
                mask_loc.1,
                dest_loc.0,
                dest_loc.1,
                size.0,
                size.1,
            )
        }
    }

    /// Composite the specified src image into this image
    #[allow(clippy::too_many_arguments)]
    pub fn composite32(
        &mut self,
        operation: Operation,
        src: &ImageRef,
        mask: Option<&ImageRef>,
        src_loc: (i32, i32),
        mask_loc: (i32, i32),
        dest_loc: (i32, i32),
        size: (i32, i32),
    ) {
        let mask_ptr = if let Some(mask) = mask {
            mask.as_ptr()
        } else {
            std::ptr::null_mut()
        };
        unsafe {
            ffi::pixman_image_composite32(
                operation.into(),
                src.as_ptr(),
                mask_ptr,
                self.as_ptr(),
                src_loc.0,
                src_loc.1,
                mask_loc.0,
                mask_loc.1,
                dest_loc.0,
                dest_loc.1,
                size.0,
                size.1,
            )
        }
    }

    /// Composite the specified triangles into this image
    pub fn composite_triangles(
        &mut self,
        operation: Operation,
        src: &ImageRef,
        mask_format: FormatCode,
        src_loc: (i32, i32),
        dest_loc: (i32, i32),
        tris: &[Triangle],
    ) {
        unsafe {
            ffi::pixman_composite_triangles(
                operation.into(),
                src.as_ptr(),
                self.as_ptr(),
                mask_format.into(),
                src_loc.0,
                src_loc.1,
                dest_loc.0,
                dest_loc.1,
                tris.len() as i32,
                tris.as_ptr() as *const ffi::pixman_triangle_t,
            );
        }
    }

    /// Composite the specified trapezoids into this image
    pub fn composite_trapezoids(
        &mut self,
        operation: Operation,
        src: &ImageRef,
        mask_format: FormatCode,
        src_loc: (i32, i32),
        dest_loc: (i32, i32),
        traps: &[Trapezoid],
    ) {
        unsafe {
            ffi::pixman_composite_trapezoids(
                operation.into(),
                src.as_ptr(),
                self.as_ptr(),
                mask_format.into(),
                src_loc.0,
                src_loc.1,
                dest_loc.0,
                dest_loc.1,
                traps.len() as i32,
                traps.as_ptr() as *const ffi::pixman_trapezoid_t,
            );
        }
    }

    /// Add the specified traps to this image
    pub fn add_traps(&mut self, offset: (i16, i16), traps: &[Trap]) {
        unsafe {
            ffi::pixman_add_traps(
                self.as_ptr(),
                offset.0,
                offset.1,
                traps.len() as i32,
                traps.as_ptr() as *const ffi::pixman_trap_t,
            );
        }
    }

    /// Add the specified trapezoids to this image
    pub fn add_trapezoids(&mut self, offset: (i16, i32), traps: &[Trapezoid]) {
        unsafe {
            ffi::pixman_add_trapezoids(
                self.as_ptr(),
                offset.0,
                offset.1,
                traps.len() as i32,
                traps.as_ptr() as *const _,
            );
        }
    }

    /// Add the specified triangles to this image
    pub fn add_triangles(&mut self, offset: (i32, i32), tris: &[Triangle]) {
        unsafe {
            ffi::pixman_add_triangles(
                self.as_ptr(),
                offset.0,
                offset.1,
                tris.len() as i32,
                tris.as_ptr() as *const _,
            );
        }
    }

    /// Compute the composite region for the specified src
    pub fn compute_composite_region(
        &self,
        src: &ImageRef,
        mask: Option<&ImageRef>,
        src_loc: (i16, i16),
        mask_loc: (i16, i16),
        dest_loc: (i16, i16),
        size: (u16, u16),
    ) -> Option<Region16> {
        let mask_ptr = if let Some(mask) = mask {
            mask.as_ptr()
        } else {
            std::ptr::null_mut()
        };

        let mut region = MaybeUninit::uninit();
        let res = unsafe {
            ffi::pixman_compute_composite_region(
                region.as_mut_ptr(),
                src.as_ptr(),
                mask_ptr,
                self.as_ptr(),
                src_loc.0,
                src_loc.1,
                mask_loc.0,
                mask_loc.1,
                dest_loc.0,
                dest_loc.1,
                size.0,
                size.1,
            )
        };
        if res == 1 {
            Some(Region16::from(unsafe { region.assume_init() }))
        } else {
            None
        }
    }

    /// Rasterize the specified edges
    pub fn rasterize_edges(&mut self, l: Edge, r: Edge, t: impl Into<Fixed>, b: impl Into<Fixed>) {
        unsafe {
            ffi::pixman_rasterize_edges(
                self.as_ptr(),
                l.as_ptr() as *mut _,
                r.as_ptr() as *mut _,
                t.into().into_raw(),
                b.into().into_raw(),
            )
        }
    }

    /// Rasterize the specified trapezoids
    pub fn rasterize_trapezoid(&mut self, trap: Trapezoid, offset: (i32, i32)) {
        unsafe { ffi::pixman_rasterize_trapezoid(self.as_ptr(), trap.as_ptr(), offset.0, offset.1) }
    }
}

impl Image<'_, '_> {
    /// Initialize the image from a raw pointer
    ///
    /// # Safety
    ///
    /// The pointer is expected to be valid and have a ref-count of at least one.
    /// Ownership of the pointer is transferred and unref will be called on drop.
    ///
    /// Any other references to the `pixman_image_t` must not be mutated while this
    /// `Image` exists, including changes to the reference count.
    pub unsafe fn from_ptr(ptr: *mut ffi::pixman_image_t) -> Self {
        Self {
            image: unsafe { ImageRef::from_ptr(ptr) },
            _bits: PhantomData,
            _alpha: PhantomData,
        }
    }
}
