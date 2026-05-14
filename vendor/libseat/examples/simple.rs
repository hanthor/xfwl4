use libseat::{Seat, SeatEvent};

use std::{cell::RefCell, rc::Rc};

fn main() {
    stderrlog::new().verbosity(log::LevelFilter::Trace).init().unwrap();

    let active = Rc::new(RefCell::new(false));

    let seat = {
        let active = active.clone();
        Seat::open(move |seat, event| match event {
            SeatEvent::Enable => {
                println!("Enable");
                println!("Name: {}", seat.name());

                *active.borrow_mut() = true;
            }
            SeatEvent::Disable => {
                println!("Disable");

                *active.borrow_mut() = false;
                seat.disable().unwrap();
            }
        })
    };

    if let Ok(mut seat) = seat {
        while !(*active.borrow()) {
            println!("waiting for activation...n");
            seat.dispatch(-1).unwrap();
        }

        // Close seat
        drop(seat);
    }
}
