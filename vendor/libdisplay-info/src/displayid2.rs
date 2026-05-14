//! Low-level API for VESA Display Identification Data (DisplayID) version 2.
//!
//! The library implements DisplayID version 2.1, available at:
//! <https://vesa.org/vesa-standards/>
use std::marker::PhantomData;

use libdisplay_info_derive::FFIFrom;

use crate::{edid::ExtensionRef, ffi};

pub struct DisplayId2<'ext> {
    display_id: *const ffi::displayid2::di_displayid2,
    phantom: PhantomData<&'ext ()>,
}

impl<'ext> DisplayId2<'ext> {
    /// Get a DisplayID v2 extension block
    ///
    /// Returns `None` if the extension block tag is not [DisplayId](crate::edid::ExtensionTag::DisplayId) or if
    /// the block does not contain a version 2 DisplayID blob.
    pub fn from_extension(extensions: &'ext ExtensionRef) -> Option<DisplayId2<'ext>> {
        let display_id = unsafe { ffi::edid::di_edid_ext_get_displayid2(extensions.as_ptr()) };

        if display_id.is_null() {
            None
        } else {
            Some(Self {
                display_id: display_id as *const ffi::displayid2::di_displayid2,
                phantom: PhantomData,
            })
        }
    }

    /// Get the DisplayID v2 revision.
    pub fn revision(&self) -> i32 {
        unsafe { ffi::displayid2::di_displayid2_get_revision(self.display_id) }
    }

    /// Get the DisplayID v2 product primary use case.
    pub fn product_primary_use_case(&self) -> PrimaryUseCase {
        PrimaryUseCase::from(unsafe {
            ffi::displayid2::di_displayid2_get_product_primary_use_case(self.display_id)
        })
    }
}

/// Product primary use case identifier, defined in table 2-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid2::di_displayid2_product_primary_use_case)]
#[repr(u32)]
pub enum PrimaryUseCase {
    Extension = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_EXTENSION,
    Test = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_TEST,
    Generic = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_GENERIC,
    TV = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_TV,
    DesktopProductivity = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_DESKTOP_PRODUCTIVITY,
    DesktopGaming = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_DESKTOP_GAMING,
    Presentation = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_PRESENTATION,
    HMDVR = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_HMD_VR,
    HMDAR = ffi::displayid2::di_displayid2_product_primary_use_case_DI_DISPLAYID2_PRODUCT_PRIMARY_USE_CASE_HMD_AR,
}
