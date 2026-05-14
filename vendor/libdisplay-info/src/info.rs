//! High-level API.
use crate::{edid::Edid, ffi, string_from_owned_ffi_ptr};

#[cfg(any(feature = "v0_2", feature = "v0_3"))]
use libdisplay_info_derive::FFIFrom;

/// Display HDR static metadata
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::info::di_hdr_static_metadata)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub struct HdrStaticMetadata {
    pub desired_content_max_luminance: f32,
    pub desired_content_max_frame_avg_luminance: f32,
    pub desired_content_min_luminance: f32,
    pub type1: bool,
    pub traditional_sdr: bool,
    pub traditional_hdr: bool,
    pub pq: bool,
    pub hlg: bool,
}

/// CIE 1931 2-degree observer chromaticity coordinates
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::info::di_chromaticity_cie1931)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub struct ChromaticityCie1931 {
    pub x: f32,
    pub y: f32,
}

/// Display color primaries and default white point
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::info::di_color_primaries)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub struct ColorPrimaries {
    pub has_primaries: bool,
    pub has_default_white_point: bool,
    pub primary: [ChromaticityCie1931; 3usize],
    pub default_white: ChromaticityCie1931,
}

/// Additional signal colorimetry encodings supported by the display
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::info::di_supported_signal_colorimetry)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub struct SupportedSignalColorimetry {
    pub bt2020_cycc: bool,
    pub bt2020_ycc: bool,
    pub bt2020_rgb: bool,
    pub st2113_rgb: bool,
    pub ictcp: bool,
}

/// Information about a display device.
///
/// This includes at least one EDID or DisplayID blob.
///
/// Use [`Info::parse_edid`](Info::parse_edid) to create a [`Info`] from an EDID blob.
/// DisplayID blobs are not yet supported.
#[derive(Debug)]
pub struct Info(*mut ffi::info::di_info);

/// Parsing the EDID blob failed
#[derive(Debug, thiserror::Error)]
#[error("Parsing the EDID blob failed")]
pub struct ParseFailed;

impl Info {
    /// Parse an EDID blob.
    pub fn parse_edid(data: &[u8]) -> Result<Self, ParseFailed> {
        let info = unsafe {
            ffi::info::di_info_parse_edid(data.as_ptr() as *const std::ffi::c_void, data.len())
        };

        if info.is_null() {
            return Err(ParseFailed);
        }

        Ok(Self(info))
    }

    /// Get the failure messages for this blob.
    ///
    /// `None` is returned if the blob conforms to the relevant specifications.
    pub fn failure_msg(&self) -> Option<&std::ffi::CStr> {
        let failure_msg = unsafe { ffi::info::di_info_get_failure_msg(self.0) };

        if failure_msg.is_null() {
            None
        } else {
            Some(unsafe { std::ffi::CStr::from_ptr(failure_msg) })
        }
    }

