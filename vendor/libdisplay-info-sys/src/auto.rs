macro_rules! auto_import {
    ($mod_name:ident) => {
        #[cfg_attr(docsrs, cfg(feature = "v0_1"))]
        #[cfg_attr(
            not(docsrs),
            cfg(all(feature = "v0_1", not(feature = "v0_2"), not(feature = "v0_3")))
        )]
        pub use crate::v0_1::$mod_name::*;
        #[cfg_attr(docsrs, cfg(feature = "v0_2"))]
        #[cfg_attr(not(docsrs), cfg(all(feature = "v0_2", not(feature = "v0_3"))))]
        pub use crate::v0_2::$mod_name::*;
        #[cfg(feature = "v0_3")]
        pub use crate::v0_3::$mod_name::*;
    };
}

macro_rules! auto_mod {
    ($mod_name:ident) => {
        pub mod $mod_name {
            auto_import!($mod_name);
        }
    };
}

pub mod cta {
    auto_import!(cta);

    #[cfg(feature = "v0_3")]
    pub use polyfills::*;
    #[cfg(feature = "v0_3")]
    mod polyfills {
        use super::*;

        pub unsafe fn di_cta_data_block_get_sads(
            data_block: *const di_cta_data_block,
        ) -> *const *const di_cta_sad {
            let audio_block = di_cta_data_block_get_audio(data_block);
            if audio_block.is_null() {
                return std::ptr::null();
            }
            (*audio_block).sads
        }

        pub type di_cta_vesa_dddb = di_cta_vesa_display_device_block;
        pub unsafe fn di_cta_data_block_get_vesa_dddb(
            block: *const di_cta_data_block,
        ) -> *const di_cta_vesa_dddb {
            di_cta_data_block_get_vesa_display_device(block)
        }

        pub unsafe fn di_cta_data_block_get_svds(
            block: *const di_cta_data_block,
        ) -> *const *const di_cta_svd {
            let video_block = di_cta_data_block_get_video(block);
            if video_block.is_null() {
                return std::ptr::null();
            }
            (*video_block).svds
        }

        pub unsafe fn di_cta_data_block_get_ycbcr420_svds(
            block: *const di_cta_data_block,
        ) -> *const *const di_cta_svd {
            let video_block = di_cta_data_block_get_ycbcr420_video(block);
            if video_block.is_null() {
                return std::ptr::null();
            }
            (*video_block).svds
        }

        pub unsafe fn di_cta_data_block_get_svrs(
            block: *const di_cta_data_block,
        ) -> *const *const di_cta_svr {
            let video_format_pref_block = di_cta_data_block_get_video_format_pref(block);
            if video_format_pref_block.is_null() {
                return std::ptr::null();
            }
            (*video_format_pref_block).svrs
        }

        pub type di_cta_vesa_dddb_interface_type = di_cta_vesa_display_device_interface_type;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_VGA:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_VGA;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_NAVI_V:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_NAVI_V;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_NAVI_D:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_NAVI_D;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_LVDS:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_LVDS;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_RSDS:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_RSDS;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_DVI_D:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_DVI_D;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_DVI_I_ANALOG:
        di_cta_vesa_dddb_interface_type =
        di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_DVI_I_ANALOG;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_DVI_I_DIGITAL:
    di_cta_vesa_dddb_interface_type =
    di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_DVI_I_DIGITAL;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_HDMI_A:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_HDMI_A;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_HDMI_B:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_HDMI_B;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_MDDI:
            di_cta_vesa_dddb_interface_type =
            di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_MDDI;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_DISPLAYPORT:
        di_cta_vesa_dddb_interface_type =
        di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_DISPLAYPORT;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_IEEE_1394:
        di_cta_vesa_dddb_interface_type =
        di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_IEEE_1394;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_M1_ANALOG:
        di_cta_vesa_dddb_interface_type =
        di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_M1_ANALOG;
        pub const di_cta_vesa_dddb_interface_type_DI_CTA_VESA_DDDB_INTERFACE_M1_DIGITAL:
        di_cta_vesa_dddb_interface_type =
        di_cta_vesa_display_device_interface_type_DI_CTA_VESA_DISPLAY_DEVICE_INTERFACE_M1_DIGITAL;

