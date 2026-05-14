//! Low-level API for VESA Display Identification Data (DisplayID).
//!
//! The library implements DisplayID version 1.3, available at:
//! <https://vesa.org/vesa-standards/>
use std::marker::PhantomData;

use libdisplay_info_derive::FFIFrom;

use crate::{edid::ExtensionRef, ffi, FFIIter};

pub struct DisplayId<'ext> {
    display_id: *const ffi::displayid::di_displayid,
    phantom: PhantomData<&'ext ()>,
}

impl<'ext> DisplayId<'ext> {
    /// Get a DisplayID extension block.
    ///
    /// Returns `None` if the extension block tag is not [DisplayId](crate::edid::ExtensionTag::DisplayId).
    pub fn from_extension(extensions: &'ext ExtensionRef) -> Option<DisplayId<'ext>> {
        let display_id = unsafe { ffi::edid::di_edid_ext_get_displayid(extensions.as_ptr()) };

        if display_id.is_null() {
            None
        } else {
            Some(Self {
                display_id: display_id as *const ffi::displayid::di_displayid,
                phantom: PhantomData,
            })
        }
    }

    /// Get the DisplayID version.
    pub fn version(&self) -> i32 {
        unsafe { ffi::displayid::di_displayid_get_version(self.display_id) }
    }

    /// Get the DisplayID revision.
    pub fn revision(&self) -> i32 {
        unsafe { ffi::displayid::di_displayid_get_revision(self.display_id) }
    }

    /// Get the DisplayID product type.
    pub fn product_type(&self) -> ProductType {
        ProductType::from(unsafe { ffi::displayid::di_displayid_get_product_type(self.display_id) })
    }

    /// Get DisplayID data blocks
    pub fn data_blocks(&self) -> &[DataBlockRef] {
        let data_blocks = unsafe { ffi::displayid::di_displayid_get_data_blocks(self.display_id) };

        let mut len = 0;
        while !unsafe { *data_blocks.offset(len) }.is_null() {
            len += 1;
        }

        unsafe { std::slice::from_raw_parts(data_blocks as *const DataBlockRef, len as usize) }
    }
}

/// Product type identifier, defined in section 2.3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_product_type)]
#[repr(u32)]
pub enum ProductType {
    Extension = ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_EXTENSION,
    Test = ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_TEST,
    DisplayPanel =
        ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_DISPLAY_PANEL,
    StandaloneDisplay =
        ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_STANDALONE_DISPLAY,
    TvReceiver = ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_TV_RECEIVER,
    Repeater = ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_REPEATER,
    DirectDrive = ffi::displayid::di_displayid_product_type_DI_DISPLAYID_PRODUCT_TYPE_DIRECT_DRIVE,
}

/// DisplayID data block tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_product_type)]
#[repr(u32)]
pub enum DataBlockTag {
    ProductId = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_PRODUCT_ID,
    DisplayParams =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_DISPLAY_PARAMS,
    ColorCharact =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_COLOR_CHARACT,
    TypeITiming = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_I_TIMING,
    TypeIITiming =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_II_TIMING,
    TypeIIITiming =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_III_TIMING,
    TypIVTiming =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_IV_TIMING,
    VesaTiming = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_VESA_TIMING,
    CeaTiming = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_CEA_TIMING,
    TimingRangeLimits =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TIMING_RANGE_LIMITS,
    ProductSerial =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_PRODUCT_SERIAL,
    AsciiString = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_ASCII_STRING,
    DisplayDeviceData =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_DISPLAY_DEVICE_DATA,
    InterfacePowerSeq =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_INTERFACE_POWER_SEQ,
    TransferCharact =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TRANSFER_CHARACT,
    DisplayInterface =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_DISPLAY_INTERFACE,
    StereoDisplayInterface = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_STEREO_DISPLAY_INTERFACE,
    TypeVTiming = ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_V_TIMING,
    TiledDisplayTopo =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TILED_DISPLAY_TOPO,
    TypeVITiming =
        ffi::displayid::di_displayid_data_block_tag_DI_DISPLAYID_DATA_BLOCK_TYPE_VI_TIMING,
}

/// A DisplayID data block.
#[repr(transparent)]
pub struct DataBlockRef(*const ffi::displayid::di_displayid_data_block);

impl DataBlockRef {
    /// Get a DisplayID data block tag.
    pub fn tag(&self) -> DataBlockTag {
        DataBlockTag::from(unsafe { ffi::displayid::di_displayid_data_block_get_tag(self.0) })
    }

