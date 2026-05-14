//! A simple crate for measuring the average, minimum and maximum frame rate over a window of time.
//!
//! ## Usage
//!
//! Specify a window length over which the average, minimum and maximum values will be measured.
//!
//! ```rust
//! use fps_ticker::Fps;
//!
//! fn main() {
//!     let fps = Fps::with_window_len(100);
//! }
//! ```
//!
//! Or create an instance with the default window length of `60`.
//!
//! ```rust
//! # use fps_ticker::Fps;
//! # fn main() {
//! let fps = Fps::default();
//! # }
//! ```
//!
//! Call `tick` once per frame at the point at which you wish to measure the rate.  This samples
//! the duration since the last tick, adds it to the window, removes the oldest duration from the
//! window if necessary and re-calculates the average, minimum and maximum rate over the resulting
//! window.
//!
//! ```rust
//! # use fps_ticker::Fps;
//! # fn main() {
//! # let fps = Fps::default();
//! fps.tick();
//! # }
//! ```
//!
//! Now we can retrieve the average, minimum and maximum rate over the window of time.
//!
//! ```rust
//! # use fps_ticker::Fps;
//! # fn main() {
//! # let fps = Fps::default();
//! # fps.tick();
//! fps.avg();
//! fps.min();
//! fps.max();
//! # }
//! ```

use std::cell::RefCell;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Simple type for tracking frames-per-second.
#[derive(Clone, Debug)]
pub struct Fps {
    window_len: usize,
    inner: RefCell<Inner>,
}

#[derive(Clone, Debug)]
struct Inner {
    window: VecDeque<Duration>,
    last: Instant,
    avg: f64,
    min: f64,
    max: f64,
}

impl Fps {
    /// The window length used by the default constructor.
    pub const DEFAULT_WINDOW_LEN: usize = 60;

    /// Create a new `Fps` with the given window length as a number of frames.
    ///
    /// The larger the window, the "smoother" the FPS.
    pub fn with_window_len(window_len: usize) -> Self {
        let window = VecDeque::with_capacity(window_len);
        let last = Instant::now();
        let (avg, min, max) = (0.0, 0.0, 0.0);
        let inner = RefCell::new(Inner {
            window,
            last,
            avg,
            min,
            max,
        });
        Fps { window_len, inner }
    }

    /// Call this once per frame to allow the `Fps` instance to sample the rate internally.
    pub fn tick(&self) {
        let now = Instant::now();
        let mut inner = self.inner.borrow_mut();
        let delta = now.duration_since(inner.last);
        inner.last = now;
        while inner.window.len() + 1 > self.window_len {
            inner.window.pop_front();
        }
        inner.window.push_back(delta);
        inner.avg = inner.calc_avg();
        inner.min = inner.calc_min();
        inner.max = inner.calc_max();
    }

    /// Retrieve the average frames-per-second at the moment of the last call to `tick`.
    pub fn avg(&self) -> f64 {
        self.inner.borrow().avg
    }

    /// Retrieve the minimum frames-per-second that was reached within the window at the moment
    /// `tick` was last called.
    pub fn min(&self) -> f64 {
        self.inner.borrow().min
    }

    /// Retrieve the maximum frames-per-second that was reached within the window at the moment
    /// `tick` was last called.
    pub fn max(&self) -> f64 {
        self.inner.borrow().max
    }
}

impl Inner {
    /// Calculate the frames per second from the current state of the window.
    fn calc_avg(&self) -> f64 {
        let sum_secs = self.window.iter().map(|d| d.as_secs_f64()).sum::<f64>();
        1.0 / (sum_secs / self.window.len() as f64)
    }

    /// Find the minimum frames per second that occurs over the window.
    fn calc_min(&self) -> f64 {
        1.0 / self
            .window
            .iter()
            .max()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }

    /// Find the minimum frames per second that occurs over the window.
    fn calc_max(&self) -> f64 {
        1.0 / self
            .window
            .iter()
            .min()
            .map(|d| d.as_secs_f64())
            .unwrap_or(0.0)
    }
}

impl Default for Fps {
    fn default() -> Self {
        Fps::with_window_len(Self::DEFAULT_WINDOW_LEN)
    }
}