        pub type di_cta_vesa_dddb_content_protection =
            di_cta_vesa_display_device_content_protection;
        pub const di_cta_vesa_dddb_content_protection_DI_CTA_VESA_DDDB_CONTENT_PROTECTION_NONE: di_cta_vesa_dddb_content_protection = di_cta_vesa_display_device_content_protection_DI_CTA_VESA_DISPLAY_DEVICE_CONTENT_PROTECTION_NONE;
        pub const di_cta_vesa_dddb_content_protection_DI_CTA_VESA_DDDB_CONTENT_PROTECTION_HDCP: di_cta_vesa_dddb_content_protection = di_cta_vesa_display_device_content_protection_DI_CTA_VESA_DISPLAY_DEVICE_CONTENT_PROTECTION_HDCP;
        pub const di_cta_vesa_dddb_content_protection_DI_CTA_VESA_DDDB_CONTENT_PROTECTION_DTCP: di_cta_vesa_dddb_content_protection = di_cta_vesa_display_device_content_protection_DI_CTA_VESA_DISPLAY_DEVICE_CONTENT_PROTECTION_DTCP;
        pub const di_cta_vesa_dddb_content_protection_DI_CTA_VESA_DDDB_CONTENT_PROTECTION_DPCP: di_cta_vesa_dddb_content_protection = di_cta_vesa_display_device_content_protection_DI_CTA_VESA_DISPLAY_DEVICE_CONTENT_PROTECTION_DPCP;

        pub type di_cta_vesa_dddb_default_orientation =
            di_cta_vesa_display_device_default_orientation;
        pub const di_cta_vesa_dddb_default_orientation_DI_CTA_VESA_DDDB_DEFAULT_ORIENTATION_LANDSCAPE: di_cta_vesa_dddb_default_orientation = di_cta_vesa_display_device_default_orientation_DI_CTA_VESA_DISPLAY_DEVICE_DEFAULT_ORIENTATION_LANDSCAPE;
        pub const di_cta_vesa_dddb_default_orientation_DI_CTA_VESA_DDDB_DEFAULT_ORIENTATION_PORTAIT: di_cta_vesa_dddb_default_orientation = di_cta_vesa_display_device_default_orientation_DI_CTA_VESA_DISPLAY_DEVICE_DEFAULT_ORIENTATION_PORTAIT;
        pub const di_cta_vesa_dddb_default_orientation_DI_CTA_VESA_DDDB_DEFAULT_ORIENTATION_UNFIXED: di_cta_vesa_dddb_default_orientation = di_cta_vesa_display_device_default_orientation_DI_CTA_VESA_DISPLAY_DEVICE_DEFAULT_ORIENTATION_UNFIXED;
        pub const di_cta_vesa_dddb_default_orientation_DI_CTA_VESA_DDDB_DEFAULT_ORIENTATION_UNDEFINED: di_cta_vesa_dddb_default_orientation = di_cta_vesa_display_device_default_orientation_DI_CTA_VESA_DISPLAY_DEVICE_DEFAULT_ORIENTATION_UNDEFINED;

