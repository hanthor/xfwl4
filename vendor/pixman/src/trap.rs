use crate::{ffi, Span};

/// A single trap
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Trap(ffi::pixman_trap_t);

impl Trap {
    /// Initialize the trap from the provided values
    #[inline]
    pub fn new(top: impl Into<Span>, bot: impl Into<Span>) -> Self {
        Self(ffi::pixman_trap_t {
            top: top.into().into(),
            bot: bot.into().into(),
        })
    }

    /// Access the top value of this trap
    #[inline]
    pub fn top(&self) -> Span {
        Span::from(self.0.top)
    }

    /// Access the bot value of this trap
    #[inline]
    pub fn bot(&self) -> Span {
        Span::from(self.0.bot)
    }
}

impl From<ffi::pixman_trap_t> for Trap {
    #[inline]
    fn from(value: ffi::pixman_trap_t) -> Self {
        Self(value)
    }
}

impl From<Trap> for ffi::pixman_trap_t {
    #[inline]
    fn from(value: Trap) -> Self {
        value.0
    }
}
