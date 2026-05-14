// rustdoc-stripper-ignore-next
/// Shuts down and frees any resources consumed by the Xfconf library.
///
/// If [`init()`][crate::init()] is called multiple times, [`shutdown()`][crate::shutdown()] must be
/// called an equal number of times to shut down the library.
///
/// # Safety
///
/// You must ensure that there are no [crate::Channel] instances that are live
pub unsafe fn shutdown() {
    ffi::xfconf_shutdown();
}