        pub type di_cta_vesa_dddb_rotation_cap = di_cta_vesa_display_device_rotation_cap;
        pub const di_cta_vesa_dddb_rotation_cap_DI_CTA_VESA_DDDB_ROTATION_CAP_NONE:
            di_cta_vesa_dddb_rotation_cap =
            di_cta_vesa_display_device_rotation_cap_DI_CTA_VESA_DISPLAY_DEVICE_ROTATION_CAP_NONE;
        pub const di_cta_vesa_dddb_rotation_cap_DI_CTA_VESA_DDDB_ROTATION_CAP_90DEG_CLOCKWISE:
    di_cta_vesa_dddb_rotation_cap =
    di_cta_vesa_display_device_rotation_cap_DI_CTA_VESA_DISPLAY_DEVICE_ROTATION_CAP_90DEG_CLOCKWISE;
        pub const di_cta_vesa_dddb_rotation_cap_DI_CTA_VESA_DDDB_ROTATION_CAP_90DEG_COUNTERCLOCKWISE: di_cta_vesa_dddb_rotation_cap = di_cta_vesa_display_device_rotation_cap_DI_CTA_VESA_DISPLAY_DEVICE_ROTATION_CAP_90DEG_COUNTERCLOCKWISE;
        pub const di_cta_vesa_dddb_rotation_cap_DI_CTA_VESA_DDDB_ROTATION_CAP_90DEG_EITHER:
    di_cta_vesa_dddb_rotation_cap =
    di_cta_vesa_display_device_rotation_cap_DI_CTA_VESA_DISPLAY_DEVICE_ROTATION_CAP_90DEG_EITHER;

        pub type di_cta_vesa_dddb_zero_pixel_location =
            di_cta_vesa_display_device_zero_pixel_location;
        pub const di_cta_vesa_dddb_zero_pixel_location_DI_CTA_VESA_DDDB_ZERO_PIXEL_UPPER_LEFT:
    di_cta_vesa_dddb_zero_pixel_location =
    di_cta_vesa_display_device_zero_pixel_location_DI_CTA_VESA_DISPLAY_DEVICE_ZERO_PIXEL_UPPER_LEFT;
        pub const di_cta_vesa_dddb_zero_pixel_location_DI_CTA_VESA_DDDB_ZERO_PIXEL_UPPER_RIGHT: di_cta_vesa_dddb_zero_pixel_location = di_cta_vesa_display_device_zero_pixel_location_DI_CTA_VESA_DISPLAY_DEVICE_ZERO_PIXEL_UPPER_RIGHT;
        pub const di_cta_vesa_dddb_zero_pixel_location_DI_CTA_VESA_DDDB_ZERO_PIXEL_LOWER_LEFT:
    di_cta_vesa_dddb_zero_pixel_location =
    di_cta_vesa_display_device_zero_pixel_location_DI_CTA_VESA_DISPLAY_DEVICE_ZERO_PIXEL_LOWER_LEFT;
        pub const di_cta_vesa_dddb_zero_pixel_location_DI_CTA_VESA_DDDB_ZERO_PIXEL_LOWER_RIGHT: di_cta_vesa_dddb_zero_pixel_location = di_cta_vesa_display_device_zero_pixel_location_DI_CTA_VESA_DISPLAY_DEVICE_ZERO_PIXEL_LOWER_RIGHT;

        pub type di_cta_vesa_dddb_scan_direction = di_cta_vesa_display_device_scan_direction;
        pub const di_cta_vesa_dddb_scan_direction_DI_CTA_VESA_DDDB_SCAN_DIRECTION_UNDEFINED:
    di_cta_vesa_dddb_scan_direction =
    di_cta_vesa_display_device_scan_direction_DI_CTA_VESA_DISPLAY_DEVICE_SCAN_DIRECTION_UNDEFINED;
        pub const di_cta_vesa_dddb_scan_direction_DI_CTA_VESA_DDDB_SCAN_DIRECTION_FAST_LONG_SLOW_SHORT: di_cta_vesa_dddb_scan_direction = di_cta_vesa_display_device_scan_direction_DI_CTA_VESA_DISPLAY_DEVICE_SCAN_DIRECTION_FAST_LONG_SLOW_SHORT;
        pub const di_cta_vesa_dddb_scan_direction_DI_CTA_VESA_DDDB_SCAN_DIRECTION_FAST_SHORT_SLOW_LONG: di_cta_vesa_dddb_scan_direction = di_cta_vesa_display_device_scan_direction_DI_CTA_VESA_DISPLAY_DEVICE_SCAN_DIRECTION_FAST_SHORT_SLOW_LONG;

