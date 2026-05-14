use pixman_sys as ffi;

/// Fixed-point value
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Fixed(ffi::pixman_fixed_t);

impl Fixed {
    /// One
    pub const ONE: Fixed = Fixed::from_int(1);
    /// Zero
    pub const ZERO: Fixed = Fixed::from_raw(0);

    /// Initialize this fixed-point from a raw value
    #[inline]
    pub const fn from_raw(value: ffi::pixman_fixed_t) -> Self {
        Self(value)
    }

    /// Initialize this fixed-point from an integer
    #[inline]
    pub const fn from_int(value: i32) -> Self {
        Self(((value as u32) << 16) as ffi::pixman_fixed_t)
    }

    /// Initialize this fixed-point from an `f64`
    #[inline]
    pub fn from_double(value: f64) -> Self {
        Self((value * 65536.0) as ffi::pixman_fixed_t)
    }

    /// Get the int value of this fixed point
    #[inline]
    pub const fn to_int(self) -> i32 {
        self.0 >> 16
    }

    /// Access the raw fixed-point value
    #[inline]
    pub fn into_raw(self) -> ffi::pixman_fixed_t {
        self.0
    }
}

impl std::ops::Add for Fixed {
    type Output = Fixed;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl From<i32> for Fixed {
    #[inline]
    fn from(value: i32) -> Self {
        Self::from_int(value)
    }
}

impl From<f32> for Fixed {
    #[inline]
    fn from(value: f32) -> Self {
        Self::from_double(value as f64)
    }
}

impl From<f64> for Fixed {
    #[inline]
    fn from(value: f64) -> Self {
        Self::from_double(value)
    }
}
