# libseat-rs

```rust
let seat = Seat::open(
    |seat, event| match event {
        SeatEvent::Enable => {
            println!("Enable");
            println!("Name: {}", seat.name());
        }
        SeatEvent::Disable => {
            println!("Disable");
            seat.disable().unwrap();
        }
    },
    None,
)
```