    /// Get display parameters from a DisplayID data block.
    ///
    /// Returns `None` if the data block tag isn't
    /// DI_DISPLAYID_DATA_BLOCK_DISPLAY_PARAMS.
    pub fn display_params(&self) -> Option<DisplayParams> {
        DisplayParams::from_ptr(unsafe {
            ffi::displayid::di_displayid_data_block_get_display_params(self.0)
        })
    }

    /// Get type I timings from a DisplayID data block.
    /// Returns `None` if the data block tag isn't
    /// DI_DISPLAYID_DATA_BLOCK_TYPE_I_TIMING.
    pub fn type_i_timings(&self) -> impl Iterator<Item = TypeIIIVIITiming> {
        FFIIter::new(unsafe { ffi::displayid::di_displayid_data_block_get_type_i_timings(self.0) })
    }

    /// Get type II timings from a DisplayID data block.
    /// Returns `None` if the data block tag isn't
    /// DI_DISPLAYID_DATA_BLOCK_TYPE_II_TIMING.
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn type_ii_timings(&self) -> impl Iterator<Item = TypeIIIVIITiming> {
        FFIIter::new(unsafe { ffi::displayid::di_displayid_data_block_get_type_ii_timings(self.0) })
    }

    /// Get type III timings from a DisplayID data block.
    /// Returns `None` if the data block tag isn't
    /// DI_DISPLAYID_DATA_BLOCK_TYPE_III_TIMING.
    #[cfg(any(feature = "v0_2", feature = "v0_3"))]
    pub fn type_iii_timings(&self) -> impl Iterator<Item = TypeIIITiming> {
        FFIIter::new(unsafe {
            ffi::displayid::di_displayid_data_block_get_type_iii_timings(self.0)
        })
    }

    /// Get tiled display topology from a DisplayID data block.
    ///
    /// Returns `None` if the data block tag isn't
    /// DI_DISPLAYID_DATA_BLOCK_TILED_DISPLAY_TOPO.
    pub fn tiled_topo(&self) -> Option<TiledTopo> {
        TiledTopo::from_ptr(unsafe {
            ffi::displayid::di_displayid_data_block_get_tiled_topo(self.0)
        })
    }
}

/// Display parameters feature support flags, defined in section 4.2.3.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_display_params_features)]
pub struct DisplayParamsFeatures {
    pub audio: bool,
    pub separate_audio_inputs: bool,
    pub audio_input_override: bool,
    pub power_management: bool,
    pub fixed_timing: bool,
    pub fixed_pixel_format: bool,
    pub ai: bool,
    pub deinterlacing: bool,
}

/// Display parameters data block, defined in section 4.2.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_display_params)]
pub struct DisplayParams {
    pub horiz_image_mm: f32,
    pub vert_image_mm: f32,
    pub horiz_pixels: i32,
    pub vert_pixels: i32,
    #[ptr_deref]
    pub features: Option<DisplayParamsFeatures>,
    #[optional(0f32)]
    pub gamma: Option<f32>,
    pub aspect_ratio: f32,
    pub bits_per_color_overall: i32,
    pub bits_per_color_native: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_type_i_ii_vii_timing_stereo_3d)]
#[repr(u32)]
pub enum TypeIIIVIITimingStereo3d {
    Never = ffi::displayid::di_displayid_type_i_ii_vii_timing_stereo_3d_DI_DISPLAYID_TYPE_I_II_VII_TIMING_STEREO_3D_NEVER,
    Always = ffi::displayid::di_displayid_type_i_ii_vii_timing_stereo_3d_DI_DISPLAYID_TYPE_I_II_VII_TIMING_STEREO_3D_ALWAYS,
    User = ffi::displayid::di_displayid_type_i_ii_vii_timing_stereo_3d_DI_DISPLAYID_TYPE_I_II_VII_TIMING_STEREO_3D_USER,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_timing_aspect_ratio)]
#[repr(u32)]
pub enum TimingAspectRatio {
    _1_1 = ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_1_1,
    _5_4 = ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_5_4,
    _4_3 = ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_4_3,
    _15_9 = ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_15_9,
    _16_9 = ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_16_9,
    _16_10 =
        ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_16_10,
    _64_27 =
        ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_64_27,
    _256_135 =
        ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_256_135,
    Undefined =
        ffi::displayid::di_displayid_timing_aspect_ratio_DI_DISPLAYID_TIMING_ASPECT_RATIO_UNDEFINED,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_type_i_ii_vii_timing_sync_polarity)]
#[repr(u32)]
pub enum TypeIIIVIITimingSyncPolarity {
    Negative = ffi::displayid::di_displayid_type_i_ii_vii_timing_sync_polarity_DI_DISPLAYID_TYPE_I_II_VII_TIMING_SYNC_NEGATIVE,
    Positive = ffi::displayid::di_displayid_type_i_ii_vii_timing_sync_polarity_DI_DISPLAYID_TYPE_I_II_VII_TIMING_SYNC_POSITIVE,
}

