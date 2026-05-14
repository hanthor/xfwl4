//! Low-level API for VESA Coordinated Video Timings (CVT) version 2.0.
use std::mem::MaybeUninit;

use libdisplay_info_derive::FFIFrom;

use crate::ffi;

#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::cvt::di_cvt_reduced_blanking_version)]
#[repr(u32)]
pub enum ReducedBlankingVersion {
    None = ffi::cvt::di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_NONE,
    V1 = ffi::cvt::di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_V1,
    V2 = ffi::cvt::di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_V2,
    V3 = ffi::cvt::di_cvt_reduced_blanking_version_DI_CVT_REDUCED_BLANKING_V3,
}

/// Input parameters, defined in table 3-1.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::cvt::di_cvt_options)]
pub struct Options {
    pub red_blank_ver: ReducedBlankingVersion,
    pub h_pixels: i32,
    pub v_lines: i32,
    pub ip_freq_rqd: f64,
    pub video_opt: bool,
    pub vblank: f64,
    pub additional_hblank: i32,
    pub early_vsync_rqd: bool,
    pub int_rqd: bool,
    pub margins_rqd: bool,
}

/// Output parameters, defined in table 3-4.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::cvt::di_cvt_timing)]
pub struct Timing {
    pub act_pixel_freq: f64,
    pub total_active_pixels: f64,
    pub v_lines_rnd: f64,
    pub h_front_porch: f64,
    pub h_sync: f64,
    pub h_back_porch: f64,
    pub v_front_porch: f64,
    pub v_sync: f64,
    pub v_back_porch: f64,
    pub act_frame_rate: f64,
}

impl Timing {
    /// Compute a timing via the CVT formula.
    pub fn compute(options: Options) -> Self {
        let mut timing = MaybeUninit::<ffi::cvt::di_cvt_timing>::uninit();
        let options = ffi::cvt::di_cvt_options {
            red_blank_ver: options.red_blank_ver as u32,
            h_pixels: options.h_pixels,
            v_lines: options.v_lines,
            ip_freq_rqd: options.ip_freq_rqd,
            video_opt: options.video_opt,
            vblank: options.vblank,
            additional_hblank: options.additional_hblank,
            early_vsync_rqd: options.early_vsync_rqd,
            int_rqd: options.int_rqd,
            margins_rqd: options.margins_rqd,
        };
        unsafe { ffi::cvt::di_cvt_compute(timing.as_mut_ptr(), &options) };
        Timing::from(unsafe { timing.assume_init() })
    }
}
