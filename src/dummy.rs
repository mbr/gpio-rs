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
//! The `DummyGpioIn` reads value from a callback:
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

use super::{GpioIn, GpioOut, GpioValue};

/// Dummy GPIO input pin
#[derive(Debug)]
pub struct DummyGpioIn<F> {
    value: F,
}

impl<F> DummyGpioIn<F> {
    /// Create new dummy pin that returns the value of `value` every it is read
    pub fn new(value: F) -> DummyGpioIn<F> {
        DummyGpioIn { value }
    }
}

impl<V, F> GpioIn for DummyGpioIn<F>
where
    V: Into<GpioValue>,
    F: Fn() -> V,
{
    type Error = ();

    fn read_value(&mut self) -> Result<GpioValue, Self::Error> {
        Ok((self.value)().into())
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
        Ok((self.dest)(GpioValue::Low))
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok((self.dest)(GpioValue::High))
    }
}
