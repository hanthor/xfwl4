//! Low-level API for Extended Display Identification Data (EDID).
//!
//! EDID 1.4 is defined in VESA Enhanced Extended Display Identification Data
//! Standard release A revision 2.
use std::marker::PhantomData;

use libdisplay_info_derive::FFIFrom;

use crate::{dmt, ffi, string_from_ffi_ptr, FFIIter};

/// EDID data structure.
#[derive(Debug)]
pub struct Edid<'info> {
    edid: *const ffi::edid::di_edid,
    phantom: PhantomData<&'info ()>,
}

impl Edid<'_> {
    /// Get the EDID version.
    pub fn version(&self) -> i32 {
        unsafe { ffi::edid::di_edid_get_version(self.edid) }
    }

    /// Get the EDID revision.
    pub fn revision(&self) -> i32 {
        unsafe { ffi::edid::di_edid_get_revision(self.edid) }
    }

    /// Get the EDID vendor product
    pub fn vendor_product(&self) -> VendorProduct {
        VendorProduct::from(unsafe { *ffi::edid::di_edid_get_vendor_product(self.edid) })
    }

    /// Get the analog video input basic information.
    ///
    /// Returns `None` if this isn't an analog display.
    pub fn video_input_analog(&self) -> Option<VideoInputAnalog> {
        VideoInputAnalog::from_ptr(unsafe { ffi::edid::di_edid_get_video_input_analog(self.edid) })
    }

    /// Get the digital video input basic information.
    ///
    /// Returns `None` if this isn't a digital display.
    pub fn video_input_digital(&self) -> Option<VideoInputDigital> {
        VideoInputDigital::from_ptr(unsafe {
            ffi::edid::di_edid_get_video_input_digital(self.edid)
        })
    }

    /// Get the screen size.
    pub fn screen_size(&self) -> ScreenSize {
        ScreenSize::from_ptr(unsafe { ffi::edid::di_edid_get_screen_size(self.edid) })
            .expect("expected non null ptr")
    }

    /// Get the display transfer characteristics from the basic EDID parameters, also
    /// known as \"gamma\".
    ///
    /// Returns `None` if unset (ie, stored in an extension block).
    pub fn basic_gamma(&self) -> Option<f32> {
        let basic_gamma = unsafe { ffi::edid::di_edid_get_basic_gamma(self.edid) };

        if basic_gamma == 0f32 {
            None
        } else {
            Some(basic_gamma)
        }
    }

    /// Get the set of supported legacy DPMS states.
    pub fn dpms(&self) -> Dpms {
        Dpms::from_ptr(unsafe { ffi::edid::di_edid_get_dpms(self.edid) })
            .expect("expected non null ptr")
    }

    /// Get the display color type.
    ///
    /// For digital displays using EDID 1.4 and later, [`DisplayColorType::Undefined`]
    /// is always returned.
    pub fn display_color_type(&self) -> DisplayColorType {
        DisplayColorType::from(unsafe { ffi::edid::di_edid_get_display_color_type(self.edid) })
    }

    /// Get the set of supported color encoding formats.
    ///
    /// Returns `None` if the display is analog or if the color encoding formats are
    /// not specified.
    pub fn color_encoding_formats(&self) -> Option<ColorEncodingFormats> {
        ColorEncodingFormats::from_ptr(unsafe {
            ffi::edid::di_edid_get_color_encoding_formats(self.edid)
        })
    }

    /// Get the set of miscellaneous basic features.
    pub fn misc_features(&self) -> MiscFeatures {
        MiscFeatures::from_ptr(unsafe { ffi::edid::di_edid_get_misc_features(self.edid) })
            .expect("expected non null ptr")
    }

    /// Get chromaticity coordinates.
    pub fn chromaticity_coords(&self) -> ChromaticityCoords {
        ChromaticityCoords::from_ptr(unsafe {
            ffi::edid::di_edid_get_chromaticity_coords(self.edid)
        })
        .expect("expected non null ptr")
    }

    /// Get established timings I and II.
    pub fn established_timings(&self) -> EstablishedTimings {
        EstablishedTimings::from_ptr(unsafe {
            ffi::edid::di_edid_get_established_timings_i_ii(self.edid)
        })
        .expect("expected non null ptr")
    }

    /// Get a list of EDID standard timings.
    pub fn standard_timings(&self) -> &[StandardTimingRef] {
        let standard_timings = unsafe { ffi::edid::di_edid_get_standard_timings(self.edid) };

        let mut len = 0;
        while !unsafe { *standard_timings.offset(len) }.is_null() {
            len += 1;
        }

        unsafe {
            std::slice::from_raw_parts(standard_timings as *const StandardTimingRef, len as usize)
        }
    }

    // Get a list of EDID detailed timing definitions.
    pub fn detailed_timing_defs(&self) -> impl Iterator<Item = DetailedTimingDef> {
        FFIIter::new(unsafe { ffi::edid::di_edid_get_detailed_timing_defs(self.edid) })
    }

    /// Get a list of EDID display descriptors.
    pub fn display_descriptors(&self) -> &[DisplayDescriptorRef] {
        let display_descriptors = unsafe { ffi::edid::di_edid_get_display_descriptors(self.edid) };

        let mut len = 0;
        while !unsafe { *display_descriptors.offset(len) }.is_null() {
            len += 1;
        }

        unsafe {
            std::slice::from_raw_parts(
                display_descriptors as *const DisplayDescriptorRef,
                len as usize,
            )
        }
    }

    /// Get a list of EDID extensions.
    pub fn extensions(&self) -> &[ExtensionRef] {
        let extensions = unsafe { ffi::edid::di_edid_get_extensions(self.edid) };

        let mut len = 0;
        while !unsafe { *extensions.offset(len) }.is_null() {
            len += 1;
        }

        unsafe { std::slice::from_raw_parts(extensions as *const ExtensionRef, len as usize) }
    }

    pub(crate) fn from_ptr(ptr: *const ffi::edid::di_edid) -> Option<Self> {
        if ptr.is_null() {
            None
        } else {
            Some(Self {
                edid: ptr,
                phantom: PhantomData,
            })
        }
    }
}

