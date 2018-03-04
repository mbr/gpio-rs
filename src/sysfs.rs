//! Linux `/sys`-fs based GPIO control
//!
//! Uses filesystem operations to control GPIO ports. Very portable (across
//! devices running Linux), but incurs quite a bit of syscall overhead.

use std::{fs, io};
use std::io::{Read, Write};
use super::{GpioIn, GpioOut, GpioValue};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GpioDirection {
    Input,
    Output,
}

fn export_gpio_if_unexported(gpio_num: u16) -> io::Result<()> {
    // export port first if not exported
    if let Err(_) = fs::metadata(&format!("/sys/class/gpio/gpio{}", gpio_num)) {
        let mut export_fp = fs::File::create("/sys/class/gpio/export")?;
        write!(export_fp, "{}", gpio_num)?;
    }

    // ensure we're using '0' as low
    fs::File::create(format!("/sys/class/gpio/gpio{}/active_low", gpio_num))?.write_all(b"0")
}

fn set_gpio_direction(gpio_num: u16, direction: GpioDirection) -> io::Result<()> {
    fs::File::create(format!("/sys/class/gpio/gpio{}/direction", gpio_num))?.write_all(
        match direction {
            GpioDirection::Input => b"in",
            GpioDirection::Output => b"out",
        },
    )
}

fn open_gpio(gpio_num: u16, direction: GpioDirection) -> io::Result<fs::File> {
    fs::File::create(format!("/sys/class/gpio/gpio{}/value", gpio_num))
}

#[derive(Debug)]
struct SysFsGpio {
    gpio_num: u16,
    sysfp: fs::File,
}

impl SysFsGpio {
    fn open(gpio_num: u16, direction: GpioDirection) -> io::Result<SysFsGpio> {
        export_gpio_if_unexported(gpio_num)?;

        // ensure we're using '0' as low.
        // FIXME: this should be configurable
        fs::File::create(format!("/sys/class/gpio/gpio{}/active_low", gpio_num))?.write_all(b"0")?;

        set_gpio_direction(gpio_num, direction)?;

        // finally, we can open the device
        Ok(SysFsGpio {
            gpio_num,
            sysfp: open_gpio(gpio_num, direction)?,
        })
    }

    fn set_direction(&mut self, direction: GpioDirection) -> io::Result<()> {
        set_gpio_direction(self.gpio_num, direction)?;
        self.sysfp = open_gpio(self.gpio_num, direction)?;

        Ok(())
    }
}

impl Drop for SysFsGpio {
    fn drop(&mut self) {
        // unexport the pin, if we have not done so already
        // best effort, failures are ignored
        let unexport_fp = fs::File::create("/sys/class/gpio/unexport");

        if let Ok(mut fp) = unexport_fp {
            write!(fp, "{}\n", self.gpio_num).ok();
        }
    }
}

/// `/sys`-fs based GPIO output
#[derive(Debug)]
pub struct SysFsGpioOutput {
    gpio: SysFsGpio,
}

impl SysFsGpioOutput {
    /// Open a GPIO port for Output.
    pub fn open(gpio_num: u16) -> io::Result<SysFsGpioOutput> {
        Ok(SysFsGpioOutput {
            gpio: SysFsGpio::open(gpio_num, GpioDirection::Output)?,
        })
    }

    pub fn into_input(mut self) -> io::Result<SysFsGpioInput> {
        self.gpio.set_direction(GpioDirection::Input)?;
        Ok(SysFsGpioInput { gpio: self.gpio })
    }
}

impl GpioOut for SysFsGpioOutput {
    type Error = io::Error;

    #[inline(always)]
    fn set_low(&mut self) -> io::Result<()> {
        self.gpio.sysfp.write_all(b"0")
    }

    #[inline(always)]
    fn set_high(&mut self) -> io::Result<()> {
        self.gpio.sysfp.write_all(b"1")
    }
}

/// `/sys`-fs based GPIO output
#[derive(Debug)]
pub struct SysFsGpioInput {
    gpio: SysFsGpio,
}

impl SysFsGpioInput {
    /// Open a GPIO port for Output.
    pub fn open(gpio_num: u16) -> io::Result<SysFsGpioInput> {
        Ok(SysFsGpioInput {
            gpio: SysFsGpio::open(gpio_num, GpioDirection::Input)?,
        })
    }

    pub fn into_output(mut self) -> io::Result<SysFsGpioOutput> {
        self.gpio.set_direction(GpioDirection::Output)?;
        Ok(SysFsGpioOutput { gpio: self.gpio })
    }
}

impl GpioIn for SysFsGpioInput {
    type Error = io::Error;

    fn read_value(&mut self) -> Result<GpioValue, Self::Error> {
        let mut buf: [u8; 1] = [0];
        self.gpio.sysfp.read_exact(&mut buf)?;

        match buf[0] {
            b'0' => Ok(GpioValue::Low),
            b'1' => Ok(GpioValue::High),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "read a value that was neither a '0' nor a '1' from Linux sysfs GPIO interface",
            )),
        }
    }
}
