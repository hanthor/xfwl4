use paste::paste;

use crate::{Box16, Box32};

/// Describes overlap of a region with a rectangle
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Overlap {
    /// No intersection
    Out,
    /// Region contains a rectangle
    In,
    /// Partial intersection
    Part,
}

macro_rules! impl_region {
    ($(#[$attr:meta])* $name:ident, ffi => $ffi:path, impl => $impl:ident, box_t => $box_t:path, loc_t => $loc_t:path, size_t => $size_t:path$(,)?) => {
        $(#[$attr])*
        pub struct $name($ffi);

        impl $name {
            /// Initialize a region from the provided values
            pub fn init_rect(x: $loc_t, y: $loc_t, width: $size_t, height: $size_t) -> Self {
                let mut region = std::mem::MaybeUninit::uninit();
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _init_rect>](
                            region.as_mut_ptr(),
                            x as std::ffi::c_int,
                            y as std::ffi::c_int,
                            width as std::ffi::c_uint,
                            height as std::ffi::c_uint,
                        );
                    }
                }
                Self(unsafe { region.assume_init() })
            }

            /// Initialize the region from the provided boxes
            pub fn init_rects(boxes: &[$box_t]) -> Self {
                let mut region = std::mem::MaybeUninit::uninit();
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _init_rects>](
                            region.as_mut_ptr(),
                            boxes.as_ptr(),
                            boxes.len() as i32,
                        );
                    }
                }
                Self(unsafe { region.assume_init() })
            }

            /// Initialize the region from the provided extents
            pub fn init_with_extents(extents: &[$box_t]) -> Self {
                let mut region = std::mem::MaybeUninit::uninit();
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _init_with_extents>](
                            region.as_mut_ptr(),
                            extents.as_ptr(),
                        )
                    }
                }
                Self(unsafe { region.assume_init() })
            }

            /// Intersect the region with another region
            pub fn intersect(&self, other: &$name) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste!($crate::ffi::[<$impl _intersect>](&mut region.0, &self.0, &other.0));
                }
                region
            }

            /// Intersect the region with a rect
            pub fn intersect_rect(&self, x: $loc_t, y: $loc_t, width: $size_t, height: $size_t) -> Self {
                let mut dest = $name::default();
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _intersect_rect>](
                            &mut dest.0,
                            &self.0,
                            x as std::ffi::c_int,
                            y as std::ffi::c_int,
                            width as std::ffi::c_uint,
                            height as std::ffi::c_uint,
                        );
                    }
                }
                dest
            }

            /// Take a region and a box and return a region that is everything
            /// in the box but not in the region. The careful reader will note
            /// that this is the same as subtracting the region from the box...
            pub fn inverse(&self, bbox: $box_t) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste!($crate::ffi::[<$impl _inverse>](&mut region.0, &self.0, &bbox));
                }
                region
            }

            /// Return the number of rects in this region
            pub fn n_rects(&self) -> usize {
                unsafe { paste!($crate::ffi::[<$impl _n_rects>](&self.0) as usize) }
            }

            /// Whether this region is empty
            pub fn is_non_empty(&self) -> bool {
                unsafe { paste!($crate::ffi::[<$impl _not_empty>](&self.0) == 1) }
            }

            /// Returns the rectangles in this region
            pub fn rectangles(&self) -> &[$box_t] {
                let mut n_rects = 0;

                let rectangles = unsafe {
                    let rects = paste!($crate::ffi::[<$impl _rectangles>](&self.0, &mut n_rects));
                    std::slice::from_raw_parts(rects, n_rects as usize)
                };

                rectangles
            }

            /// Reset this region to the provided box
            pub fn reset(&mut self, box_: $box_t) {
                unsafe {
                    paste!($crate::ffi::[<$impl _reset>](&mut self.0, &box_));
                }
            }

            /// Run a selfcheck on the region
            pub fn selfcheck(&mut self) -> bool {
                unsafe { paste!($crate::ffi::[<$impl _selfcheck>](&mut self.0) == 1) }
            }

            /// Subtract reg_s from reg_m and leave the result in reg_d.
            /// S stands for subtrahend, M for minuend and D for difference.
            pub fn subtract(&self, other: &$name) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste!($crate::ffi::[<$impl _subtract>](&mut region.0, &self.0, &other.0));
                }
                region
            }

            /// Translate this region by the specified amount
            pub fn translate(&mut self, x: $loc_t, y: $loc_t) {
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _translate>](
                            &mut self.0,
                            x as std::ffi::c_int,
                            y as std::ffi::c_int,)
                    }
                };
            }

            /// Create the union between this region and another region
            pub fn union(&self, other: &$name) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste!($crate::ffi::[<$impl _union>](&mut region.0, &self.0, &other.0));
                }
                region
            }

            /// Create the union between this region and the provided rect
            pub fn union_rect(&self, x: $loc_t, y: $loc_t, width: $size_t, height: $size_t) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste! {
                        $crate::ffi::[<$impl _union_rect>](
                            &mut region.0,
                            &self.0,
                            x as std::ffi::c_int,
                            y as std::ffi::c_int,
                            width as std::ffi::c_uint,
                            height as std::ffi::c_uint,
                        );
                    }
                }
                region
            }

            /// Clear this region
            pub fn clear(&mut self) {
                unsafe {
                    paste!($crate::ffi::[<$impl _clear>](&mut self.0));
                }
            }

            /// Whether this region contains the provided point
            pub fn contains_point(&self, x: $loc_t, y: $loc_t) -> Option<$box_t> {
                let mut r#box = std::mem::MaybeUninit::uninit();
                let contains_point = unsafe {
                    paste! {
                        $crate::ffi::[<$impl _contains_point>](
                            &self.0,
                            x as std::ffi::c_int,
                            y as std::ffi::c_int,
                            r#box.as_mut_ptr(),
                        ) == 1
                    }
                };

                if contains_point {
                    Some(unsafe { r#box.assume_init() })
                } else {
                    None
                }
            }

            /// Whether this region contains the provided rectangle
            pub fn contains_rectangle(&self, rect: $box_t) -> Overlap {
                let overlap =
                    unsafe { paste!($crate::ffi::[<$impl _contains_rectangle>](&self.0, &rect)) };
                match overlap {
                    $crate::ffi::pixman_region_overlap_t_PIXMAN_REGION_OUT => Overlap::Out,
                    $crate::ffi::pixman_region_overlap_t_PIXMAN_REGION_IN => Overlap::In,
                    $crate::ffi::pixman_region_overlap_t_PIXMAN_REGION_PART => Overlap::Part,
                    _ => unreachable!(),
                }
            }

            #[inline]
            pub(crate) fn as_ptr(&self) -> *const $ffi {
                &self.0
            }
        }

        impl Default for $name {
            #[inline]
            fn default() -> Self {
                let mut region = std::mem::MaybeUninit::uninit();
                unsafe {
                    paste!($crate::ffi::[<$impl _init>](region.as_mut_ptr()));
                }
                Self(unsafe { region.assume_init() })
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

        impl Clone for $name {
            #[inline]
            fn clone(&self) -> Self {
                let mut region = $name::default();
                unsafe {
                    paste!($crate::ffi::[<$impl _copy>](&mut region.0, &self.0));
                }
                region
            }
        }

        impl PartialEq for $name {
            #[inline]
            fn eq(&self, other: &Self) -> bool {
                unsafe { paste!($crate::ffi::[<$impl _equal>](&self.0, &other.0) == 1) }
            }
        }
        impl Eq for $name {}

        impl Drop for $name {
            fn drop(&mut self) {
                unsafe {
                    paste!($crate::ffi::[<$impl _fini>](&mut self.0));
                }
            }
        }
    };
}

impl_region! {
    /// 16bit region
    #[derive(Debug)]
    Region16,
    ffi => crate::ffi::pixman_region16_t,
    impl => pixman_region,
    box_t => Box16,
    loc_t => i16,
    size_t => u16,
}

impl_region! {
    /// 32bit region
    #[derive(Debug)]
    Region32,
    ffi => crate::ffi::pixman_region32_t,
    impl => pixman_region32,
    box_t => Box32,
    loc_t => i32,
    size_t => u32,
}
