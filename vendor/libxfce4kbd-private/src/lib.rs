#![cfg_attr(docsrs, feature(doc_cfg))]
#![allow(unused_imports)]

mod auto;
#[cfg(feature = "v4_21_4")]
mod shortcut;

pub use auto::traits::*;
pub use auto::*;
pub use ffi;
#[cfg(feature = "v4_21_4")]
pub use shortcut::*;