    /// Returns the EDID the display device information was constructed with.
    ///
    /// The returned [`Edid`] can be used to query low-level EDID information,
    /// see [`edid`](crate::edid) module level docs. Users should prefer the high-level API if
    /// possible.
    ///
    /// `None` is returned if the [`Info`] doesn't contain an EDID.
    pub fn edid(&self) -> Option<Edid<'_>> {
        Edid::from_ptr(unsafe { ffi::info::di_info_get_edid(self.0) as *const ffi::edid::di_edid })
    }

    /// Get the make of the display device.
    ///
    /// This is the manufacturer name, either company name or PNP ID.
    /// This string is informational and not meant to be used in programmatic
    /// decisions, configuration keys, etc.
    ///
    /// The string is in UTF-8 and may contain any characters except ASCII control
    /// codes.
    ///
    /// `None` is returned if the information is not available.
    pub fn make(&self) -> Option<String> {
        string_from_owned_ffi_ptr(unsafe { ffi::info::di_info_get_make(self.0) })
    }

    /// Get the model of the display device.
    ///
    /// This is the product name/model string or product number.
    /// This string is informational and not meant to be used in programmatic
    /// decisions, configuration keys, etc.
    ///
    /// The string is in UTF-8 and may contain any characters except ASCII control
    /// codes.
    ///
    /// `None` is returned if the information is not available.
    pub fn model(&self) -> Option<String> {
        string_from_owned_ffi_ptr(unsafe { ffi::info::di_info_get_model(self.0) })
    }

    /// Get the serial of the display device.
    ///
    /// This is the product serial string or the serial number.
    /// This string is informational and not meant to be used in programmatic
    /// decisions, configuration keys, etc.
    ///
    /// The string is in UTF-8 and may contain any characters except ASCII control
    /// codes.
    ///
    /// `None` is returned if the information is not available.
    pub fn serial(&self) -> Option<String> {
        string_from_owned_ffi_ptr(unsafe { ffi::info::di_info_get_serial(self.0) })
    }

    /// Get HDR static metadata support information as defined in ANSI/CTA-861-H
    /// as HDR Static Metadata Data Block.
    ///  
    /// When HDR static metadata does not exist,
    /// all luminance fields are zero and only traditional_sdr is flagged as
    /// supported.
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn hdr_static_metadata(&self) -> HdrStaticMetadata {
        // SAFETY: The returned pointer is owned by the struct di_info passed in. It remains
        // valid only as long as the di_info exists, and must not be freed by the
        // caller.
        //
        // This function does not return NULL.
        HdrStaticMetadata::from(unsafe { *ffi::info::di_info_get_hdr_static_metadata(self.0) })
    }

    /// Get display color primaries and default white point
    ///
    /// Get the parameters of the default RGB colorimetry mode which is always
    /// supported. Primaries for monochrome displays might be all zeroes.
    ///
    /// These primaries might not be display's physical primaries, but only the
    /// primaries of the default RGB colorimetry signal when using IT Video Format
    /// (ANSI/CTA-861-H, Section 5).
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn default_color_primaries(&self) -> ColorPrimaries {
        // SAFETY: The returned pointer is owned by the struct di_info passed in. It remains
        // valid only as long as the di_info exists, and must not be freed by the
        // caller.
        //
        // This function does not return NULL.
        ColorPrimaries::from(unsafe { *ffi::info::di_info_get_default_color_primaries(self.0) })
    }

    /// Get signal colorimetry encodings supported by the display
    ///
    /// These signal colorimetry encodings are supported in addition to the
    /// display's default RGB colorimetry. When you wish to use one of the additional
    /// encodings, they need to be explicitly enabled in the video signal. How to
    /// do that is specific to the signalling used, e.g. HDMI.
    ///
    /// Signal colorimetry encoding provides the color space that the signal is
    /// encoded for. This includes primary and white point chromaticities, and the
    /// YCbCr-RGB conversion if necessary. Also the transfer function is implied
    /// unless explicitly set otherwise, e.g. with HDR static metadata.
    /// See ANSI/CTA-861-H for details.
    ///
    /// The signal color volume can be considerably larger than the physically
    /// displayable color volume.
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn supported_signal_colorimetry(&self) -> SupportedSignalColorimetry {
        // SAFETY: The returned pointer is owned by the struct di_info passed in. It remains
        // valid only as long as the di_info exists, and must not be freed by the
        // caller.
        //
        // This function does not return NULL.
        SupportedSignalColorimetry::from(unsafe {
            *ffi::info::di_info_get_supported_signal_colorimetry(self.0)
        })
    }

    /// Get display default transfer characteristic exponent (gamma)
    ///
    /// This should be the display gamma value when the display has been reset to
    /// its factory defaults, and it is driven with the default RGB colorimetry.
    ///
    /// Returns `None` when unknown.
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn default_gamma(&self) -> Option<f32> {
        // SAFETY: The value is zero when unknown.
        let default_gamma = unsafe { ffi::info::di_info_get_default_gamma(self.0) };
        if default_gamma == 0f32 {
            None
        } else {
            Some(default_gamma)
        }
    }
}

impl Drop for Info {
    fn drop(&mut self) {
        unsafe {
            ffi::info::di_info_destroy(self.0);
        }
    }
}