/// EDID vendor & product identification.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_vendor_product)]
pub struct VendorProduct {
    #[cast_as(u8)]
    pub manufacturer: [char; 3usize],
    pub product: u16,
    #[optional(0u32)]
    pub serial: Option<u32>,
    pub manufacture_week: i32,
    pub manufacture_year: i32,
    #[optional(0i32)]
    pub model_year: Option<i32>,
}

/// EDID analog signal level standard.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_video_input_analog_signal_level_std)]
#[repr(u32)]
pub enum VideoInputAnalogSignalLevelStandard {
    Level0 = ffi::edid::di_edid_video_input_analog_signal_level_std_DI_EDID_VIDEO_INPUT_ANALOG_SIGNAL_LEVEL_0,
    Level1 = ffi::edid::di_edid_video_input_analog_signal_level_std_DI_EDID_VIDEO_INPUT_ANALOG_SIGNAL_LEVEL_1,
    Level2 = ffi::edid::di_edid_video_input_analog_signal_level_std_DI_EDID_VIDEO_INPUT_ANALOG_SIGNAL_LEVEL_2,
    Level3 = ffi::edid::di_edid_video_input_analog_signal_level_std_DI_EDID_VIDEO_INPUT_ANALOG_SIGNAL_LEVEL_3,
}

/// EDID analog video setup.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_video_input_analog_video_setup)]
#[repr(u32)]
pub enum VideoInputAnalogVideoSetup {
    BlankLevelEqBlack = ffi::edid::di_edid_video_input_analog_video_setup_DI_EDID_VIDEO_INPUT_ANALOG_BLANK_LEVEL_EQ_BLACK,
    BlankToBlackSetupPedestal = ffi::edid::di_edid_video_input_analog_video_setup_DI_EDID_VIDEO_INPUT_ANALOG_BLANK_TO_BLACK_SETUP_PEDESTAL,
}

