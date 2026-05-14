#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

pub const PIXMAN_MAX_INDEXED: u32 = 256;

pub const PIXMAN_TYPE_OTHER: u32 = 0;
pub const PIXMAN_TYPE_A: u32 = 1;
pub const PIXMAN_TYPE_ARGB: u32 = 2;
pub const PIXMAN_TYPE_ABGR: u32 = 3;
pub const PIXMAN_TYPE_COLOR: u32 = 4;
pub const PIXMAN_TYPE_GRAY: u32 = 5;
pub const PIXMAN_TYPE_YUY2: u32 = 6;
pub const PIXMAN_TYPE_YV12: u32 = 7;
pub const PIXMAN_TYPE_BGRA: u32 = 8;
pub const PIXMAN_TYPE_RGBA: u32 = 9;
pub const PIXMAN_TYPE_ARGB_SRGB: u32 = 10;
pub const PIXMAN_TYPE_RGBA_FLOAT: u32 = 11;

pub const pixman_repeat_t_PIXMAN_REPEAT_NONE: pixman_repeat_t = 0;
pub const pixman_repeat_t_PIXMAN_REPEAT_NORMAL: pixman_repeat_t = 1;
pub const pixman_repeat_t_PIXMAN_REPEAT_PAD: pixman_repeat_t = 2;
pub const pixman_repeat_t_PIXMAN_REPEAT_REFLECT: pixman_repeat_t = 3;

pub const pixman_dither_t_PIXMAN_DITHER_NONE: pixman_dither_t = 0;
pub const pixman_dither_t_PIXMAN_DITHER_FAST: pixman_dither_t = 1;
pub const pixman_dither_t_PIXMAN_DITHER_GOOD: pixman_dither_t = 2;
pub const pixman_dither_t_PIXMAN_DITHER_BEST: pixman_dither_t = 3;
pub const pixman_dither_t_PIXMAN_DITHER_ORDERED_BAYER_8: pixman_dither_t = 4;
pub const pixman_dither_t_PIXMAN_DITHER_ORDERED_BLUE_NOISE_64: pixman_dither_t = 5;

pub const pixman_filter_t_PIXMAN_FILTER_FAST: pixman_filter_t = 0;
pub const pixman_filter_t_PIXMAN_FILTER_GOOD: pixman_filter_t = 1;
pub const pixman_filter_t_PIXMAN_FILTER_BEST: pixman_filter_t = 2;
pub const pixman_filter_t_PIXMAN_FILTER_NEAREST: pixman_filter_t = 3;
pub const pixman_filter_t_PIXMAN_FILTER_BILINEAR: pixman_filter_t = 4;
pub const pixman_filter_t_PIXMAN_FILTER_CONVOLUTION: pixman_filter_t = 5;
pub const pixman_filter_t_PIXMAN_FILTER_SEPARABLE_CONVOLUTION: pixman_filter_t = 6;

pub const pixman_op_t_PIXMAN_OP_CLEAR: pixman_op_t = 0;
pub const pixman_op_t_PIXMAN_OP_SRC: pixman_op_t = 1;
pub const pixman_op_t_PIXMAN_OP_DST: pixman_op_t = 2;
pub const pixman_op_t_PIXMAN_OP_OVER: pixman_op_t = 3;
pub const pixman_op_t_PIXMAN_OP_OVER_REVERSE: pixman_op_t = 4;
pub const pixman_op_t_PIXMAN_OP_IN: pixman_op_t = 5;
pub const pixman_op_t_PIXMAN_OP_IN_REVERSE: pixman_op_t = 6;
pub const pixman_op_t_PIXMAN_OP_OUT: pixman_op_t = 7;
pub const pixman_op_t_PIXMAN_OP_OUT_REVERSE: pixman_op_t = 8;
pub const pixman_op_t_PIXMAN_OP_ATOP: pixman_op_t = 9;
pub const pixman_op_t_PIXMAN_OP_ATOP_REVERSE: pixman_op_t = 10;
pub const pixman_op_t_PIXMAN_OP_XOR: pixman_op_t = 11;
pub const pixman_op_t_PIXMAN_OP_ADD: pixman_op_t = 12;
pub const pixman_op_t_PIXMAN_OP_SATURATE: pixman_op_t = 13;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_CLEAR: pixman_op_t = 16;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_SRC: pixman_op_t = 17;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_DST: pixman_op_t = 18;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_OVER: pixman_op_t = 19;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_OVER_REVERSE: pixman_op_t = 20;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_IN: pixman_op_t = 21;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_IN_REVERSE: pixman_op_t = 22;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_OUT: pixman_op_t = 23;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_OUT_REVERSE: pixman_op_t = 24;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_ATOP: pixman_op_t = 25;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_ATOP_REVERSE: pixman_op_t = 26;
pub const pixman_op_t_PIXMAN_OP_DISJOINT_XOR: pixman_op_t = 27;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_CLEAR: pixman_op_t = 32;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_SRC: pixman_op_t = 33;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_DST: pixman_op_t = 34;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_OVER: pixman_op_t = 35;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_OVER_REVERSE: pixman_op_t = 36;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_IN: pixman_op_t = 37;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_IN_REVERSE: pixman_op_t = 38;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_OUT: pixman_op_t = 39;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_OUT_REVERSE: pixman_op_t = 40;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_ATOP: pixman_op_t = 41;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_ATOP_REVERSE: pixman_op_t = 42;
pub const pixman_op_t_PIXMAN_OP_CONJOINT_XOR: pixman_op_t = 43;
pub const pixman_op_t_PIXMAN_OP_MULTIPLY: pixman_op_t = 48;
pub const pixman_op_t_PIXMAN_OP_SCREEN: pixman_op_t = 49;
pub const pixman_op_t_PIXMAN_OP_OVERLAY: pixman_op_t = 50;
pub const pixman_op_t_PIXMAN_OP_DARKEN: pixman_op_t = 51;
pub const pixman_op_t_PIXMAN_OP_LIGHTEN: pixman_op_t = 52;
pub const pixman_op_t_PIXMAN_OP_COLOR_DODGE: pixman_op_t = 53;
pub const pixman_op_t_PIXMAN_OP_COLOR_BURN: pixman_op_t = 54;
pub const pixman_op_t_PIXMAN_OP_HARD_LIGHT: pixman_op_t = 55;
pub const pixman_op_t_PIXMAN_OP_SOFT_LIGHT: pixman_op_t = 56;
pub const pixman_op_t_PIXMAN_OP_DIFFERENCE: pixman_op_t = 57;
pub const pixman_op_t_PIXMAN_OP_EXCLUSION: pixman_op_t = 58;
pub const pixman_op_t_PIXMAN_OP_HSL_HUE: pixman_op_t = 59;
pub const pixman_op_t_PIXMAN_OP_HSL_SATURATION: pixman_op_t = 60;
pub const pixman_op_t_PIXMAN_OP_HSL_COLOR: pixman_op_t = 61;
pub const pixman_op_t_PIXMAN_OP_HSL_LUMINOSITY: pixman_op_t = 62;

