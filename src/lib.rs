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
//! The most commonly used implementation is based on the
//! [Linux GPIO Sysfs](https://www.kernel.org/doc/Documentation/gpio/sysfs.txt)
//! interface, found inside the `sysfs` crate.
//!
//! ## Example
//!
//! ```rust,no_run
//! use gpio::{GpioIn, GpioOut};
//! use std::{thread, time};
//!
//! // Let's open GPIO23 and -24, e.g. on a Raspberry Pi 2.
//! let mut gpio23 = gpio::sysfs::SysFsGpioInput::open(23).unwrap();
//! let mut gpio24 = gpio::sysfs::SysFsGpioOutput::open(24).unwrap();
//!
//! // GPIO24 will be toggled every second in the background by a different thread
//! let mut value = false;
//! thread::spawn(move || loop {
//!     gpio24.set_value(value).expect("could not set gpio24");
//!     thread::sleep(time::Duration::from_millis(1000));
//!     value = !value;
//! });
//!
//! // The main thread will simply display the current value of GPIO23 every 100ms.
//! loop {
//!     println!("GPIO23: {:?}", gpio23.read_value().unwrap());
//!     thread::sleep(time::Duration::from_millis(100));
//! }
//! ```
//!
//! ## TODO
//!
//! * `/dev/mem` interface: Higher frequency port usage
//!

pub mod sysfs;
pub mod dummy;

/// A value read from or written to a GPIO port
#[derive(Debug, Copy, Clone)]
pub enum GpioValue {
    /// A low value, usually 0 V
    Low,
    /// A high value, commonly 3.3V or 5V
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
    /// Errors that can occur during initialization of or writing to GPIO
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
    /// Errors that can occur during initialization of or reading from GPIO
    type Error;

    /// Perform a single reading of a GPIO port
    fn read_value(&mut self) -> Result<GpioValue, Self::Error>;
}
