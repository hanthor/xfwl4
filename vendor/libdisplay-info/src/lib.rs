#![cfg_attr(docsrs, feature(doc_auto_cfg))]

use std::{
    ffi::{c_char, CStr},
    marker::PhantomData,
};

#[cfg(not(docsrs))]
mod feature_check {
    #[cfg(all(feature = "v0_2", feature = "v0_3"))]
    compile_error!("Either feature \"v0_2\" or \"v0_3\" must be enabled, but not both.");
}

pub use libdisplay_info_sys as ffi;

pub mod cta;
pub mod cvt;
pub mod displayid;
#[cfg(feature = "v0_3")]
pub mod displayid2;
pub mod dmt;
pub mod edid;
pub mod gtf;
pub mod info;

/// Get the [`String`] from an owned ffi ptr
///
/// This will automatically free the ptr
fn string_from_ffi_ptr(ptr: *const c_char) -> Option<String> {
    if ptr.is_null() {
        None
    } else {
        Some(unsafe { CStr::from_ptr(ptr).to_string_lossy().into_owned() })
    }
}

/// Get the [`String`] from an owned ffi ptr
///
/// This will automatically free the ptr
///
/// `None` is returned for NULL ptr
fn string_from_owned_ffi_ptr(ptr: *mut c_char) -> Option<String> {
    let res = string_from_ffi_ptr(ptr);
    if res.is_some() {
        unsafe {
            libc::free(ptr as *mut _);
        }
    }
    res
}

struct FFIIter<'a, T, F> {
    ptr: *const *const F,
    t: PhantomData<T>,
    phantom: PhantomData<&'a ()>,
}

impl<T, F> FFIIter<'_, T, F> {
    fn new(ptr: *const *const F) -> Self {
        Self {
            ptr,
            t: PhantomData,
            phantom: PhantomData,
        }
    }
}

impl<T, F> Iterator for FFIIter<'_, T, F>
where
    T: From<F>,
    F: Copy,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.ptr.is_null() || unsafe { *self.ptr }.is_null() {
            None
        } else {
            let item = T::from(unsafe { *(*self.ptr) });
            self.ptr = self.ptr.wrapping_add(1);
            Some(item)
        }
    }
}