pub const pixman_region_overlap_t_PIXMAN_REGION_OUT: pixman_region_overlap_t = 0;
pub const pixman_region_overlap_t_PIXMAN_REGION_IN: pixman_region_overlap_t = 1;
pub const pixman_region_overlap_t_PIXMAN_REGION_PART: pixman_region_overlap_t = 2;

pub const pixman_format_code_t_PIXMAN_rgba_float: pixman_format_code_t = 281756740;
pub const pixman_format_code_t_PIXMAN_rgb_float: pixman_format_code_t = 214631492;
pub const pixman_format_code_t_PIXMAN_a8r8g8b8: pixman_format_code_t = 537036936;
pub const pixman_format_code_t_PIXMAN_x8r8g8b8: pixman_format_code_t = 537004168;
pub const pixman_format_code_t_PIXMAN_a8b8g8r8: pixman_format_code_t = 537102472;
pub const pixman_format_code_t_PIXMAN_x8b8g8r8: pixman_format_code_t = 537069704;
pub const pixman_format_code_t_PIXMAN_b8g8r8a8: pixman_format_code_t = 537430152;
pub const pixman_format_code_t_PIXMAN_b8g8r8x8: pixman_format_code_t = 537397384;
pub const pixman_format_code_t_PIXMAN_r8g8b8a8: pixman_format_code_t = 537495688;
pub const pixman_format_code_t_PIXMAN_r8g8b8x8: pixman_format_code_t = 537462920;
pub const pixman_format_code_t_PIXMAN_x14r6g6b6: pixman_format_code_t = 537003622;
pub const pixman_format_code_t_PIXMAN_x2r10g10b10: pixman_format_code_t = 537004714;
pub const pixman_format_code_t_PIXMAN_a2r10g10b10: pixman_format_code_t = 537012906;
pub const pixman_format_code_t_PIXMAN_x2b10g10r10: pixman_format_code_t = 537070250;
pub const pixman_format_code_t_PIXMAN_a2b10g10r10: pixman_format_code_t = 537078442;
pub const pixman_format_code_t_PIXMAN_a8r8g8b8_sRGB: pixman_format_code_t = 537561224;
pub const pixman_format_code_t_PIXMAN_r8g8b8: pixman_format_code_t = 402786440;
pub const pixman_format_code_t_PIXMAN_b8g8r8: pixman_format_code_t = 402851976;
pub const pixman_format_code_t_PIXMAN_r5g6b5: pixman_format_code_t = 268567909;
pub const pixman_format_code_t_PIXMAN_b5g6r5: pixman_format_code_t = 268633445;
pub const pixman_format_code_t_PIXMAN_a1r5g5b5: pixman_format_code_t = 268571989;
pub const pixman_format_code_t_PIXMAN_x1r5g5b5: pixman_format_code_t = 268567893;
pub const pixman_format_code_t_PIXMAN_a1b5g5r5: pixman_format_code_t = 268637525;
pub const pixman_format_code_t_PIXMAN_x1b5g5r5: pixman_format_code_t = 268633429;
pub const pixman_format_code_t_PIXMAN_a4r4g4b4: pixman_format_code_t = 268584004;
pub const pixman_format_code_t_PIXMAN_x4r4g4b4: pixman_format_code_t = 268567620;
pub const pixman_format_code_t_PIXMAN_a4b4g4r4: pixman_format_code_t = 268649540;
pub const pixman_format_code_t_PIXMAN_x4b4g4r4: pixman_format_code_t = 268633156;
pub const pixman_format_code_t_PIXMAN_a8: pixman_format_code_t = 134316032;
pub const pixman_format_code_t_PIXMAN_r3g3b2: pixman_format_code_t = 134349618;
pub const pixman_format_code_t_PIXMAN_b2g3r3: pixman_format_code_t = 134415154;
pub const pixman_format_code_t_PIXMAN_a2r2g2b2: pixman_format_code_t = 134357538;
pub const pixman_format_code_t_PIXMAN_a2b2g2r2: pixman_format_code_t = 134423074;
pub const pixman_format_code_t_PIXMAN_c8: pixman_format_code_t = 134479872;
pub const pixman_format_code_t_PIXMAN_g8: pixman_format_code_t = 134545408;
pub const pixman_format_code_t_PIXMAN_x4a4: pixman_format_code_t = 134299648;
pub const pixman_format_code_t_PIXMAN_x4c4: pixman_format_code_t = 134479872;
pub const pixman_format_code_t_PIXMAN_x4g4: pixman_format_code_t = 134545408;
pub const pixman_format_code_t_PIXMAN_a4: pixman_format_code_t = 67190784;
pub const pixman_format_code_t_PIXMAN_r1g2b1: pixman_format_code_t = 67240225;
pub const pixman_format_code_t_PIXMAN_b1g2r1: pixman_format_code_t = 67305761;
pub const pixman_format_code_t_PIXMAN_a1r1g1b1: pixman_format_code_t = 67244305;
pub const pixman_format_code_t_PIXMAN_a1b1g1r1: pixman_format_code_t = 67309841;
pub const pixman_format_code_t_PIXMAN_c4: pixman_format_code_t = 67371008;
pub const pixman_format_code_t_PIXMAN_g4: pixman_format_code_t = 67436544;
pub const pixman_format_code_t_PIXMAN_a1: pixman_format_code_t = 16846848;
pub const pixman_format_code_t_PIXMAN_g1: pixman_format_code_t = 17104896;
pub const pixman_format_code_t_PIXMAN_yuy2: pixman_format_code_t = 268828672;
pub const pixman_format_code_t_PIXMAN_yv12: pixman_format_code_t = 201785344;

