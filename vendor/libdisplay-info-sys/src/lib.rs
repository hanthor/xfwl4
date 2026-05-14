#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

#[cfg_attr(docsrs, cfg(feature = "v0_1"))]
#[cfg_attr(
    not(docsrs),
    cfg(all(feature = "v0_1", not(feature = "v0_2"), not(feature = "v0_3")))
)]
pub mod v0_1;

#[cfg_attr(docsrs, cfg(feature = "v0_2"))]
#[cfg_attr(not(docsrs), cfg(all(feature = "v0_2", not(feature = "v0_3"))))]
pub mod v0_2;

#[cfg(feature = "v0_3")]
pub mod v0_3;

#[cfg(feature = "auto")]
pub mod auto;
#[cfg(feature = "auto")]
pub use auto::*;
