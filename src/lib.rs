//! GPIO interface
//!
//! The GPIO crate allows easy and fast access to GPIO pins. It aims to provide
//! an ergonomic interface while being lower overhead, enabling high-frequency
//! output without complicating simple tasks.
//!
//! The core interface is defined using `GpioValue` and the `GpioOut`/`GpioIn`
//! traits. All backends implement at least some of these traits, making them
//! interchangeable, e.g. for testing.
//!
//! The most commonly used implementation is based on the [Linux GPIO Sysfs]
//! (https://www.kernel.org/doc/Documentation/gpio/sysfs.txt) interface, found
//! inside the `sysfs` crate.
//!
//! ### TODO
//!
//! * `/dev/mem` interface: Higher frequency port usage

pub mod sysfs;

/// A value read from or written to a GPIO port
#[derive(Debug, Copy, Clone)]
pub enum GpioValue {
    Low,
    High,
}

impl From<bool> for GpioValue {
    fn from(val: bool) -> GpioValue {
        if val {
            GpioValue::High
        } else {
            GpioValue::Low
        }
    }
}

impl From<u8> for GpioValue {
    fn from(val: u8) -> GpioValue {
        if val != 0 {
            GpioValue::High
        } else {
            GpioValue::Low
        }
    }
}

/// Supports sending `GPIOValue`s
pub trait GpioOut {
    type Error;

    /// Sets the output value of the GPIO port
    #[inline(always)]
    fn set_value<T: Into<GpioValue> + Copy>(&mut self, value: T) -> Result<(), Self::Error> {
        match value.into() {
            GpioValue::High => self.set_high(),
            GpioValue::Low => self.set_low(),
        }
    }

    /// Set the GPIO port to a low output value directly
    #[inline(always)]
    fn set_low(&mut self) -> Result<(), Self::Error>;

    /// Set the GPIO port to a high output value directly
    #[inline(always)]
    fn set_high(&mut self) -> Result<(), Self::Error>;
}

/// Supports reading `GPIOValue`s
pub trait GpioIn {
    type Error;

    /// Perform a single reading of a GPIO port
    fn read_value(&mut self) -> Result<GpioValue, Self::Error>;
}