pub const pixman_kernel_t_PIXMAN_KERNEL_IMPULSE: pixman_kernel_t = 0;
pub const pixman_kernel_t_PIXMAN_KERNEL_BOX: pixman_kernel_t = 1;
pub const pixman_kernel_t_PIXMAN_KERNEL_LINEAR: pixman_kernel_t = 2;
pub const pixman_kernel_t_PIXMAN_KERNEL_CUBIC: pixman_kernel_t = 3;
pub const pixman_kernel_t_PIXMAN_KERNEL_GAUSSIAN: pixman_kernel_t = 4;
pub const pixman_kernel_t_PIXMAN_KERNEL_LANCZOS2: pixman_kernel_t = 5;
pub const pixman_kernel_t_PIXMAN_KERNEL_LANCZOS3: pixman_kernel_t = 6;
pub const pixman_kernel_t_PIXMAN_KERNEL_LANCZOS3_STRETCHED: pixman_kernel_t = 7;

pub type pixman_bool_t = ::std::os::raw::c_int;
pub type pixman_fixed_32_32_t = i64;
pub type pixman_fixed_48_16_t = pixman_fixed_32_32_t;
pub type pixman_fixed_1_31_t = u32;
pub type pixman_fixed_1_16_t = u32;
pub type pixman_fixed_16_16_t = i32;
pub type pixman_fixed_t = pixman_fixed_16_16_t;
pub type pixman_color_t = pixman_color;
pub type pixman_point_fixed_t = pixman_point_fixed;
pub type pixman_line_fixed_t = pixman_line_fixed;
pub type pixman_vector_t = pixman_vector;
pub type pixman_transform_t = pixman_transform;
pub type pixman_image_t = pixman_image;
pub type pixman_f_transform_t = pixman_f_transform;
pub type pixman_f_vector_t = pixman_f_vector;
pub type pixman_repeat_t = ::std::os::raw::c_uint;
pub type pixman_dither_t = ::std::os::raw::c_uint;
pub type pixman_filter_t = ::std::os::raw::c_uint;
pub type pixman_op_t = ::std::os::raw::c_uint;
pub type pixman_region16_data_t = pixman_region16_data;
pub type pixman_box16_t = pixman_box16;
pub type pixman_rectangle16_t = pixman_rectangle16;
pub type pixman_region16_t = pixman_region16;
pub type pixman_region_overlap_t = ::std::os::raw::c_uint;
pub type pixman_region32_data_t = pixman_region32_data;
pub type pixman_box32_t = pixman_box32;
pub type pixman_rectangle32_t = pixman_rectangle32;
pub type pixman_region32_t = pixman_region32;
pub type pixman_indexed_t = pixman_indexed;
pub type pixman_gradient_stop_t = pixman_gradient_stop;
pub type pixman_index_type = u8;
pub type pixman_format_code_t = ::std::os::raw::c_uint;
pub type pixman_kernel_t = ::std::os::raw::c_uint;
pub type pixman_edge_t = pixman_edge;
pub type pixman_trapezoid_t = pixman_trapezoid;
pub type pixman_trap_t = pixman_trap;
pub type pixman_span_fix_t = pixman_span_fix;
pub type pixman_triangle_t = pixman_triangle;