/// EDID analog video input basic information, defined in section 3.6.1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_video_input_analog)]
pub struct VideoInputAnalog {
    pub signal_level_std: VideoInputAnalogSignalLevelStandard,
    pub video_setup: VideoInputAnalogVideoSetup,
    pub sync_separate: bool,
    pub sync_composite: bool,
    pub sync_on_green: bool,
    pub sync_serrations: bool,
}

/// Digital video input interface standard.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_video_input_digital_interface)]
#[repr(u32)]
pub enum VideoInputDigitalInterface {
    Undefined =
        ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_UNDEFINED,
    DVI = ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_DVI,
    HDMIA = ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_HDMI_A,
    HDMIB = ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_HDMI_B,
    MDDI = ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_MDDI,
    DisplayPort =
        ffi::edid::di_edid_video_input_digital_interface_DI_EDID_VIDEO_INPUT_DIGITAL_DISPLAYPORT,
}

/// EDID digital video input basic information, defined in section 3.6.1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_video_input_digital)]
pub struct VideoInputDigital {
    pub dfp1: bool,
    #[optional(0i32)]
    pub color_bit_depth: Option<i32>,
    pub interface: VideoInputDigitalInterface,
}

/// Screen size
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_screen_size)]
pub struct ScreenSize {
    #[optional(0i32)]
    pub width_cm: Option<i32>,
    #[optional(0i32)]
    pub height_cm: Option<i32>,
    #[optional(0f32)]
    pub landscape_aspect_ratio: Option<f32>,
    #[optional(0f32)]
    pub portait_aspect_ratio: Option<f32>,
}

/// Supported legacy Display Power Management Signaling (DPMS) states, defined in
/// section 3.6.4.
///
/// Display Power Management (DPM) compliant displays only support \"off\"."]
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_dpms)]
pub struct Dpms {
    pub standby: bool,
    pub suspend: bool,
    pub off: bool,
}

/// Display color type.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_color_type)]
#[repr(u32)]
pub enum DisplayColorType {
    Monochrome = ffi::edid::di_edid_display_color_type_DI_EDID_DISPLAY_COLOR_MONOCHROME,
    RGB = ffi::edid::di_edid_display_color_type_DI_EDID_DISPLAY_COLOR_RGB,
    NonRGB = ffi::edid::di_edid_display_color_type_DI_EDID_DISPLAY_COLOR_NON_RGB,
    Undefined = ffi::edid::di_edid_display_color_type_DI_EDID_DISPLAY_COLOR_UNDEFINED,
}

/// Basic color encoding formats, defined in section 3.6.4."
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_color_encoding_formats)]
pub struct ColorEncodingFormats {
    pub rgb444: bool,
    pub ycrcb444: bool,
    pub ycrcb422: bool,
}

/// Miscellaneous basic features, defined in section 3.6.4.
///
/// Note, the enum values don't match the specification.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_misc_features)]
pub struct MiscFeatures {
    /// First detailed timing is the preferred timing.
    ///
    /// Always set for EDID 1.4 and later.
    pub has_preferred_timing: bool,
    /// GTF using the default parameters is supported.
    ///
    /// Never set for EDID 1.4 and later.
    pub default_gtf: bool,
    /// sRGB standard default color space is primary color space.
    pub srgb_is_primary: bool,
    /// Preferred timing mode includes native pixel format and rate.
    ///
    /// Never set for EDID 1.3 and earlier.
    pub preferred_timing_is_native: bool,
    /// GTF or CVT generated timings within the display's range limits are
    /// accepted.
    ///
    /// Never set for EDID 1.3 and earlier.
    pub continuous_freq: bool,
}

/// EDID display chromaticity coordinates, defined in section 3.7.
///
/// The values are accurate to the thousandth place. The red, green and blue
/// values are zero for monochrome displays.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_chromaticity_coords)]
pub struct ChromaticityCoords {
    pub red_x: f32,
    pub red_y: f32,
    pub green_x: f32,
    pub green_y: f32,
    pub blue_x: f32,
    pub blue_y: f32,
    pub white_x: f32,
    pub white_y: f32,
}

