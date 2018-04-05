//! Linux `/sys`-fs based GPIO control
//!
//! Uses the [Linux GPIO Sysfs](https://www.kernel.org/doc/Documentation/gpio/sysfs.txt) filesystem
//! operations to control GPIO ports. It tries to reduce the otherwise hefty syscall overhead
//! by keeping the sysfs files open, instead of reopening them on each read.
//!
//! Every `open` call to a GPIO pin will automatically export the necessary pin and unexport it
//! on close.

use nix;
use nix::sys::epoll::{self, EpollEvent, EpollFlags, EpollOp};
use std::{cell, fs, io, isize};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use super::{GpioEdge, GpioIn, GpioOut, GpioValue};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum GpioDirection {
    Input,
    Output,
}

quick_error! {
    #[derive(Debug)]
    pub enum GpioError {
        Io(err: io::Error) {
            from()
            description("io error")
            display("I/O error: {}", err)
            cause(err)
        }
        Epoll(err: nix::Error) {
            from()
            description("epoll error")
            display("Epoll error: {}", err)
            cause(err)
        }
        EpollEventCount(count: usize) {
            description("epoll_wait returned unexpected event count value")
            display("epoll_wait returned unexpected event count value: {}", count)
        }
        EpollDataValue(val: u64) {
            description("epoll_wait returned unexpected data value")
            display("epoll_wait returned unexpected data value: {}", val)
        }
        InvalidData(val: u8) {
            description("read a value that was neither '0' nor '1' from Linux sysfs GPIO interface")
            display("read value {:?} from Linux sysfs GPIO interface, which is neither '0' nor '1'",
                    val)
        }
    }
}

pub type GpioResult<T> = Result<T, GpioError>;

#[inline]
fn export_gpio_if_unexported(gpio_num: u16) -> GpioResult<()> {
    // export port first if not exported
    if fs::metadata(&format!("/sys/class/gpio/gpio{}", gpio_num)).is_err() {
        let mut export_fp = fs::File::create("/sys/class/gpio/export")?;
        write!(export_fp, "{}", gpio_num)?;
    }

    // ensure we're using '0' as low
    fs::File::create(format!("/sys/class/gpio/gpio{}/active_low", gpio_num))?
        .write_all(b"0")?;
    Ok(())
}

#[inline]
fn set_gpio_direction(gpio_num: u16, direction: GpioDirection) -> GpioResult<()> {
    fs::File::create(format!("/sys/class/gpio/gpio{}/direction", gpio_num))?
        .write_all(match direction {
            GpioDirection::Input => b"in",
            GpioDirection::Output => b"out",
        })?;
    Ok(())
}

#[inline]
fn open_gpio(gpio_num: u16, direction: GpioDirection) -> GpioResult<fs::File> {
    let p = format!("/sys/class/gpio/gpio{}/value", gpio_num);

    Ok(match direction {
        GpioDirection::Input => fs::File::open(p),
        GpioDirection::Output => fs::File::create(p),
    }?)
}

#[derive(Debug)]
struct SysFsGpio {
    gpio_num: u16,
    sysfp: cell::RefCell<fs::File>,
}

impl SysFsGpio {
    fn open(gpio_num: u16, direction: GpioDirection) -> GpioResult<SysFsGpio> {
        export_gpio_if_unexported(gpio_num)?;

        // ensure we're using '0' as low.
        // FIXME: this should be configurable
        fs::File::create(format!("/sys/class/gpio/gpio{}/active_low", gpio_num))?
            .write_all(b"0")?;

        set_gpio_direction(gpio_num, direction)?;

        // finally, we can open the device
        Ok(SysFsGpio {
            gpio_num,
            sysfp: cell::RefCell::new(open_gpio(gpio_num, direction)?),
        })
    }

    #[inline]
    fn set_direction(&mut self, direction: GpioDirection) -> GpioResult<()> {
        set_gpio_direction(self.gpio_num, direction)?;
        self.sysfp = cell::RefCell::new(open_gpio(self.gpio_num, direction)?);

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
    pub fn open(gpio_num: u16) -> GpioResult<SysFsGpioOutput> {
        Ok(SysFsGpioOutput {
            gpio: SysFsGpio::open(gpio_num, GpioDirection::Output)?,
        })
    }

    #[inline]
    pub fn into_input(mut self) -> GpioResult<SysFsGpioInput> {
        self.gpio.set_direction(GpioDirection::Input)?;
        SysFsGpioInput::from_gpio(self.gpio)
    }

    #[inline]
    pub fn gpio_num(&self) -> u16 {
        self.gpio.gpio_num
    }
}

impl GpioOut for SysFsGpioOutput {
    type Error = GpioError;

