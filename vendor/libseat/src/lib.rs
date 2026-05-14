use libseat_sys as sys;

use errno::{errno, Errno};

use std::{
    ffi::CString,
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    os::unix::io::{AsFd, BorrowedFd, RawFd},
    path::Path,
    ptr::NonNull,
};

mod ffi_seat_listener;
use ffi_seat_listener::FFI_SEAT_LISTENER;

mod log;
pub use self::log::*;

#[cfg(feature = "custom_logger")]
mod log_handler;

#[derive(Debug, Clone, Copy)]
pub enum SeatEvent {
    Enable,
    Disable,
}

type SeatListenerCallback = dyn FnMut(&mut SeatRef, SeatEvent);

struct SeatListener {
    callback: Box<SeatListenerCallback>,
}

impl std::fmt::Debug for SeatListener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserSeatListener").finish()
    }
}

#[derive(Debug)]
pub struct Seat {
    inner: SeatRef,
    _seat_listener: Box<SeatListener>,
}

impl Seat {
    /// Opens a seat, taking control of it if possible and returning a pointer to
    /// the libseat instance. If LIBSEAT_BACKEND is set, the specified backend is
    /// used. Otherwise, the first successful backend will be used.
    pub fn open<C>(callback: C) -> Result<Self, Errno>
    where
        C: FnMut(&mut SeatRef, SeatEvent) + 'static,
    {
        #[cfg(feature = "custom_logger")]
        log_handler::init();

        let user_listener = SeatListener {
            callback: Box::new(callback),
        };

        let mut user_data = Box::new(user_listener);

        let seat = unsafe {
            sys::libseat_open_seat(&FFI_SEAT_LISTENER, user_data.as_mut() as *mut _ as _)
        };

        NonNull::new(seat)
            .map(|nn| Self {
                inner: SeatRef(nn),
                _seat_listener: user_data,
            })
            .ok_or_else(errno)
    }
}

impl Drop for Seat {
    fn drop(&mut self) {
        // Closes the seat. This frees the libseat structure.
        unsafe { sys::libseat_close_seat(self.0.as_mut()) };
    }
}

impl Deref for Seat {
    type Target = SeatRef;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Seat {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[derive(Debug)]
pub struct SeatRef(NonNull<sys::libseat>);

impl SeatRef {
    /// Disables a seat, used in response to a disable_seat event. After disabling
    /// the seat, the seat devices must not be used until enable_seat is received,
    /// and all requests on the seat will fail during this period.
    pub fn disable(&mut self) -> Result<(), Errno> {
        if unsafe { sys::libseat_disable_seat(self.0.as_mut()) } == 0 {
            Ok(())
        } else {
            Err(errno())
        }
    }

    /// Opens a device on the seat, returning its device ID and fd
    ///
    /// This will only succeed if the seat is active and the device is of a type
    /// permitted for opening on the backend, such as drm and evdev.
    ///
    /// The device may be revoked in some situations, such as in situations where a
    /// seat session switch is being forced.
    pub fn open_device<P: AsRef<Path>>(&mut self, path: &P) -> Result<Device, Errno> {
        let path = path.as_ref();
        let string = path.as_os_str().to_str().unwrap();
        let cstring = CString::new(string).unwrap();

        let mut fd = MaybeUninit::uninit();
        let id =
            unsafe { sys::libseat_open_device(self.0.as_mut(), cstring.as_ptr(), fd.as_mut_ptr()) };

        if id != -1 {
            Ok(Device {
                id,
                fd: unsafe { fd.assume_init() },
            })
        } else {
            Err(errno())
        }
    }

    /// Closes a device that has been opened on the seat using the device_id from
    /// libseat_open_device.
    pub fn close_device(&mut self, device: Device) -> Result<(), Errno> {
        if unsafe { sys::libseat_close_device(self.0.as_mut(), device.id) } == 0 {
            Ok(())
        } else {
            Err(errno())
        }
    }

    /// Retrieves the name of the seat that is currently made available through the
    /// provided libseat instance.
    pub fn name(&mut self) -> &str {
        unsafe {
            let cstr = sys::libseat_seat_name(self.0.as_mut());
            let cstr = std::ffi::CStr::from_ptr(cstr as *const _);
            cstr.to_str().unwrap()
        }
    }

    /// Requests that the seat switches session to the specified session number.
    /// For seats that are VT-bound, the session number matches the VT number, and
    /// switching session results in a VT switch.
    ///
    /// A call to libseat_switch_session does not imply that a switch will occur,
    /// and the caller should assume that the session continues unaffected.
    pub fn switch_session(&mut self, session: i32) -> Result<(), Errno> {
        if unsafe { sys::libseat_switch_session(self.0.as_mut(), session) } == 0 {
            Ok(())
        } else {
            Err(errno())
        }
    }

    /// Retrieve the pollable connection fd for a given libseat instance. Used to
    /// poll the libseat connection for events that need to be dispatched.
    ///
    /// Returns a pollable fd on success.
    pub fn get_fd(&mut self) -> Result<BorrowedFd<'_>, Errno> {
        let fd = unsafe { sys::libseat_get_fd(self.0.as_mut()) };
        if fd == -1 {
            Err(errno())
        } else {
            Ok(unsafe { BorrowedFd::borrow_raw(fd) })
        }
    }

    /// Reads and dispatches events on the libseat connection fd.
    ///
    /// The specified timeout dictates how long libseat might wait for data if none
    /// is available: 0 means that no wait will occur, -1 means that libseat might
    /// wait indefinitely for data to arrive, while > 0 is the maximum wait in
    /// milliseconds that might occur.
    ///
    /// Returns a positive number signifying processed internal messages on success.
    /// Returns 0 if no messages were processed. Returns -1 and sets errno on error.
    pub fn dispatch(&mut self, timeout: i32) -> Result<i32, Errno> {
        let v = unsafe { sys::libseat_dispatch(self.0.as_mut(), timeout) };
        if v == -1 {
            Err(errno())
        } else {
            Ok(v)
        }
    }
}

/// Wrapper around the device id and fd of a libseat device. Opened with
/// [SeatRef::open_device].
///
/// Use `AsFd` to access the file descriptor. The device should be closed
/// with [SeatRef::close_device].
#[derive(Debug)]
#[must_use]
pub struct Device {
    id: i32,
    fd: RawFd,
}

impl AsFd for Device {
    fn as_fd(&self) -> BorrowedFd<'_> {
        unsafe { BorrowedFd::borrow_raw(self.fd) }
    }
}
