//! Low-level API for VESA Display Monitor Timing (DMT).
//!
//! The library implements VESA DMT version 1.0 revision 13.
use libdisplay_info_derive::FFIFrom;

use crate::ffi;

// A DMT timing.
#[derive(Debug, Copy, Clone, FFIFrom)]
#[ffi(ffi::dmt::di_dmt_timing)]
pub struct Timing {
    pub dmt_id: u8,
    pub edid_std_id: u16,
    pub cvt_id: u32,
    pub horiz_video: i32,
    pub vert_video: i32,
    pub refresh_rate_hz: f32,
    pub pixel_clock_hz: i32,
    pub horiz_blank: i32,
    pub vert_blank: i32,
    pub horiz_front_porch: i32,
    pub vert_front_porch: i32,
    pub horiz_sync_pulse: i32,
    pub vert_sync_pulse: i32,
    pub horiz_border: i32,
    pub vert_border: i32,
    pub reduced_blanking: bool,
}
