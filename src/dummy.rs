//! GPIO dummy input/output
//!
//! The dummy module can be used instead of a GPIO implementation tied to
//! hardware to run unit tests or otherwise provide means to test an
//! application when no embedded device is around.
//!
//! It supports the same interface as other GPIOs and its input and output
//! behaviour can be configured in a flexible manner.
//!
//! ## Example
//!
//! The `DummyGpioIn` reads values from a callback:
//!
//! ```rust
//! use std::time;
//! use gpio::{GpioIn, GpioValue};
//! use gpio::dummy::DummyGpioIn;
//!
//! // a simple dummy gpio that is always `true`/`High`
//! let mut dg = DummyGpioIn::new(|| true);
//! assert_eq!(GpioValue::High, dg.read_value().unwrap());
//!
//! // another example that flips every second
//! let mut timed_gpio = DummyGpioIn::new(|| {
//!     std::time::SystemTime::now()
//!         .duration_since(time::UNIX_EPOCH)
//!         .unwrap()
//!         .as_secs() % 2 == 0
//! });
//! println!("timed: {:?}", timed_gpio.read_value().unwrap());
//! ```
//!
//! Output can simple be swallowed by a dummy output port:
//!
//! ```rust
//! use gpio::{GpioOut};
//! use gpio::dummy::DummyGpioOut;
//!
//! let mut dg = DummyGpioOut::new(|_| ());
//! dg.set_value(true);
//! ```

use std::{sync, thread, time};
use super::{GpioEdge, GpioIn, GpioOut, GpioValue};

/// Dummy GPIO input pin
#[derive(Clone)]
pub struct DummyGpioIn {
    value: sync::Arc<Fn() -> GpioValue>,
    edge: GpioEdge,
}

impl DummyGpioIn {
    /// Create new dummy pin that returns the value of `value` every it is read
    pub fn new<F, V>(value: F) -> DummyGpioIn
    where
        V: Into<GpioValue>,
        F: Fn() -> V + 'static,
    {
        DummyGpioIn {
            value: sync::Arc::new(move || value().into()),
            edge: GpioEdge::None,
        }
    }
}

impl GpioIn for DummyGpioIn {
    type Error = ();

    fn read_value(&self) -> Result<GpioValue, Self::Error> {
        Ok((self.value)())
    }

    fn set_edge(&mut self, edge: GpioEdge) -> Result<(), Self::Error> {
        self.edge = edge;
        Ok(())
    }
}

pub struct DummyEdgeIter<'a> {
    timeout: Option<time::Duration>,
    devs: Vec<(&'a DummyGpioIn, GpioValue)>,
}

impl<'a> DummyEdgeIter<'a> {
    pub fn new() -> Result<DummyEdgeIter<'a>, ()> {
        Ok(DummyEdgeIter {
            timeout: None,
            devs: Vec::new(),
        })
    }

    pub fn timeout_ms(&mut self, timeout_ms: u64) -> &mut Self {
        self.timeout = Some(time::Duration::from_millis(timeout_ms));
        self
    }

    pub fn add(&mut self, dev: &'a DummyGpioIn) -> Result<&mut Self, ()> {
        let val = dev.read_value()?;
        self.devs.push((dev, val));
        Ok(self)
    }
}

impl<'a> Iterator for DummyEdgeIter<'a> {
    type Item = Result<&'a DummyGpioIn, ()>;

    fn next(&mut self) -> Option<Result<&'a DummyGpioIn, ()>> {
        let start = time::Instant::now();
        loop {
            if self.timeout.map_or(false, |to| start.elapsed() > to) {
                return Some(Err(()));
            }
            for &mut (gpio, ref mut val) in &mut self.devs {
                let new_val = (gpio.value)();
                if *val == new_val {
                    continue;
                }
                *val = new_val;
                match (gpio.edge, new_val) {
                    (GpioEdge::Both, _) |
                    (GpioEdge::Rising, GpioValue::High) |
                    (GpioEdge::Falling, GpioValue::Low) => return Some(Ok(gpio)),
                    (GpioEdge::None, _) |
                    (GpioEdge::Rising, GpioValue::Low) |
                    (GpioEdge::Falling, GpioValue::High) => (),
                }
            }
            thread::sleep(time::Duration::from_millis(1))
        }
    }
}

/// Dummy GPIO output pin
#[derive(Debug)]
pub struct DummyGpioOut<F> {
    dest: F,
}

impl<F> DummyGpioOut<F> {
    /// Creates a new dummy pin that passes all set values to `dest`.
    pub fn new(dest: F) -> DummyGpioOut<F> {
        DummyGpioOut { dest }
    }
}

impl<F> GpioOut for DummyGpioOut<F>
where
    F: Fn(GpioValue) -> (),
{
    type Error = ();

    fn set_low(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::Low);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        (self.dest)(GpioValue::High);
        Ok(())
    }
}