/// Established timings I and II, defined in section 3.8.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_established_timings_i_ii)]
pub struct EstablishedTimings {
    pub has_720x400_70hz: bool,
    pub has_720x400_88hz: bool,
    pub has_640x480_60hz: bool,
    pub has_640x480_67hz: bool,
    pub has_640x480_72hz: bool,
    pub has_640x480_75hz: bool,
    pub has_800x600_56hz: bool,
    pub has_800x600_60hz: bool,
    pub has_800x600_72hz: bool,
    pub has_800x600_75hz: bool,
    pub has_832x624_75hz: bool,
    pub has_1024x768_87hz_interlaced: bool,
    pub has_1024x768_60hz: bool,
    pub has_1024x768_70hz: bool,
    pub has_1024x768_75hz: bool,
    pub has_1280x1024_75hz: bool,
    pub has_1152x870_75hz: bool,
}

/// Aspect ratio for an EDID standard timing.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_standard_timing_aspect_ratio)]
#[repr(u32)]
pub enum StandardTimingAspectRatio {
    _16_10 = ffi::edid::di_edid_standard_timing_aspect_ratio_DI_EDID_STANDARD_TIMING_16_10,
    _4_3 = ffi::edid::di_edid_standard_timing_aspect_ratio_DI_EDID_STANDARD_TIMING_4_3,
    _5_4 = ffi::edid::di_edid_standard_timing_aspect_ratio_DI_EDID_STANDARD_TIMING_5_4,
    _16_9 = ffi::edid::di_edid_standard_timing_aspect_ratio_DI_EDID_STANDARD_TIMING_16_9,
}

/// EDID standard timing, defined in section 3.9.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_standard_timing)]
#[wrap]
pub struct StandardTiming {
    pub horiz_video: i32,
    pub aspect_ratio: StandardTimingAspectRatio,
    pub refresh_rate_hz: i32,
}

impl StandardTimingRef {
    /// Get the vertical addressable line count of an EDID standard timing.
    pub fn vert_video(&self) -> i32 {
        unsafe { ffi::edid::di_edid_standard_timing_get_vert_video(self.0) }
    }

    /// Get the VESA Display Monitor Timing (DMT), if any.
    ///
    /// `None` is returned if the standard timing doesn't have a DMT.
    pub fn dmt(&self) -> Option<crate::dmt::Timing> {
        let dmt = unsafe { ffi::edid::di_edid_standard_timing_get_dmt(self.0) };

        if dmt.is_null() {
            None
        } else {
            Some(crate::dmt::Timing::from(unsafe {
                *(dmt as *const ffi::dmt::di_dmt_timing)
            }))
        }
    }
}

/// Stereo viewing support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_def_stereo)]
#[repr(u32)]
pub enum DetailedTimingDefStereo {
    NONE = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_NONE,
    FieldSeqRight = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_FIELD_SEQ_RIGHT,
    FieldSeqLeft = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_FIELD_SEQ_LEFT,
    TwoWayInterleavedRight = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_2_WAY_INTERLEAVED_RIGHT,
    TwoWayInterleavedLeft = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_2_WAY_INTERLEAVED_LEFT,
    FourWayInterleaved = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_4_WAY_INTERLEAVED,
    SideBySideInterleaved = ffi::edid::di_edid_detailed_timing_def_stereo_DI_EDID_DETAILED_TIMING_DEF_STEREO_SIDE_BY_SIDE_INTERLEAVED,
}

/// Signal definitions for EDID detailed timings, defined in notes for table 3.22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_def_signal_type)]
#[repr(u32)]
pub enum DetailedTimingDefSignalType {
    AnalogComposite = ffi::edid::di_edid_detailed_timing_def_signal_type_DI_EDID_DETAILED_TIMING_DEF_SIGNAL_ANALOG_COMPOSITE,
    BipolarAnalogComposite = ffi::edid::di_edid_detailed_timing_def_signal_type_DI_EDID_DETAILED_TIMING_DEF_SIGNAL_BIPOLAR_ANALOG_COMPOSITE,
    DigitalComposite = ffi::edid::di_edid_detailed_timing_def_signal_type_DI_EDID_DETAILED_TIMING_DEF_SIGNAL_DIGITAL_COMPOSITE,
    DigitalSeparate = ffi::edid::di_edid_detailed_timing_def_signal_type_DI_EDID_DETAILED_TIMING_DEF_SIGNAL_DIGITAL_SEPARATE,
}

