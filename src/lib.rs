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
//! ## Example: writing and reading
//!
//! ```rust,no_run
//! use gpio::{GpioIn, GpioOut};
//! use std::{thread, time};
//!
//! // Let's open GPIO23 and -24, e.g. on a Raspberry Pi 2.
//! let gpio23 = gpio::sysfs::SysFsGpioInput::open(23).unwrap();
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
//! ## Example: waiting for falling edges
//!
//! ```rust,no_run
//! use gpio::{GpioEdge, GpioIn, GpioOut};
//! use std::{thread, time};
//!
//! let mut gpio17 = gpio::sysfs::SysFsGpioInput::open(17).unwrap();
//! let mut gpio27 = gpio::sysfs::SysFsGpioOutput::open(27).unwrap();
//!
//! // GPIO27 will be toggled every second in the background by a different thread
//! let mut value = false;
//! thread::spawn(move || loop {
//!     gpio27.set_value(value).expect("could not set gpio27");
//!     println!("GPIO27 set to {:?}", value);
//!     thread::sleep(time::Duration::from_millis(1000));
//!     value = !value;
//! });
//!
//! // GPIO17 waits for falling edges and displays the value
//! gpio17
//!     .set_edge(GpioEdge::Falling)
//!     .expect("set edge on gpio17");
//! for result in gpio::sysfs::SysFsGpioEdgeIter::new()
//!     .expect("create iterator")
//!     .add(&gpio17)
//!     .expect("add gpio 17 to iter")
//! {
//!     println!("GPIO17: {:?}", result.unwrap().gpio_num());
//! }
//! ```
//!
//! ## TODO
//!
//! * `/dev/mem` interface: Higher frequency port usage
//!

extern crate nix;
#[macro_use]
extern crate quick_error;

pub mod sysfs;
pub mod dummy;

/// A value read from or written to a GPIO port
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GpioValue {
    /// A low value, usually 0 V
    Low,
    /// A high value, commonly 3.3V or 5V
    High,
}

/// A setting for signaling an interrupt.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum GpioEdge {
    /// No interrupt.
    None,
    /// Interrupt on rising edge, i.e. when going from 0 to 1.
    Rising,
    /// Interrupt on falling edge, i.e. when going from 1 to 0.
    Falling,
    /// Interrupt on both edges, i.e. whenever the value changes.
    Both,
}

impl From<bool> for GpioValue {
    #[inline]
    fn from(val: bool) -> GpioValue {
        if val { GpioValue::High } else { GpioValue::Low }
    }
}

impl From<u8> for GpioValue {
    #[inline]
    fn from(val: u8) -> GpioValue {
        if val != 0 {
            GpioValue::High
        } else {
            GpioValue::Low
        }
    }
}

impl From<GpioValue> for bool {
    #[inline]
    fn from(val: GpioValue) -> bool {
        match val {
            GpioValue::Low => false,
            GpioValue::High => true,
        }
    }
}

impl From<GpioValue> for u8 {
    #[inline]
    fn from(val: GpioValue) -> u8 {
        match val {
            GpioValue::Low => 0,
            GpioValue::High => 1,
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
    fn set_low(&mut self) -> Result<(), Self::Error>;

    /// Set the GPIO port to a high output value directly
    fn set_high(&mut self) -> Result<(), Self::Error>;
}

/// Supports reading `GPIOValue`s
pub trait GpioIn {
    /// Errors that can occur during initialization of or reading from GPIO
    type Error;

    /// Perform a single reading of a GPIO port
    fn read_value(&self) -> Result<GpioValue, Self::Error>;

    /// Configure the criterion for signaling an interrupt.
    fn set_edge(&mut self, edge: GpioEdge) -> Result<(), Self::Error>;
}