        pub type di_cta_vesa_dddb_subpixel_layout = di_cta_vesa_display_device_subpixel_layout;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_UNDEFINED:
        di_cta_vesa_dddb_subpixel_layout =
        di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_UNDEFINED;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_RGB_VERT:
            di_cta_vesa_dddb_subpixel_layout =
            di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_RGB_VERT;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_RGB_HORIZ:
        di_cta_vesa_dddb_subpixel_layout =
        di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_RGB_HORIZ;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_EDID_CHROM_VERT:
    di_cta_vesa_dddb_subpixel_layout =
    di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_EDID_CHROM_VERT;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_EDID_CHROM_HORIZ:
    di_cta_vesa_dddb_subpixel_layout =
    di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_EDID_CHROM_HORIZ;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_QUAD_RGGB:
        di_cta_vesa_dddb_subpixel_layout =
        di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_QUAD_RGGB;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_QUAD_GBRG:
        di_cta_vesa_dddb_subpixel_layout =
        di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_QUAD_GBRG;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_DELTA_RGB:
        di_cta_vesa_dddb_subpixel_layout =
        di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_DELTA_RGB;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_MOSAIC:
            di_cta_vesa_dddb_subpixel_layout =
            di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_MOSAIC;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_QUAD_ANY:
            di_cta_vesa_dddb_subpixel_layout =
            di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_QUAD_ANY;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_FIVE:
            di_cta_vesa_dddb_subpixel_layout =
            di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_FIVE;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_SIX:
            di_cta_vesa_dddb_subpixel_layout =
            di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_SIX;
        pub const di_cta_vesa_dddb_subpixel_layout_DI_CTA_VESA_DDDB_SUBPIXEL_CLAIRVOYANTE_PENTILE: di_cta_vesa_dddb_subpixel_layout = di_cta_vesa_display_device_subpixel_layout_DI_CTA_VESA_DISPLAY_DEVICE_SUBPIXEL_CLAIRVOYANTE_PENTILE;

        pub type di_cta_vesa_dddb_dithering_type = di_cta_vesa_display_device_dithering_type;
        pub const di_cta_vesa_dddb_dithering_type_DI_CTA_VESA_DDDB_DITHERING_NONE:
            di_cta_vesa_dddb_dithering_type =
            di_cta_vesa_display_device_dithering_type_DI_CTA_VESA_DISPLAY_DEVICE_DITHERING_NONE;
        pub const di_cta_vesa_dddb_dithering_type_DI_CTA_VESA_DDDB_DITHERING_SPACIAL:
            di_cta_vesa_dddb_dithering_type =
            di_cta_vesa_display_device_dithering_type_DI_CTA_VESA_DISPLAY_DEVICE_DITHERING_SPACIAL;
        pub const di_cta_vesa_dddb_dithering_type_DI_CTA_VESA_DDDB_DITHERING_TEMPORAL:
            di_cta_vesa_dddb_dithering_type =
            di_cta_vesa_display_device_dithering_type_DI_CTA_VESA_DISPLAY_DEVICE_DITHERING_TEMPORAL;
        pub const di_cta_vesa_dddb_dithering_type_DI_CTA_VESA_DDDB_DITHERING_SPATIAL_AND_TEMPORAL: di_cta_vesa_dddb_dithering_type = di_cta_vesa_display_device_dithering_type_DI_CTA_VESA_DISPLAY_DEVICE_DITHERING_SPATIAL_AND_TEMPORAL;

        pub type di_cta_vesa_dddb_additional_primary_chromaticity =
            di_cta_vesa_display_device_additional_primary_chromaticity;