/// Digital separate sync polarity for EDID detailed timings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_def_sync_polarity)]
#[repr(u32)]
pub enum DetailedTimingDefSyncPolarity {
    Negative = ffi::edid::di_edid_detailed_timing_def_sync_polarity_DI_EDID_DETAILED_TIMING_DEF_SYNC_NEGATIVE,
    Positive = ffi::edid::di_edid_detailed_timing_def_sync_polarity_DI_EDID_DETAILED_TIMING_DEF_SYNC_POSITIVE,
}

/// Flags for ANALOG_COMPOSITE signals
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_analog_composite)]
pub struct DetailedTimingAnalogComposite {
    pub sync_serrations: bool,
    pub sync_on_green: bool,
}

// Flags for BIPOLAR_ANALOG_COMPOSITE signals
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_bipolar_analog_composite)]
pub struct DetailedTimingBipolarAnalogComposite {
    pub sync_serrations: bool,
    pub sync_on_green: bool,
}

// Flags for DIGITAL_COMPOSITE signals
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_digital_composite)]
pub struct DetailedTimingDigitalComposite {
    pub sync_serrations: bool,
    pub sync_horiz_polarity: DetailedTimingDefSyncPolarity,
}

/// Flags for DIGITAL_SEPARATE signals
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_digital_separate)]
pub struct DetailedTimingDigitalSeparate {
    pub sync_vert_polarity: DetailedTimingDefSyncPolarity,
    pub sync_horiz_polarity: DetailedTimingDefSyncPolarity,
}

/// EDID detailed timing definition, defined in section 3.10.2.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_detailed_timing_def)]
pub struct DetailedTimingDef {
    pub pixel_clock_hz: i32,
    pub horiz_video: i32,
    pub vert_video: i32,
    pub horiz_blank: i32,
    pub vert_blank: i32,
    pub horiz_front_porch: i32,
    pub vert_front_porch: i32,
    pub horiz_sync_pulse: i32,
    pub vert_sync_pulse: i32,
    pub horiz_image_mm: i32,
    pub vert_image_mm: i32,
    #[optional(0i32)]
    pub horiz_border: Option<i32>,
    #[optional(0i32)]
    pub vert_border: Option<i32>,
    pub interlaced: bool,
    pub stereo: DetailedTimingDefStereo,
    pub signal_type: DetailedTimingDefSignalType,
    #[ptr_deref]
    pub analog_composite: Option<DetailedTimingAnalogComposite>,
    #[ptr_deref]
    pub bipolar_analog_composite: Option<DetailedTimingBipolarAnalogComposite>,
    #[ptr_deref]
    pub digital_composite: Option<DetailedTimingDigitalComposite>,
    #[ptr_deref]
    pub digital_separate: Option<DetailedTimingDigitalSeparate>,
}

/// EDID display descriptor tag, defined in section 3.10.3.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_descriptor_tag)]
#[repr(u32)]
pub enum DisplayDescriptorTag {
    ProductSerial =
        ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_PRODUCT_SERIAL,
    DataString = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_DATA_STRING,
    RangeLimits = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_RANGE_LIMITS,
    ProductName = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_PRODUCT_NAME,
    ColorPoint = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_COLOR_POINT,
    StdTimingIds =
        ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_STD_TIMING_IDS,
    DcmData = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_DCM_DATA,
    CvtTimingCodes =
        ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_CVT_TIMING_CODES,
    EstablishedTimingsIII = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_ESTABLISHED_TIMINGS_III,
    Dummy = ffi::edid::di_edid_display_descriptor_tag_DI_EDID_DISPLAY_DESCRIPTOR_DUMMY,
}

// EDID display descriptor.
#[derive(Debug)]
pub struct DisplayDescriptorRef(*const ffi::edid::di_edid_display_descriptor);

