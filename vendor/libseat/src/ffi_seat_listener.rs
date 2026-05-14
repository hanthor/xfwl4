use libseat_sys as sys;

use crate::{SeatEvent, SeatListener, SeatRef};
use std::ptr::NonNull;

/// The seat has been enabled, and is now valid for use. Re-open all seat
/// devices to ensure that they are operational, as existing fds may have
/// had their functionality blocked or revoked.
extern "C" fn enable_seat(seat: *mut sys::libseat, data: *mut std::os::raw::c_void) {
    let data = data as *mut SeatListener;
    let data = unsafe { &mut *data };

    let mut seat = unsafe { SeatRef(NonNull::new_unchecked(seat)) };
    (data.callback)(&mut seat, SeatEvent::Enable);
}

/// The seat has been disabled. This event signals that the application
/// is going to lose its seat access. The event *must* be acknowledged
/// with libseat_disable_seat shortly after receiving this event.
///
/// If the recepient fails to acknowledge the event in time, seat devices
/// may be forcibly revoked by the seat provider.
extern "C" fn disable_seat(seat: *mut sys::libseat, data: *mut std::os::raw::c_void) {
    let data = data as *mut SeatListener;
    let data = unsafe { &mut *data };

    let mut seat = unsafe { SeatRef(NonNull::new_unchecked(seat)) };
    (data.callback)(&mut seat, SeatEvent::Disable);
}

/// A seat event listener, given to libseat_open_seat.
pub static FFI_SEAT_LISTENER: sys::libseat_seat_listener = sys::libseat_seat_listener {
    enable_seat: Some(enable_seat),
    disable_seat: Some(disable_seat),
};
