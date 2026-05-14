use std::{
    ffi::{CStr, CString},
    str::FromStr,
};

use glib::translate::from_glib_full;

use crate::Shortcut;

pub trait ShortcutManualExt {
    fn new(property_name: &str, shortcut: &str, command: &str, startup_notify: bool) -> Self;

    fn property_name(&self) -> &str;
    fn shortcut(&self) -> &str;
    fn command(&self) -> &str;
    fn snotify(&self) -> bool;
}

impl ShortcutManualExt for Shortcut {
    fn new(property_name: &str, shortcut: &str, command: &str, startup_notify: bool) -> Self {
        let ptr = unsafe { glib::ffi::g_slice_alloc0(std::mem::size_of::<ffi::XfceShortcut>()) }
            as *mut ffi::XfceShortcut;

        // SAFETY: glib abort()s if memory allocation fails, 'ptr' is valid.
        // SAFETY: CString always gives us valid pointers; passing them to g_strdup() is safe.
        unsafe {
            // We copy the strings with g_strdup() because xfce_shortcut_free() uses g_free().
            (*ptr).property_name =
                glib::ffi::g_strdup(CString::new(property_name).unwrap().as_ptr());
            (*ptr).shortcut = glib::ffi::g_strdup(CString::new(shortcut).unwrap().as_ptr());
            (*ptr).command = glib::ffi::g_strdup(CString::new(command).unwrap().as_ptr());
            (*ptr).snotify = if startup_notify { 1 } else { 0 };
        }

        // 'ptr' is a valid boxed type
        unsafe { from_glib_full(ptr) }
    }

    fn property_name(&self) -> &str {
        // SAFETY: 'self.as_ptr()' is guaranteed to return a non-NULL pointer.
        let ptr = unsafe { (*self.as_ptr()).property_name };
        // SAFETY: the libxfce4kbd-private code never returns a shortcut with a NULL property_name.
        unsafe { CStr::from_ptr(ptr).to_str().unwrap() }
    }

    fn shortcut(&self) -> &str {
        // SAFETY: 'self.as_ptr()' is guaranteed to return a non-NULL pointer.
        let ptr = unsafe { (*self.as_ptr()).shortcut };
        // SAFETY: the libxfce4kbd-private code never returns a shortcut with a NULL shortcut.
        unsafe { CStr::from_ptr(ptr).to_str().unwrap() }
    }

    fn command(&self) -> &str {
        // SAFETY: 'self.as_ptr()' is guaranteed to return a non-NULL pointer.
        let ptr = unsafe { (*self.as_ptr()).command };
        // SAFETY: the libxfce4kbd-private code never returns a shortcut with a NULL command.
        unsafe { CStr::from_ptr(ptr).to_str().unwrap() }
    }

    fn snotify(&self) -> bool {
        // SAFETY: 'self.as_ptr()' is guaranteed to return a non-NULL pointer.
        unsafe { (*self.as_ptr()).snotify != 0 }
    }
}
