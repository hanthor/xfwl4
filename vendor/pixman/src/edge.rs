use std::{mem::MaybeUninit, os::raw::c_int};

use crate::{ffi, Fixed, Line};

/// Defines a single edge
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Edge(ffi::pixman_edge_t);

impl Edge {
    /// Create a edge from the provided values
    pub fn new(
        bpp: i32,
        y_start: impl Into<Fixed>,
        x_top: impl Into<Fixed>,
        y_top: impl Into<Fixed>,
        x_bot: impl Into<Fixed>,
        y_bot: impl Into<Fixed>,
    ) -> Self {
        let mut edge = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_edge_init(
                edge.as_mut_ptr(),
                bpp,
                y_start.into().into_raw(),
                x_top.into().into_raw(),
                y_top.into().into_raw(),
                x_bot.into().into_raw(),
                y_bot.into().into_raw(),
            );
        }
        Self(unsafe { edge.assume_init() })
    }

    /// Initialize one edge structure given a line, starting y value
    /// and a pixel offset for the line
    pub fn from_line(
        bpp: isize,
        y: impl Into<Fixed>,
        line: impl Into<Line>,
        x_off: i32,
        y_off: i32,
    ) -> Self {
        let mut edge = MaybeUninit::uninit();
        unsafe {
            ffi::pixman_line_fixed_edge_init(
                edge.as_mut_ptr(),
                bpp as c_int,
                y.into().into_raw(),
                line.into().as_ptr(),
                x_off,
                y_off,
            )
        }
        Self(unsafe { edge.assume_init() })
    }

    /// Step an edge by any amount (including negative values)
    pub fn step(&mut self, n: i32) {
        unsafe {
            ffi::pixman_edge_step(&mut self.0, n);
        }
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const ffi::pixman_edge_t {
        &self.0
    }
}

impl From<ffi::pixman_edge_t> for Edge {
    #[inline]
    fn from(value: ffi::pixman_edge_t) -> Self {
        Self(value)
    }
}

impl From<Edge> for ffi::pixman_edge_t {
    #[inline]
    fn from(value: Edge) -> Self {
        value.0
    }
}