impl DisplayDescriptorRef {
    /// Get the tag of an EDID display descriptor.
    pub fn tag(&self) -> DisplayDescriptorTag {
        DisplayDescriptorTag::from(unsafe { ffi::edid::di_edid_display_descriptor_get_tag(self.0) })
    }

    /// Get the contents of a product serial number, a data string, or a product name
    /// display descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't either
    /// DI_EDID_DISPLAY_DESCRIPTOR_PRODUCT_SERIAL_NUMBER,
    /// DI_EDID_DISPLAY_DESCRIPTOR_DATA_STRING or
    /// DI_EDID_DISPLAY_DESCRIPTOR_PRODUCT_NAME.
    pub fn string(&self) -> Option<String> {
        string_from_ffi_ptr(unsafe { ffi::edid::di_edid_display_descriptor_get_string(self.0) })
    }

    /// Get the contents of a display range limits descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_RANGE_LIMITS.
    pub fn range_limits(&self) -> Option<DisplayRangeLimits> {
        let display_range_limits =
            unsafe { ffi::edid::di_edid_display_descriptor_get_range_limits(self.0) };

        if display_range_limits.is_null() {
            None
        } else {
            Some(DisplayRangeLimits::from(unsafe { *display_range_limits }))
        }
    }

    /// Get a standard timing list from an EDID display descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_STD_TIMING_IDS.
    pub fn standard_timings(&self) -> Option<&[StandardTimingRef]> {
        let standard_timings =
            unsafe { ffi::edid::di_edid_display_descriptor_get_standard_timings(self.0) };

        if standard_timings.is_null() {
            None
        } else {
            let mut len = 0;
            while !unsafe { *standard_timings.offset(len) }.is_null() {
                len += 1;
            }

            Some(unsafe {
                std::slice::from_raw_parts(
                    standard_timings as *const StandardTimingRef,
                    len as usize,
                )
            })
        }
    }

    /// Get a color point list from an EDID display descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_COLOR_POINT.
    ///
    /// Upstream is not aware of any EDID blob containing Color Point Descriptors.
    /// If such a blob is found, please share it with upstream!
    pub fn color_points(&self) -> impl Iterator<Item = ColorPoint> {
        FFIIter::new(unsafe { ffi::edid::di_edid_display_descriptor_get_color_points(self.0) })
    }

    /// Get a list of established timings III from an EDID display descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_ESTABLISHED_TIMINGS_III.
    pub fn established_timings_iii(&self) -> impl Iterator<Item = dmt::Timing> {
        FFIIter::new(unsafe {
            ffi::edid::di_edid_display_descriptor_get_established_timings_iii(self.0)
                as *const *const ffi::dmt::di_dmt_timing
        })
    }

    /// Get the contents of a Display Color Management (DCM) Data descriptor.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_DCM_DATA.
    ///
    /// Upstream is not aware of any EDID blob containing DCM Data descriptors.
    /// If such a blob is found, please share it with upstream!
    pub fn color_management_data(&self) -> Option<ColorManagementData> {
        let cmd =
            unsafe { ffi::edid::di_edid_display_descriptor_get_color_management_data(self.0) };

        if cmd.is_null() {
            None
        } else {
            Some(ColorManagementData::from(unsafe { *cmd }))
        }
    }

    /// Get a list of CVT timing codes from an EDID display descriptor.
    ///
    /// The highest priority code comes first, the lowest priority code last.
    ///
    /// Returns `None` if the display descriptor tag isn't
    /// DI_EDID_DISPLAY_DESCRIPTOR_CVT_TIMING_CODES.
    pub fn cvt_timing_codes(&self) -> impl Iterator<Item = CvtTimingCode> {
        FFIIter::new(unsafe { ffi::edid::di_edid_display_descriptor_get_cvt_timing_codes(self.0) })
    }
}

