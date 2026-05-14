use pixman_sys as ffi;

/// Rgba color in the range of [`u16::MIN`] to [`u16::MAX`]
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Color(ffi::pixman_color_t);

impl Color {
    /// Create a [`Color`] from the provided components
    ///
    /// Note: Color component range of [`u16::MIN`] to [`u16::MAX`]
    #[inline]
    pub fn new(r: u16, g: u16, b: u16, a: u16) -> Self {
        Self(ffi::pixman_color_t {
            red: r,
            green: g,
            blue: b,
            alpha: a,
        })
    }

    /// Create a [`Color`] from the provided components
    ///
    /// Note: Color component range of `0f32` to `1f32`
    #[inline]
    pub fn from_f32(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self::from_f64(r as f64, g as f64, b as f64, a as f64)
    }

    /// Create a [`Color`] from the provided components
    ///
    /// Note: Color component range of `0f64` to `1f64`
    #[inline]
    pub fn from_f64(r: f64, g: f64, b: f64, a: f64) -> Self {
        Self(ffi::pixman_color_t {
            red: double_to_color(r),
            green: double_to_color(g),
            blue: double_to_color(b),
            alpha: double_to_color(a),
        })
    }

    /// Create a [`Color`] from the provided color value
    #[inline]
    pub fn from_u32(color8: u32) -> Self {
        let alpha = (color8 & 0xff000000) >> 24;
        let red = (color8 & 0x00ff0000) >> 16;
        let green = (color8 & 0x0000ff00) >> 8;
        let blue = color8 & 0x000000ff;

        let alpha = alpha | alpha << 8;
        let red = red | red << 8;
        let green = green | green << 8;
        let blue = blue | blue << 8;

        Self::new(red as u16, green as u16, blue as u16, alpha as u16)
    }

    /// Get the red color component
    #[inline]
    pub fn r(&self) -> u16 {
        self.0.red
    }

    /// Get the green color component
    #[inline]
    pub fn g(&self) -> u16 {
        self.0.green
    }

    /// Get the blue color component
    #[inline]
    pub fn b(&self) -> u16 {
        self.0.blue
    }

    /// Get the alpha color component
    #[inline]
    pub fn a(&self) -> u16 {
        self.0.alpha
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *const ffi::pixman_color_t {
        &self.0 as *const _
    }
}

impl From<ffi::pixman_color_t> for Color {
    #[inline]
    fn from(value: ffi::pixman_color_t) -> Self {
        Self(value)
    }
}

impl From<Color> for ffi::pixman_color_t {
    #[inline]
    fn from(value: Color) -> Self {
        value.0
    }
}

impl From<[u16; 4]> for Color {
    #[inline]
    fn from(value: [u16; 4]) -> Self {
        Self::new(value[0], value[1], value[2], value[3])
    }
}

impl From<[f32; 4]> for Color {
    #[inline]
    fn from(value: [f32; 4]) -> Self {
        Self::from_f32(value[0], value[1], value[2], value[3])
    }
}

impl From<[f64; 4]> for Color {
    #[inline]
    fn from(value: [f64; 4]) -> Self {
        Self::from_f64(value[0], value[1], value[2], value[3])
    }
}

impl From<u32> for Color {
    #[inline]
    fn from(value: u32) -> Self {
        Self::from_u32(value)
    }
}

#[inline]
fn double_to_color(x: f64) -> u16 {
    (((x * 65536.0) as u32) - (((x * 65536.0) as u32) >> 16)) as u16
}