pub type pixman_read_memory_func_t = ::std::option::Option<
    unsafe extern "C" fn(src: *const ::std::os::raw::c_void, size: ::std::os::raw::c_int) -> u32,
>;
pub type pixman_write_memory_func_t = ::std::option::Option<
    unsafe extern "C" fn(dst: *mut ::std::os::raw::c_void, value: u32, size: ::std::os::raw::c_int),
>;
pub type pixman_image_destroy_func_t = ::std::option::Option<
    unsafe extern "C" fn(image: *mut pixman_image_t, data: *mut ::std::os::raw::c_void),
>;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_color {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_point_fixed {
    pub x: pixman_fixed_t,
    pub y: pixman_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_line_fixed {
    pub p1: pixman_point_fixed_t,
    pub p2: pixman_point_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_vector {
    pub vector: [pixman_fixed_t; 3usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_transform {
    pub matrix: [[pixman_fixed_t; 3usize]; 3usize],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct pixman_image {
    _unused: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_f_vector {
    pub v: [f64; 3usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_f_transform {
    pub m: [[f64; 3usize]; 3usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_region16_data {
    pub size: ::std::os::raw::c_long,
    pub numRects: ::std::os::raw::c_long,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_rectangle16 {
    pub x: i16,
    pub y: i16,
    pub width: u16,
    pub height: u16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_box16 {
    pub x1: i16,
    pub y1: i16,
    pub x2: i16,
    pub y2: i16,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_region16 {
    pub extents: pixman_box16_t,
    pub data: *mut pixman_region16_data_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_region32_data {
    pub size: ::std::os::raw::c_long,
    pub numRects: ::std::os::raw::c_long,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_rectangle32 {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_box32 {
    pub x1: i32,
    pub y1: i32,
    pub x2: i32,
    pub y2: i32,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_region32 {
    pub extents: pixman_box32_t,
    pub data: *mut pixman_region32_data_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_gradient_stop {
    pub x: pixman_fixed_t,
    pub color: pixman_color_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_indexed {
    pub color: pixman_bool_t,
    pub rgba: [u32; 256usize],
    pub ent: [pixman_index_type; 32768usize],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_glyph_cache_t {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_glyph_t {
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
    pub glyph: *const ::std::os::raw::c_void,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_edge {
    pub x: pixman_fixed_t,
    pub e: pixman_fixed_t,
    pub stepx: pixman_fixed_t,
    pub signdx: pixman_fixed_t,
    pub dy: pixman_fixed_t,
    pub dx: pixman_fixed_t,
    pub stepx_small: pixman_fixed_t,
    pub stepx_big: pixman_fixed_t,
    pub dx_small: pixman_fixed_t,
    pub dx_big: pixman_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_trapezoid {
    pub top: pixman_fixed_t,
    pub bottom: pixman_fixed_t,
    pub left: pixman_line_fixed_t,
    pub right: pixman_line_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_triangle {
    pub p1: pixman_point_fixed_t,
    pub p2: pixman_point_fixed_t,
    pub p3: pixman_point_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_span_fix {
    pub l: pixman_fixed_t,
    pub r: pixman_fixed_t,
    pub y: pixman_fixed_t,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct pixman_trap {
    pub top: pixman_span_fix_t,
    pub bot: pixman_span_fix_t,
}

extern "C" {
    pub fn pixman_transform_init_identity(matrix: *mut pixman_transform);
}
extern "C" {
    pub fn pixman_transform_point_3d(
        transform: *const pixman_transform,
        vector: *mut pixman_vector,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_point(
        transform: *const pixman_transform,
        vector: *mut pixman_vector,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_multiply(
        dst: *mut pixman_transform,
        l: *const pixman_transform,
        r: *const pixman_transform,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_init_scale(
        t: *mut pixman_transform,
        sx: pixman_fixed_t,
        sy: pixman_fixed_t,
    );
}
extern "C" {
    pub fn pixman_transform_scale(
        forward: *mut pixman_transform,
        reverse: *mut pixman_transform,
        sx: pixman_fixed_t,
        sy: pixman_fixed_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_init_rotate(
        t: *mut pixman_transform,
        cos: pixman_fixed_t,
        sin: pixman_fixed_t,
    );
}
extern "C" {
    pub fn pixman_transform_rotate(
        forward: *mut pixman_transform,
        reverse: *mut pixman_transform,
        c: pixman_fixed_t,
        s: pixman_fixed_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_init_translate(
        t: *mut pixman_transform,
        tx: pixman_fixed_t,
        ty: pixman_fixed_t,
    );
}
extern "C" {
    pub fn pixman_transform_translate(
        forward: *mut pixman_transform,
        reverse: *mut pixman_transform,
        tx: pixman_fixed_t,
        ty: pixman_fixed_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_bounds(
        matrix: *const pixman_transform,
        b: *mut pixman_box16,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_invert(
        dst: *mut pixman_transform,
        src: *const pixman_transform,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_is_identity(t: *const pixman_transform) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_is_scale(t: *const pixman_transform) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_is_int_translate(t: *const pixman_transform) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_is_inverse(
        a: *const pixman_transform,
        b: *const pixman_transform,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_transform_from_pixman_f_transform(
        t: *mut pixman_transform,
        ft: *const pixman_f_transform,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_from_pixman_transform(
        ft: *mut pixman_f_transform,
        t: *const pixman_transform,
    );
}
extern "C" {
    pub fn pixman_f_transform_invert(
        dst: *mut pixman_f_transform,
        src: *const pixman_f_transform,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_point(
        t: *const pixman_f_transform,
        v: *mut pixman_f_vector,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_point_3d(t: *const pixman_f_transform, v: *mut pixman_f_vector);
}
extern "C" {
    pub fn pixman_f_transform_multiply(
        dst: *mut pixman_f_transform,
        l: *const pixman_f_transform,
        r: *const pixman_f_transform,
    );
}
extern "C" {
    pub fn pixman_f_transform_init_scale(t: *mut pixman_f_transform, sx: f64, sy: f64);
}
extern "C" {
    pub fn pixman_f_transform_scale(
        forward: *mut pixman_f_transform,
        reverse: *mut pixman_f_transform,
        sx: f64,
        sy: f64,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_init_rotate(t: *mut pixman_f_transform, cos: f64, sin: f64);
}
extern "C" {
    pub fn pixman_f_transform_rotate(
        forward: *mut pixman_f_transform,
        reverse: *mut pixman_f_transform,
        c: f64,
        s: f64,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_init_translate(t: *mut pixman_f_transform, tx: f64, ty: f64);
}
extern "C" {
    pub fn pixman_f_transform_translate(
        forward: *mut pixman_f_transform,
        reverse: *mut pixman_f_transform,
        tx: f64,
        ty: f64,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_bounds(
        t: *const pixman_f_transform,
        b: *mut pixman_box16,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_f_transform_init_identity(t: *mut pixman_f_transform);
}
extern "C" {
    pub fn pixman_region_set_static_pointers(
        empty_box: *mut pixman_box16_t,
        empty_data: *mut pixman_region16_data_t,
        broken_data: *mut pixman_region16_data_t,
    );
}
extern "C" {
    pub fn pixman_region_init(region: *mut pixman_region16_t);
}
extern "C" {
    pub fn pixman_region_init_rect(
        region: *mut pixman_region16_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    );
}
extern "C" {
    pub fn pixman_region_init_rects(
        region: *mut pixman_region16_t,
        boxes: *const pixman_box16_t,
        count: ::std::os::raw::c_int,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_init_with_extents(
        region: *mut pixman_region16_t,
        extents: *const pixman_box16_t,
    );
}
extern "C" {
    pub fn pixman_region_init_from_image(
        region: *mut pixman_region16_t,
        image: *mut pixman_image_t,
    );
}
extern "C" {
    pub fn pixman_region_fini(region: *mut pixman_region16_t);
}
extern "C" {
    pub fn pixman_region_translate(
        region: *mut pixman_region16_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn pixman_region_copy(
        dest: *mut pixman_region16_t,
        source: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_intersect(
        new_reg: *mut pixman_region16_t,
        reg1: *const pixman_region16_t,
        reg2: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_union(
        new_reg: *mut pixman_region16_t,
        reg1: *const pixman_region16_t,
        reg2: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_union_rect(
        dest: *mut pixman_region16_t,
        source: *const pixman_region16_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_intersect_rect(
        dest: *mut pixman_region16_t,
        source: *const pixman_region16_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_subtract(
        reg_d: *mut pixman_region16_t,
        reg_m: *const pixman_region16_t,
        reg_s: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_inverse(
        new_reg: *mut pixman_region16_t,
        reg1: *const pixman_region16_t,
        inv_rect: *const pixman_box16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_contains_point(
        region: *const pixman_region16_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        box_: *mut pixman_box16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_contains_rectangle(
        region: *const pixman_region16_t,
        prect: *const pixman_box16_t,
    ) -> pixman_region_overlap_t;
}
extern "C" {
    pub fn pixman_region_not_empty(region: *const pixman_region16_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_extents(region: *const pixman_region16_t) -> *mut pixman_box16_t;
}
extern "C" {
    pub fn pixman_region_n_rects(region: *const pixman_region16_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_region_rectangles(
        region: *const pixman_region16_t,
        n_rects: *mut ::std::os::raw::c_int,
    ) -> *mut pixman_box16_t;
}
extern "C" {
    pub fn pixman_region_equal(
        region1: *const pixman_region16_t,
        region2: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_selfcheck(region: *mut pixman_region16_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region_reset(region: *mut pixman_region16_t, box_: *const pixman_box16_t);
}
extern "C" {
    pub fn pixman_region_clear(region: *mut pixman_region16_t);
}
extern "C" {
    pub fn pixman_region32_init(region: *mut pixman_region32_t);
}
extern "C" {
    pub fn pixman_region32_init_rect(
        region: *mut pixman_region32_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    );
}
extern "C" {
    pub fn pixman_region32_init_rects(
        region: *mut pixman_region32_t,
        boxes: *const pixman_box32_t,
        count: ::std::os::raw::c_int,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_init_with_extents(
        region: *mut pixman_region32_t,
        extents: *const pixman_box32_t,
    );
}
extern "C" {
    pub fn pixman_region32_init_from_image(
        region: *mut pixman_region32_t,
        image: *mut pixman_image_t,
    );
}
extern "C" {
    pub fn pixman_region32_fini(region: *mut pixman_region32_t);
}
extern "C" {
    pub fn pixman_region32_translate(
        region: *mut pixman_region32_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn pixman_region32_copy(
        dest: *mut pixman_region32_t,
        source: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_intersect(
        new_reg: *mut pixman_region32_t,
        reg1: *const pixman_region32_t,
        reg2: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_union(
        new_reg: *mut pixman_region32_t,
        reg1: *const pixman_region32_t,
        reg2: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_intersect_rect(
        dest: *mut pixman_region32_t,
        source: *const pixman_region32_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_union_rect(
        dest: *mut pixman_region32_t,
        source: *const pixman_region32_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_uint,
        height: ::std::os::raw::c_uint,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_subtract(
        reg_d: *mut pixman_region32_t,
        reg_m: *const pixman_region32_t,
        reg_s: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_inverse(
        new_reg: *mut pixman_region32_t,
        reg1: *const pixman_region32_t,
        inv_rect: *const pixman_box32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_contains_point(
        region: *const pixman_region32_t,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        box_: *mut pixman_box32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_contains_rectangle(
        region: *const pixman_region32_t,
        prect: *const pixman_box32_t,
    ) -> pixman_region_overlap_t;
}
extern "C" {
    pub fn pixman_region32_not_empty(region: *const pixman_region32_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_extents(region: *const pixman_region32_t) -> *mut pixman_box32_t;
}
extern "C" {
    pub fn pixman_region32_n_rects(region: *const pixman_region32_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_region32_rectangles(
        region: *const pixman_region32_t,
        n_rects: *mut ::std::os::raw::c_int,
    ) -> *mut pixman_box32_t;
}
extern "C" {
    pub fn pixman_region32_equal(
        region1: *const pixman_region32_t,
        region2: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_selfcheck(region: *mut pixman_region32_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_region32_reset(region: *mut pixman_region32_t, box_: *const pixman_box32_t);
}
extern "C" {
    pub fn pixman_region32_clear(region: *mut pixman_region32_t);
}
extern "C" {
    pub fn pixman_blt(
        src_bits: *mut u32,
        dst_bits: *mut u32,
        src_stride: ::std::os::raw::c_int,
        dst_stride: ::std::os::raw::c_int,
        src_bpp: ::std::os::raw::c_int,
        dst_bpp: ::std::os::raw::c_int,
        src_x: ::std::os::raw::c_int,
        src_y: ::std::os::raw::c_int,
        dest_x: ::std::os::raw::c_int,
        dest_y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_fill(
        bits: *mut u32,
        stride: ::std::os::raw::c_int,
        bpp: ::std::os::raw::c_int,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
        _xor: u32,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_version() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_version_string() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn pixman_format_supported_destination(format: pixman_format_code_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_format_supported_source(format: pixman_format_code_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_create_solid_fill(color: *const pixman_color_t) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_create_linear_gradient(
        p1: *const pixman_point_fixed_t,
        p2: *const pixman_point_fixed_t,
        stops: *const pixman_gradient_stop_t,
        n_stops: ::std::os::raw::c_int,
    ) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_create_radial_gradient(
        inner: *const pixman_point_fixed_t,
        outer: *const pixman_point_fixed_t,
        inner_radius: pixman_fixed_t,
        outer_radius: pixman_fixed_t,
        stops: *const pixman_gradient_stop_t,
        n_stops: ::std::os::raw::c_int,
    ) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_create_conical_gradient(
        center: *const pixman_point_fixed_t,
        angle: pixman_fixed_t,
        stops: *const pixman_gradient_stop_t,
        n_stops: ::std::os::raw::c_int,
    ) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_create_bits(
        format: pixman_format_code_t,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
        bits: *mut u32,
        rowstride_bytes: ::std::os::raw::c_int,
    ) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_create_bits_no_clear(
        format: pixman_format_code_t,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
        bits: *mut u32,
        rowstride_bytes: ::std::os::raw::c_int,
    ) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_ref(image: *mut pixman_image_t) -> *mut pixman_image_t;
}
extern "C" {
    pub fn pixman_image_unref(image: *mut pixman_image_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_destroy_function(
        image: *mut pixman_image_t,
        function: pixman_image_destroy_func_t,
        data: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn pixman_image_get_destroy_data(image: *mut pixman_image_t)
        -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn pixman_image_set_clip_region(
        image: *mut pixman_image_t,
        region: *const pixman_region16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_clip_region32(
        image: *mut pixman_image_t,
        region: *const pixman_region32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_has_client_clip(image: *mut pixman_image_t, clien_clip: pixman_bool_t);
}
extern "C" {
    pub fn pixman_image_set_transform(
        image: *mut pixman_image_t,
        transform: *const pixman_transform_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_repeat(image: *mut pixman_image_t, repeat: pixman_repeat_t);
}
extern "C" {
    pub fn pixman_image_set_dither(image: *mut pixman_image_t, dither: pixman_dither_t);
}
extern "C" {
    pub fn pixman_image_set_dither_offset(
        image: *mut pixman_image_t,
        offset_x: ::std::os::raw::c_int,
        offset_y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn pixman_image_set_filter(
        image: *mut pixman_image_t,
        filter: pixman_filter_t,
        filter_params: *const pixman_fixed_t,
        n_filter_params: ::std::os::raw::c_int,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_source_clipping(
        image: *mut pixman_image_t,
        source_clipping: pixman_bool_t,
    );
}
extern "C" {
    pub fn pixman_image_set_alpha_map(
        image: *mut pixman_image_t,
        alpha_map: *mut pixman_image_t,
        x: i16,
        y: i16,
    );
}
extern "C" {
    pub fn pixman_image_set_component_alpha(
        image: *mut pixman_image_t,
        component_alpha: pixman_bool_t,
    );
}
extern "C" {
    pub fn pixman_image_get_component_alpha(image: *mut pixman_image_t) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_set_accessors(
        image: *mut pixman_image_t,
        read_func: pixman_read_memory_func_t,
        write_func: pixman_write_memory_func_t,
    );
}
extern "C" {
    pub fn pixman_image_set_indexed(image: *mut pixman_image_t, indexed: *const pixman_indexed_t);
}
extern "C" {
    pub fn pixman_image_get_data(image: *mut pixman_image_t) -> *mut u32;
}
extern "C" {
    pub fn pixman_image_get_width(image: *mut pixman_image_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_image_get_height(image: *mut pixman_image_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_image_get_stride(image: *mut pixman_image_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_image_get_depth(image: *mut pixman_image_t) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn pixman_image_get_format(image: *mut pixman_image_t) -> pixman_format_code_t;
}
extern "C" {
    pub fn pixman_filter_create_separable_convolution(
        n_values: *mut ::std::os::raw::c_int,
        scale_x: pixman_fixed_t,
        scale_y: pixman_fixed_t,
        reconstruct_x: pixman_kernel_t,
        reconstruct_y: pixman_kernel_t,
        sample_x: pixman_kernel_t,
        sample_y: pixman_kernel_t,
        subsample_bits_x: ::std::os::raw::c_int,
        subsample_bits_y: ::std::os::raw::c_int,
    ) -> *mut pixman_fixed_t;
}
extern "C" {
    pub fn pixman_image_fill_rectangles(
        op: pixman_op_t,
        image: *mut pixman_image_t,
        color: *const pixman_color_t,
        n_rects: ::std::os::raw::c_int,
        rects: *const pixman_rectangle16_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_fill_boxes(
        op: pixman_op_t,
        dest: *mut pixman_image_t,
        color: *const pixman_color_t,
        n_boxes: ::std::os::raw::c_int,
        boxes: *const pixman_box32_t,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_compute_composite_region(
        region: *mut pixman_region16_t,
        src_image: *mut pixman_image_t,
        mask_image: *mut pixman_image_t,
        dest_image: *mut pixman_image_t,
        src_x: i16,
        src_y: i16,
        mask_x: i16,
        mask_y: i16,
        dest_x: i16,
        dest_y: i16,
        width: u16,
        height: u16,
    ) -> pixman_bool_t;
}
extern "C" {
    pub fn pixman_image_composite(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        mask: *mut pixman_image_t,
        dest: *mut pixman_image_t,
        src_x: i16,
        src_y: i16,
        mask_x: i16,
        mask_y: i16,
        dest_x: i16,
        dest_y: i16,
        width: u16,
        height: u16,
    );
}
extern "C" {
    pub fn pixman_image_composite32(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        mask: *mut pixman_image_t,
        dest: *mut pixman_image_t,
        src_x: i32,
        src_y: i32,
        mask_x: i32,
        mask_y: i32,
        dest_x: i32,
        dest_y: i32,
        width: i32,
        height: i32,
    );
}
extern "C" {
    pub fn pixman_disable_out_of_bounds_workaround();
}
extern "C" {
    pub fn pixman_glyph_cache_create() -> *mut pixman_glyph_cache_t;
}
extern "C" {
    pub fn pixman_glyph_cache_destroy(cache: *mut pixman_glyph_cache_t);
}
extern "C" {
    pub fn pixman_glyph_cache_freeze(cache: *mut pixman_glyph_cache_t);
}
extern "C" {
    pub fn pixman_glyph_cache_thaw(cache: *mut pixman_glyph_cache_t);
}
extern "C" {
    pub fn pixman_glyph_cache_lookup(
        cache: *mut pixman_glyph_cache_t,
        font_key: *mut ::std::os::raw::c_void,
        glyph_key: *mut ::std::os::raw::c_void,
    ) -> *const ::std::os::raw::c_void;
}
extern "C" {
    pub fn pixman_glyph_cache_insert(
        cache: *mut pixman_glyph_cache_t,
        font_key: *mut ::std::os::raw::c_void,
        glyph_key: *mut ::std::os::raw::c_void,
        origin_x: ::std::os::raw::c_int,
        origin_y: ::std::os::raw::c_int,
        glyph_image: *mut pixman_image_t,
    ) -> *const ::std::os::raw::c_void;
}
extern "C" {
    pub fn pixman_glyph_cache_remove(
        cache: *mut pixman_glyph_cache_t,
        font_key: *mut ::std::os::raw::c_void,
        glyph_key: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn pixman_glyph_get_extents(
        cache: *mut pixman_glyph_cache_t,
        n_glyphs: ::std::os::raw::c_int,
        glyphs: *mut pixman_glyph_t,
        extents: *mut pixman_box32_t,
    );
}
extern "C" {
    pub fn pixman_glyph_get_mask_format(
        cache: *mut pixman_glyph_cache_t,
        n_glyphs: ::std::os::raw::c_int,
        glyphs: *const pixman_glyph_t,
    ) -> pixman_format_code_t;
}
extern "C" {
    pub fn pixman_composite_glyphs(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        dest: *mut pixman_image_t,
        mask_format: pixman_format_code_t,
        src_x: i32,
        src_y: i32,
        mask_x: i32,
        mask_y: i32,
        dest_x: i32,
        dest_y: i32,
        width: i32,
        height: i32,
        cache: *mut pixman_glyph_cache_t,
        n_glyphs: ::std::os::raw::c_int,
        glyphs: *const pixman_glyph_t,
    );
}
extern "C" {
    pub fn pixman_composite_glyphs_no_mask(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        dest: *mut pixman_image_t,
        src_x: i32,
        src_y: i32,
        dest_x: i32,
        dest_y: i32,
        cache: *mut pixman_glyph_cache_t,
        n_glyphs: ::std::os::raw::c_int,
        glyphs: *const pixman_glyph_t,
    );
}
extern "C" {
    pub fn pixman_sample_ceil_y(y: pixman_fixed_t, bpp: ::std::os::raw::c_int) -> pixman_fixed_t;
}
extern "C" {
    pub fn pixman_sample_floor_y(y: pixman_fixed_t, bpp: ::std::os::raw::c_int) -> pixman_fixed_t;
}
extern "C" {
    pub fn pixman_edge_step(e: *mut pixman_edge_t, n: ::std::os::raw::c_int);
}
extern "C" {
    pub fn pixman_edge_init(
        e: *mut pixman_edge_t,
        bpp: ::std::os::raw::c_int,
        y_start: pixman_fixed_t,
        x_top: pixman_fixed_t,
        y_top: pixman_fixed_t,
        x_bot: pixman_fixed_t,
        y_bot: pixman_fixed_t,
    );
}
extern "C" {
    pub fn pixman_line_fixed_edge_init(
        e: *mut pixman_edge_t,
        bpp: ::std::os::raw::c_int,
        y: pixman_fixed_t,
        line: *const pixman_line_fixed_t,
        x_off: ::std::os::raw::c_int,
        y_off: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn pixman_rasterize_edges(
        image: *mut pixman_image_t,
        l: *mut pixman_edge_t,
        r: *mut pixman_edge_t,
        t: pixman_fixed_t,
        b: pixman_fixed_t,
    );
}
extern "C" {
    pub fn pixman_add_traps(
        image: *mut pixman_image_t,
        x_off: i16,
        y_off: i16,
        ntrap: ::std::os::raw::c_int,
        traps: *const pixman_trap_t,
    );
}
extern "C" {
    pub fn pixman_add_trapezoids(
        image: *mut pixman_image_t,
        x_off: i16,
        y_off: ::std::os::raw::c_int,
        ntraps: ::std::os::raw::c_int,
        traps: *const pixman_trapezoid_t,
    );
}
extern "C" {
    pub fn pixman_rasterize_trapezoid(
        image: *mut pixman_image_t,
        trap: *const pixman_trapezoid_t,
        x_off: ::std::os::raw::c_int,
        y_off: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn pixman_composite_trapezoids(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        dst: *mut pixman_image_t,
        mask_format: pixman_format_code_t,
        x_src: ::std::os::raw::c_int,
        y_src: ::std::os::raw::c_int,
        x_dst: ::std::os::raw::c_int,
        y_dst: ::std::os::raw::c_int,
        n_traps: ::std::os::raw::c_int,
        traps: *const pixman_trapezoid_t,
    );
}
extern "C" {
    pub fn pixman_composite_triangles(
        op: pixman_op_t,
        src: *mut pixman_image_t,
        dst: *mut pixman_image_t,
        mask_format: pixman_format_code_t,
        x_src: ::std::os::raw::c_int,
        y_src: ::std::os::raw::c_int,
        x_dst: ::std::os::raw::c_int,
        y_dst: ::std::os::raw::c_int,
        n_tris: ::std::os::raw::c_int,
        tris: *const pixman_triangle_t,
    );
}
extern "C" {
    pub fn pixman_add_triangles(
        image: *mut pixman_image_t,
        x_off: i32,
        y_off: i32,
        n_tris: ::std::os::raw::c_int,
        tris: *const pixman_triangle_t,
    );
}
