//! Low-level API for Generalized Timing Formula Standard version 1.1.
use std::mem::MaybeUninit;

use libdisplay_info_derive::FFIFrom;

use crate::ffi;

pub const DEFAULT_M: f64 = 600.0;
pub const DEFAULT_C: f64 = 40.0;
pub const DEFAULT_K: f64 = 128.0;
pub const DEFAULT_J: f64 = 20.0;

/// Type of frequency parameter used in di_gtf_options.ip_freq_rqd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FFIFrom)]
#[ffi(ffi::gtf::di_gtf_ip_param)]
#[repr(u32)]
pub enum IpParam {
    /// Vertical frame frequency (Hz)
    VFrameRate = ffi::gtf::di_gtf_ip_param_DI_GTF_IP_PARAM_V_FRAME_RATE,
    /// Horizontal frequency (kHz)
    HFreq = ffi::gtf::di_gtf_ip_param_DI_GTF_IP_PARAM_H_FREQ,
    /// Pixel clock rate (MHz)
    HPixels = ffi::gtf::di_gtf_ip_param_DI_GTF_IP_PARAM_H_PIXELS,
}

/// Input options for GTF.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::gtf::di_gtf_options)]
pub struct Options {
    /// Number of active image pixels displayed on a line, not including any margin
    pub h_pixels: i32,
    /// Number of vertical lines in the displayed image
    pub v_lines: i32,
    /// Whether margins are required
    pub margins_rqd: bool,
    /// Indicates which frequency parameter is specified in ip_freq_rqd
    pub ip_param: IpParam,
    /// Vertical frame frequency (in Hz), horizontal frequency (in kHz) or pixel clock rate (in MHz)
    pub ip_freq_rqd: f64,
    /// Whether interlaced is required
    pub int_rqd: bool,
    /// Blanking formula gradient
    pub m: f64,
    /// Blanking formula offset
    pub c: f64,
    /// Blanking formula scaling factor
    pub k: f64,
    /// Blanking formula scaling factor weighting
    pub j: f64,
}

/// Output timing data for GTF.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::gtf::di_gtf_timing)]
pub struct Timing {
    pub h_pixels: i32,
    pub v_lines: i32,
    pub h_sync: i32,
    pub v_sync: i32,
    pub h_front_porch: i32,
    pub h_back_porch: i32,
    pub v_front_porch: i32,
    pub v_back_porch: i32,
    pub h_border: i32,
    pub v_border: i32,
    pub pixel_freq_mhz: f64,
}

impl Timing {
    /// Compute a timing via the GTF formula.
    pub fn compute(options: Options) -> Self {
        let mut timing = MaybeUninit::<ffi::gtf::di_gtf_timing>::uninit();
        let options = ffi::gtf::di_gtf_options {
            h_pixels: options.h_pixels,
            v_lines: options.v_lines,
            margins_rqd: options.margins_rqd,
            ip_param: options.ip_param as u32,
            ip_freq_rqd: options.ip_freq_rqd,
            int_rqd: options.int_rqd,
            m: options.m,
            c: options.c,
            k: options.k,
            j: options.j,
        };
        unsafe {
            ffi::gtf::di_gtf_compute(timing.as_mut_ptr(), &options);
        }
        Timing::from(unsafe { timing.assume_init() })
    }
}
