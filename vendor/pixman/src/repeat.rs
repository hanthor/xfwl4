use pixman_sys as ffi;
use thiserror::Error;

/// Defines the possible repeat operations
#[derive(Debug, Clone, Copy)]
pub enum Repeat {
    /// No repeat
    None,
    /// Normal repeat
    Normal,
    /// Pad repeat
    Pad,
    /// Reflect repeat
    Reflect,
}

/// The repeat operations is unknown
#[derive(Debug, Error)]
#[error("Unknown repeat {0}")]
pub struct UnknownRepeat(ffi::pixman_repeat_t);

impl TryFrom<ffi::pixman_repeat_t> for Repeat {
    type Error = UnknownRepeat;

    fn try_from(value: ffi::pixman_repeat_t) -> Result<Self, Self::Error> {
        let repeat = match value {
            ffi::pixman_repeat_t_PIXMAN_REPEAT_NONE => Repeat::None,
            ffi::pixman_repeat_t_PIXMAN_REPEAT_NORMAL => Repeat::Normal,
            ffi::pixman_repeat_t_PIXMAN_REPEAT_PAD => Repeat::Pad,
            ffi::pixman_repeat_t_PIXMAN_REPEAT_REFLECT => Repeat::Reflect,
            _ => return Err(UnknownRepeat(value)),
        };
        Ok(repeat)
    }
}

impl From<Repeat> for ffi::pixman_repeat_t {
    fn from(value: Repeat) -> Self {
        match value {
            Repeat::None => ffi::pixman_repeat_t_PIXMAN_REPEAT_NONE,
            Repeat::Normal => ffi::pixman_repeat_t_PIXMAN_REPEAT_NORMAL,
            Repeat::Pad => ffi::pixman_repeat_t_PIXMAN_REPEAT_PAD,
            Repeat::Reflect => ffi::pixman_repeat_t_PIXMAN_REPEAT_REFLECT,
        }
    }
}