/// Type I timing, defined in section 4.4.1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_type_i_ii_vii_timing)]
pub struct TypeIIIVIITiming {
    pub pixel_clock_mhz: f64,
    pub preferred: bool,
    pub stereo_3d: TypeIIIVIITimingStereo3d,
    pub interlaced: bool,
    pub aspect_ratio: TimingAspectRatio,
    pub horiz_active: i32,
    pub vert_active: i32,
    pub horiz_blank: i32,
    pub vert_blank: i32,
    pub horiz_offset: i32,
    pub vert_offset: i32,
    pub horiz_sync_width: i32,
    pub vert_sync_width: i32,
    pub horiz_sync_polarity: TypeIIIVIITimingSyncPolarity,
    pub vert_sync_polarity: TypeIIIVIITimingSyncPolarity,
}

/// Formula/algorithm for type III timings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_type_iii_timing_algo)]
#[repr(u32)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub enum TyoeIIITimingAlgo {
    CvtStandardBlanking = ffi::displayid::di_displayid_type_iii_timing_algo_DI_DISPLAYID_TYPE_III_TIMING_CVT_STANDARD_BLANKING,
    CvtReducedBlacking = ffi::displayid::di_displayid_type_iii_timing_algo_DI_DISPLAYID_TYPE_III_TIMING_CVT_REDUCED_BLANKING,
}

/// Type I timing, defined in section 4.4.1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_type_iii_timing)]
#[cfg(any(feature = "v0_2", feature = "v0_3"))]
pub struct TypeIIITiming {
    pub preferred: bool,
    pub algo: TyoeIIITimingAlgo,
    pub aspect_ratio: TimingAspectRatio,
    pub horiz_active: i32,
    pub interlaced: bool,
    pub refresh_rate_hz: i32,
}

/// Behavior when more than 1 tile and less than total number of tiles are driven
/// by the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_tiled_topo_missing_recv_behavior)]
#[repr(u32)]
pub enum TiledTopoMissingRecvBehavior {
    Undef = ffi::displayid::di_displayid_tiled_topo_missing_recv_behavior_DI_DISPLAYID_TILED_TOPO_MISSING_RECV_UNDEF,
    TileOnly = ffi::displayid::di_displayid_tiled_topo_missing_recv_behavior_DI_DISPLAYID_TILED_TOPO_MISSING_RECV_TILE_ONLY,
}

/// Behavior of this tile when it is the only tile receiving an image from the
/// source.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_tiled_topo_single_recv_behavior)]
#[repr(u32)]
pub enum TiledTopoSingleRecvBehavior {
    Undef = ffi::displayid::di_displayid_tiled_topo_single_recv_behavior_DI_DISPLAYID_TILED_TOPO_SINGLE_RECV_UNDEF,
    TileOnly = ffi::displayid::di_displayid_tiled_topo_single_recv_behavior_DI_DISPLAYID_TILED_TOPO_SINGLE_RECV_TILE_ONLY,
    Scaled = ffi::displayid::di_displayid_tiled_topo_single_recv_behavior_DI_DISPLAYID_TILED_TOPO_SINGLE_RECV_SCALED,
    Cloned = ffi::displayid::di_displayid_tiled_topo_single_recv_behavior_DI_DISPLAYID_TILED_TOPO_SINGLE_RECV_CLONED,
}

/// Tiled display capabilities.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_tiled_topo_caps)]
pub struct TiledTopoCaps {
    pub single_enclosure: bool,
    pub missing_recv_behavior: TiledTopoMissingRecvBehavior,
    pub single_recv_behavior: TiledTopoSingleRecvBehavior,
}

/// Tiled display bezel information.
///
/// The lengths are measured in pixels, accurate to the tenths place.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_tiled_topo_bezel)]
pub struct TiledTopoBezel {
    pub top_px: f32,
    pub bottom_px: f32,
    pub right_px: f32,
    pub left_px: f32,
}

/// Tiled display topology, defined in section 4.14.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::displayid::di_displayid_tiled_topo)]
pub struct TiledTopo {
    #[ptr_deref]
    pub caps: Option<TiledTopoCaps>,
    pub total_horiz_tiles: i32,
    pub total_vert_tiles: i32,
    pub horiz_tile_location: i32,
    pub vert_tile_location: i32,
    pub horiz_tile_pixels: i32,
    pub vert_tile_lines: i32,
    #[ptr_deref]
    pub bezel: Option<TiledTopoBezel>,
    #[cast_as(u8)]
    pub vendor_id: [char; 3usize],
    pub product_code: u16,
    pub serial_number: u32,
}