/// EDID display range limits type.
///
/// The values do not match the EDID specification.
///
/// The CVT entry was introduced in EDID 1.4.
#[derive(Debug, Copy, Clone, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_range_limits_type)]
#[repr(u32)]
pub enum DisplayRangeLimitsType {
    Bare = ffi::edid::di_edid_display_range_limits_type_DI_EDID_DISPLAY_RANGE_LIMITS_BARE,
    DefaultGtf =
        ffi::edid::di_edid_display_range_limits_type_DI_EDID_DISPLAY_RANGE_LIMITS_DEFAULT_GTF,
    SecondaryGtf =
        ffi::edid::di_edid_display_range_limits_type_DI_EDID_DISPLAY_RANGE_LIMITS_SECONDARY_GTF,
    Cvt = ffi::edid::di_edid_display_range_limits_type_DI_EDID_DISPLAY_RANGE_LIMITS_CVT,
}

#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_range_limits_secondary_gtf)]
pub struct DisplayRangeLimitsSecondaryGtf {
    pub start_freq_hz: ::std::os::raw::c_int,
    pub c: f32,
    pub m: f32,
    pub k: f32,
    pub j: f32,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct CvtAspectRatio: u32 {
        const _4_3 = ffi::edid::di_edid_cvt_aspect_ratio_DI_EDID_CVT_ASPECT_RATIO_4_3;
        const _16_9 = ffi::edid::di_edid_cvt_aspect_ratio_DI_EDID_CVT_ASPECT_RATIO_16_9;
        const _16_10 = ffi::edid::di_edid_cvt_aspect_ratio_DI_EDID_CVT_ASPECT_RATIO_16_10;
        const _5_4 = ffi::edid::di_edid_cvt_aspect_ratio_DI_EDID_CVT_ASPECT_RATIO_5_4;
        const _15_9 = ffi::edid::di_edid_cvt_aspect_ratio_DI_EDID_CVT_ASPECT_RATIO_15_9;
        const _ = !0;
    }
}

impl From<ffi::edid::di_edid_cvt_aspect_ratio> for CvtAspectRatio {
    fn from(value: ffi::edid::di_edid_cvt_aspect_ratio) -> Self {
        CvtAspectRatio::from_bits_retain(value)
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
    pub struct CvtScaling: u32 {
        const HorizShrink = ffi::edid::di_edid_cvt_scaling_DI_EDID_CVT_SCALING_HORIZ_SHRINK;
        const HorizStretch = ffi::edid::di_edid_cvt_scaling_DI_EDID_CVT_SCALING_HORIZ_STRETCH;
        const VertShrink = ffi::edid::di_edid_cvt_scaling_DI_EDID_CVT_SCALING_VERT_SHRINK;
        const VertStretch = ffi::edid::di_edid_cvt_scaling_DI_EDID_CVT_SCALING_VERT_STRETCH;
        const _ = !0;
    }
}

impl From<ffi::edid::di_edid_cvt_scaling> for CvtScaling {
    fn from(value: ffi::edid::di_edid_cvt_scaling) -> Self {
        CvtScaling::from_bits_retain(value)
    }
}

#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_range_limits_cvt)]
pub struct DisplayRangeLimitsCvt {
    pub version: i32,
    pub revision: i32,
    #[optional(0i32)]
    pub max_horiz_px: Option<i32>,
    pub supported_aspect_ratio: CvtAspectRatio,
    pub preferred_aspect_ratio: CvtAspectRatio,
    pub standard_blanking: bool,
    pub reduced_blanking: bool,
    pub supported_scaling: CvtScaling,
    pub preferred_vert_refresh_hz: i32,
}

/// EDID display range limits, defined in section 3.10.3.3.1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_display_range_limits)]
pub struct DisplayRangeLimits {
    pub min_vert_rate_hz: i32,
    pub max_vert_rate_hz: i32,
    pub min_horiz_rate_hz: i32,
    pub max_horiz_rate_hz: i32,
    #[optional(0)]
    pub max_pixel_clock_hz: Option<i64>,
    pub type_: DisplayRangeLimitsType,
    #[ptr_deref]
    pub secondary_gtf: Option<DisplayRangeLimitsSecondaryGtf>,
    #[ptr_deref]
    pub cvt: Option<DisplayRangeLimitsCvt>,
}

// EDID Color Points, defined in section 3.10.3.5.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_color_point)]
pub struct ColorPoint {
    pub index: ::std::os::raw::c_int,
    pub white_x: f32,
    pub white_y: f32,
    #[optional(0f32)]
    pub gamma: Option<f32>,
}

/// EDID display Color Management Data, defined in section 3.10.3.7
///
/// Contains the coefficients for the function `L = a₃ × v³ + a₂ × v²`
/// describing the luminance response L to some voltage v [0, 0.7] for each color
/// channel.
///
/// For more information see VESA DCM Standard, Version 1; January 6, 2003
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_color_management_data)]
pub struct ColorManagementData {
    pub version: i32,
    pub red_a3: f32,
    pub red_a2: f32,
    pub green_a3: f32,
    pub green_a2: f32,
    pub blue_a3: f32,
    pub blue_a2: f32,
}