    #[inline]
    fn set_low(&mut self) -> GpioResult<()> {
        self.gpio.sysfp.get_mut().write_all(b"0")?;
        Ok(())
    }

    #[inline]
    fn set_high(&mut self) -> GpioResult<()> {
        self.gpio.sysfp.get_mut().write_all(b"1")?;
        Ok(())
    }
}

/// `/sys`-fs based GPIO output
#[derive(Debug)]
pub struct SysFsGpioInput {
    gpio: SysFsGpio,
}

impl SysFsGpioInput {
    /// Open a GPIO port for Output.
    #[inline]
    pub fn open(gpio_num: u16) -> GpioResult<SysFsGpioInput> {
        Self::from_gpio(SysFsGpio::open(gpio_num, GpioDirection::Input)?)
    }

    #[inline]
    fn from_gpio(gpio: SysFsGpio) -> GpioResult<SysFsGpioInput> {
        Ok(SysFsGpioInput { gpio })
    }

    #[inline]
    pub fn into_output(mut self) -> GpioResult<SysFsGpioOutput> {
        self.gpio.set_direction(GpioDirection::Output)?;
        Ok(SysFsGpioOutput { gpio: self.gpio })
    }

    #[inline]
    pub fn gpio_num(&self) -> u16 {
        self.gpio.gpio_num
    }
}

impl GpioIn for SysFsGpioInput {
    type Error = GpioError;

    #[inline]
    fn read_value(&self) -> Result<GpioValue, Self::Error> {
        let mut buf: [u8; 1] = [0; 1];

        // we rewind the file descriptor first, otherwise read will fail
        self.gpio.sysfp.borrow_mut().seek(SeekFrom::Start(0))?;

        // we read one byte, the trailing byte is a newline
        self.gpio.sysfp.borrow_mut().read_exact(&mut buf)?;

        match buf[0] {
            b'0' => Ok(GpioValue::Low),
            b'1' => Ok(GpioValue::High),
            val => {
                println!("BUFFER: {:?}", buf);
                Err(GpioError::InvalidData(val))
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
}

pub struct SysFsGpioEdgeIter<'a> {
    /// The timeout, if any.
    timeout: Option<u64>,
    /// The GPIO devices whose edges will be included in this iterator.
    devs: Vec<&'a SysFsGpioInput>,
    /// The file descriptor of the epoll instance.
    epoll_fd: RawFd,
}

impl<'a> SysFsGpioEdgeIter<'a> {
    pub fn new() -> GpioResult<SysFsGpioEdgeIter<'a>> {
        let epoll_fd = epoll::epoll_create()?;
        Ok(SysFsGpioEdgeIter {
            timeout: None,
            devs: Vec::new(),
            epoll_fd,
        })
    }

    pub fn timeout_ms(&mut self, timeout_ms: u64) -> &mut Self {
        self.timeout = Some(timeout_ms);
        self
    }

    pub fn add(&mut self, dev: &'a SysFsGpioInput) -> GpioResult<&mut Self> {
        // We use the device's index in the `devs` vector as the data registered with epoll.
        let index = self.devs.len() as u64;
        let flags = EpollFlags::EPOLLPRI | EpollFlags::EPOLLET;
        let mut event = EpollEvent::new(flags, index);
        let dev_fd = dev.gpio.sysfp.borrow().as_raw_fd();
        epoll::epoll_ctl(self.epoll_fd, EpollOp::EpollCtlAdd, dev_fd, &mut event)?;
        self.devs.push(dev);
        Ok(self)
    }

    fn get_next(&mut self) -> GpioResult<&'a SysFsGpioInput> {
        let timeout = self.timeout.map_or(isize::MAX, |t| t as isize);
        // A dummy event, to be overwritten by `epoll`.
        let mut events = [EpollEvent::empty()];
        let event_count = epoll::epoll_wait(self.epoll_fd, &mut events, timeout)?;
        if event_count != 1 {
            return Err(GpioError::EpollEventCount(event_count));
        }
        // Epoll wrote the event data into the array. We used the device's index as the data:
        self.devs
            .get(events[0].data() as usize)
            .map(|d| *d)
            .ok_or_else(|| GpioError::EpollDataValue(events[0].data()))
    }
}

impl<'a> Iterator for SysFsGpioEdgeIter<'a> {
    type Item = GpioResult<&'a SysFsGpioInput>;

    fn next(&mut self) -> Option<GpioResult<&'a SysFsGpioInput>> {
        Some(self.get_next())
    }
}
