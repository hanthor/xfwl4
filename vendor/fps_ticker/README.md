# fps_ticker [![Crates.io](https://img.shields.io/crates/v/fps_ticker.svg)](https://crates.io/crates/fps_ticker) [![docs.rs](https://docs.rs/fps_ticker/badge.svg)](https://docs.rs/fps_ticker/)

A simple crate for measuring the average, minimum and maximum frame rate over a
window of time.

## Usage

Specify a window length over which the average, minimum and maximum values will be measured.

```rust
use fps_ticker::Fps;

fn main() {
    let fps = Fps::with_window_len(100);
}
```

Or create an instance with the default window length of `60`.

```rust
let fps = Fps::default();
```

Call `tick` once per frame at the point at which you wish to measure the rate. This samples
the duration since the last tick, adds it to the window, removes the oldest duration from the
window if necessary and re-calculates the average, minimum and maximum rate over the resulting
window.

```rust
fps.tick();
```

Now we can retrieve the average, minimum and maximum rate over the window of time.

```rust
fps.avg();
fps.min();
fps.max();
```