/// Aspect ratio for an EDID CVT Timing Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_cvt_timing_code_aspect_ratio)]
#[repr(u32)]
pub enum CvtTimingCodeAspectRatio {
    _4_3 = ffi::edid::di_edid_cvt_timing_code_aspect_ratio_DI_EDID_CVT_TIMING_CODE_4_3,
    _16_9 = ffi::edid::di_edid_cvt_timing_code_aspect_ratio_DI_EDID_CVT_TIMING_CODE_16_9,
    _16_10 = ffi::edid::di_edid_cvt_timing_code_aspect_ratio_DI_EDID_CVT_TIMING_CODE_16_10,
    _15_9 = ffi::edid::di_edid_cvt_timing_code_aspect_ratio_DI_EDID_CVT_TIMING_CODE_15_9,
}

/// Preferred Vertical Rate for an EDID CVT Timing Code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_cvt_timing_code_preferred_vrate)]
#[repr(u32)]
pub enum CvtTimingCodePreferredVrate {
    _50HZ = ffi::edid::di_edid_cvt_timing_code_preferred_vrate_DI_EDID_CVT_TIMING_CODE_PREFERRED_VRATE_50HZ,
    _60HZ = ffi::edid::di_edid_cvt_timing_code_preferred_vrate_DI_EDID_CVT_TIMING_CODE_PREFERRED_VRATE_60HZ,
    _75HZ = ffi::edid::di_edid_cvt_timing_code_preferred_vrate_DI_EDID_CVT_TIMING_CODE_PREFERRED_VRATE_75HZ,
    _85HZ = ffi::edid::di_edid_cvt_timing_code_preferred_vrate_DI_EDID_CVT_TIMING_CODE_PREFERRED_VRATE_85HZ,
}

/// EDID CVT Timing Code, defined in section 3.10.3.8
///
/// For more information see VESA Coordinated Video Timings (CVT) Standard.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::edid::di_edid_cvt_timing_code)]
pub struct CvtTimingCode {
    pub addressable_lines_per_field: i32,
    pub aspect_ratio: CvtTimingCodeAspectRatio,
    pub supports_50hz_sb: bool,
    pub supports_60hz_sb: bool,
    pub supports_75hz_sb: bool,
    pub supports_85hz_sb: bool,
    pub supports_60hz_rb: bool,
    pub preferred_vertical_rate: CvtTimingCodePreferredVrate,
}

/// EDID extension block tags, defined in section 2.2.4.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::edid::di_edid_ext_tag)]
#[repr(u32)]
pub enum ExtensionTag {
    CEA = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_CEA,
    VTB = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_VTB,
    DI = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_DI,
    LS = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_LS,
    DPVL = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_DPVL,
    BlockMap = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_BLOCK_MAP,
    Vendor = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_VENDOR,
    DisplayId = ffi::edid::di_edid_ext_tag_DI_EDID_EXT_DISPLAYID,
    #[other]
    Unknown,
}

#[derive(Debug)]
#[repr(transparent)]
pub struct ExtensionRef(*const ffi::edid::di_edid_ext);

impl ExtensionRef {
    /// Get the tag of an EDID extension block.
    pub fn tag(&self) -> ExtensionTag {
        ExtensionTag::from(unsafe { ffi::edid::di_edid_ext_get_tag(self.0) })
    }

    pub(crate) fn as_ptr(&self) -> *const ffi::edid::di_edid_ext {
        self.0
    }
}
