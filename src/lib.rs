//! GPIO interface
//!
//! The GPIO crate allows easy and fast access to GPIO pins. It tries to have
//! an ergonomic interface while being as fast as possible, to enable
//! bitbanging requiring fast switching, as well as simple applications.
//!
//! The core interface is defined using `GpioValue` and the `GpioOut` and
//! `GpioIn` traits. Concrete implementations are available in submodules.

pub mod sysfs;

#[derive(Debug, Copy, Clone)]
pub enum GpioValue {
    Low,
    High,
}

impl From<bool> for GpioValue {
    fn from(val: bool) -> GpioValue {
        if val { GpioValue::High } else { GpioValue::Low }
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

pub trait GpioOut {
    #[inline(always)]
    fn set_value<T: Into<GpioValue> + Copy>(&mut self, value: T) -> bool {
        match value.into() {
            GpioValue::High => self.set_high(),
            GpioValue::Low => self.set_low(),
        }
    }

    #[inline(always)]
    fn set_low(&mut self) -> bool;

    #[inline(always)]
    fn set_high(&mut self) -> bool;
}
