use pixman_sys as ffi;
use thiserror::Error;

/// Defines the possible filter operations
#[derive(Debug, Clone, Copy)]
pub enum Filter {
    /// Fast filtering
    Fast,
    /// Good filtering
    Good,
    /// Best filtering
    Best,
    /// Nearest-neighbor filtering
    Nearest,
    /// Bilinear filtering
    Bilinear,
    /// Custom convolution kernel
    Convolution,
    /// Custom separable convolution kernel
    SeparableConvolution,
}

/// The filter operation is unknown
#[derive(Debug, Error)]
#[error("Unknown filter {0}")]
pub struct UnknownFilter(ffi::pixman_dither_t);

impl TryFrom<ffi::pixman_dither_t> for Filter {
    type Error = UnknownFilter;

    fn try_from(value: ffi::pixman_dither_t) -> Result<Self, Self::Error> {
        let repeat = match value {
            ffi::pixman_filter_t_PIXMAN_FILTER_FAST => Filter::Fast,
            ffi::pixman_filter_t_PIXMAN_FILTER_GOOD => Filter::Good,
            ffi::pixman_filter_t_PIXMAN_FILTER_BEST => Filter::Best,
            ffi::pixman_filter_t_PIXMAN_FILTER_NEAREST => Filter::Nearest,
            ffi::pixman_filter_t_PIXMAN_FILTER_BILINEAR => Filter::Bilinear,
            ffi::pixman_filter_t_PIXMAN_FILTER_CONVOLUTION => Filter::Convolution,
            ffi::pixman_filter_t_PIXMAN_FILTER_SEPARABLE_CONVOLUTION => {
                Filter::SeparableConvolution
            }
            _ => return Err(UnknownFilter(value)),
        };
        Ok(repeat)
    }
}

impl From<Filter> for ffi::pixman_dither_t {
    fn from(value: Filter) -> Self {
        match value {
            Filter::Fast => ffi::pixman_filter_t_PIXMAN_FILTER_FAST,
            Filter::Good => ffi::pixman_filter_t_PIXMAN_FILTER_GOOD,
            Filter::Best => ffi::pixman_filter_t_PIXMAN_FILTER_BEST,
            Filter::Nearest => ffi::pixman_filter_t_PIXMAN_FILTER_NEAREST,
            Filter::Bilinear => ffi::pixman_filter_t_PIXMAN_FILTER_BILINEAR,
            Filter::Convolution => ffi::pixman_filter_t_PIXMAN_FILTER_CONVOLUTION,
            Filter::SeparableConvolution => {
                ffi::pixman_filter_t_PIXMAN_FILTER_SEPARABLE_CONVOLUTION
            }
        }
    }
}