        pub type di_cta_vesa_dddb_frame_rate_conversion =
            di_cta_vesa_display_device_frame_rate_conversion;
        pub const di_cta_vesa_dddb_frame_rate_conversion_DI_CTA_VESA_DDDB_FRAME_RATE_CONVERSION_NONE: di_cta_vesa_dddb_frame_rate_conversion = di_cta_vesa_display_device_frame_rate_conversion_DI_CTA_VESA_DISPLAY_DEVICE_FRAME_RATE_CONVERSION_NONE;
        pub const di_cta_vesa_dddb_frame_rate_conversion_DI_CTA_VESA_DDDB_FRAME_RATE_CONVERSION_SINGLE_BUFFERING : di_cta_vesa_dddb_frame_rate_conversion = di_cta_vesa_display_device_frame_rate_conversion_DI_CTA_VESA_DISPLAY_DEVICE_FRAME_RATE_CONVERSION_SINGLE_BUFFERING ;
        pub const di_cta_vesa_dddb_frame_rate_conversion_DI_CTA_VESA_DDDB_FRAME_RATE_CONVERSION_DOUBLE_BUFFERING : di_cta_vesa_dddb_frame_rate_conversion = di_cta_vesa_display_device_frame_rate_conversion_DI_CTA_VESA_DISPLAY_DEVICE_FRAME_RATE_CONVERSION_DOUBLE_BUFFERING ;
        pub const di_cta_vesa_dddb_frame_rate_conversion_DI_CTA_VESA_DDDB_FRAME_RATE_CONVERSION_ADVANCED: di_cta_vesa_dddb_frame_rate_conversion = di_cta_vesa_display_device_frame_rate_conversion_DI_CTA_VESA_DISPLAY_DEVICE_FRAME_RATE_CONVERSION_ADVANCED;

        pub type di_cta_vesa_dddb_resp_time_transition =
            di_cta_vesa_display_device_resp_time_transition;
        pub const di_cta_vesa_dddb_resp_time_transition_DI_CTA_VESA_DDDB_RESP_TIME_BLACK_TO_WHITE: di_cta_vesa_dddb_resp_time_transition = di_cta_vesa_display_device_resp_time_transition_DI_CTA_VESA_DISPLAY_DEVICE_RESP_TIME_BLACK_TO_WHITE;
        pub const di_cta_vesa_dddb_resp_time_transition_DI_CTA_VESA_DDDB_RESP_TIME_WHITE_TO_BLACK: di_cta_vesa_dddb_resp_time_transition = di_cta_vesa_display_device_resp_time_transition_DI_CTA_VESA_DISPLAY_DEVICE_RESP_TIME_WHITE_TO_BLACK;

        pub type di_cta_hdr_static_metadata_block_eotfs = di_cta_hdr_static_metadata_eotfs;
        pub type di_cta_hdr_static_metadata_block_descriptors =
            di_cta_hdr_static_metadata_descriptors;

        pub type di_cta_hdr_dynamic_metadata_block_type1 = di_cta_hdr_dynamic_metadata_type1;
        pub type di_cta_hdr_dynamic_metadata_block_type2 = di_cta_hdr_dynamic_metadata_type2;
        pub type di_cta_hdr_dynamic_metadata_block_type3 = di_cta_hdr_dynamic_metadata_type3;
        pub type di_cta_hdr_dynamic_metadata_block_type4 = di_cta_hdr_dynamic_metadata_type4;
        pub type di_cta_hdr_dynamic_metadata_block_type256 = di_cta_hdr_dynamic_metadata_type256;

        pub type di_cta_vesa_transfer_characteristics = di_cta_vesa_transfer_characteristics_block;

        pub type di_cta_ycbcr420_cap_map = di_cta_ycbcr420_cap_map_block;

        pub type di_cta_speaker_locations = di_cta_speaker_location_descriptor;

        pub type di_cta_room_configuration = di_cta_room_configuration_block;
    }
}
auto_mod!(cvt);
auto_mod!(displayid);
pub mod displayid2 {
    #[cfg(feature = "v0_3")]
    pub use crate::v0_3::displayid2::*;
}
auto_mod!(dmt);
auto_mod!(edid);
auto_mod!(gtf);
auto_mod!(info);
