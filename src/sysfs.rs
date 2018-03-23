//! Linux `/sys`-fs based GPIO control
//!
//! Uses the [Linux GPIO Sysfs](https://www.kernel.org/doc/Documentation/gpio/sysfs.txt) filesystem
//! operations to control GPIO ports. It tries to reduce the otherwise hefty syscall overhead
//! by keeping the sysfs files open, instead of reopening them on each read.
//!
//! Every `open` call to a GPIO pin will automatically export the necessary pin and unexport it
//! on close.

use nix::sys::epoll::{self, EpollEvent, EpollFlags, EpollOp};
use std::{fs, io};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use super::{GpioEdge, GpioIn, GpioOut, GpioValue};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GpioDirection {
    Input,
    Output,
}

#[inline]
fn export_gpio_if_unexported(gpio_num: u16) -> io::Result<()> {
    // export port first if not exported
    if let Err(_) = fs::metadata(&format!("/sys/class/gpio/gpio{}", gpio_num)) {
        let mut export_fp = fs::File::create("/sys/class/gpio/export")?;
        write!(export_fp, "{}", gpio_num)?;
    }

    // ensure we're using '0' as low
    fs::File::create(format!("/sys/class/gpio/gpio{}/active_low", gpio_num))?.write_all(b"0")
}

#[inline]
fn set_gpio_direction(gpio_num: u16, direction: GpioDirection) -> io::Result<()> {
    fs::File::create(format!("/sys/class/gpio/gpio{}/direction", gpio_num))?.write_all(
        match direction {
            GpioDirection::Input => b"in",
            GpioDirection::Output => b"out",
        },
    )
}

#[inline]
fn open_gpio(gpio_num: u16, direction: GpioDirection) -> io::Result<fs::File> {
    let p = format!("/sys/class/gpio/gpio{}/value", gpio_num);

    match direction {
        GpioDirection::Input => fs::File::open(p),
        GpioDirection::Output => fs::File::create(p),
    }
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

    #[inline]
    fn set_direction(&mut self, direction: GpioDirection) -> io::Result<()> {
        set_gpio_direction(self.gpio_num, direction)?;
        self.sysfp = open_gpio(self.gpio_num, direction)?;

        Ok(())
    }
}

impl Drop for SysFsGpio {
    #[inline]
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
    #[inline]
    pub fn open(gpio_num: u16) -> io::Result<SysFsGpioOutput> {
        Ok(SysFsGpioOutput {
            gpio: SysFsGpio::open(gpio_num, GpioDirection::Output)?,
        })
    }

    #[inline]
    pub fn into_input(mut self) -> io::Result<SysFsGpioInput> {
        self.gpio.set_direction(GpioDirection::Input)?;
        SysFsGpioInput::from_gpio(self.gpio)
    }
}

impl GpioOut for SysFsGpioOutput {
    type Error = io::Error;

    #[inline]
    fn set_low(&mut self) -> io::Result<()> {
        self.gpio.sysfp.write_all(b"0")
    }

    #[inline]
    fn set_high(&mut self) -> io::Result<()> {
        self.gpio.sysfp.write_all(b"1")
    }
}

/// `/sys`-fs based GPIO output
#[derive(Debug)]
pub struct SysFsGpioInput {
    gpio: SysFsGpio,
    epoll_fd: RawFd,
}

impl SysFsGpioInput {
    /// Open a GPIO port for Output.
    #[inline]
    pub fn open(gpio_num: u16) -> io::Result<SysFsGpioInput> {
        Self::from_gpio(SysFsGpio::open(gpio_num, GpioDirection::Input)?)
    }

    #[inline]
    fn from_gpio(gpio: SysFsGpio) -> io::Result<SysFsGpioInput> {
        let dev_fd = gpio.sysfp.as_raw_fd();
        let epoll_fd =
            epoll::epoll_create().map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        let mut event = EpollEvent::new(EpollFlags::EPOLLPRI | EpollFlags::EPOLLET, 0);
        epoll::epoll_ctl(epoll_fd, EpollOp::EpollCtlAdd, dev_fd, &mut event)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(SysFsGpioInput { gpio, epoll_fd })
    }

    #[inline]
    pub fn into_output(mut self) -> io::Result<SysFsGpioOutput> {
        self.gpio.set_direction(GpioDirection::Output)?;
        Ok(SysFsGpioOutput { gpio: self.gpio })
    }
}

impl GpioIn for SysFsGpioInput {
    type Error = io::Error;

    #[inline]
    fn read_value(&mut self) -> Result<GpioValue, Self::Error> {
        let mut buf: [u8; 1] = [0; 1];

        // we rewind the file descriptor first, otherwise read will fail
        self.gpio.sysfp.seek(SeekFrom::Start(0))?;

        // we read one byte, the trailing byte is a newline
        self.gpio.sysfp.read_exact(&mut buf)?;

        match buf[0] {
            b'0' => Ok(GpioValue::Low),
            b'1' => Ok(GpioValue::High),
            _ => {
                println!("BUFFER: {:?}", buf);
                Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "read a value that was neither a '0' nor a '1' from Linux sysfs GPIO interface",
                ))
            }
        }
    }

    fn set_edge(&mut self, edge: GpioEdge) -> Result<(), Self::Error> {
        fs::OpenOptions::new()
            .write(true)
            .open(format!("/sys/class/gpio/gpio{}/edge", self.gpio.gpio_num))?
            .write_all(match edge {
                GpioEdge::None => b"none",
                GpioEdge::Rising => b"rising",
                GpioEdge::Falling => b"falling",
                GpioEdge::Both => b"both",
            })?;
        Ok(())
    }

    fn wait_for_edge(&mut self, timeout_ms: u64) -> Result<Option<GpioValue>, Self::Error> {
        let _ = self.read_value()?;
        let event = EpollEvent::new(EpollFlags::EPOLLPRI | EpollFlags::EPOLLET, 0);
        let mut events: [EpollEvent; 1] = [event];
        let cnt = epoll::epoll_wait(self.epoll_fd, &mut events, timeout_ms as isize)
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err))?;
        Ok(match cnt {
            0 => None, // timeout
            _ => Some(self.read_value()?),
        })
    }
}
