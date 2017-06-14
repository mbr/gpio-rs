//! GPIO interface
//!
//! The GPIO crate allows easy and fast access to GPIO pins. It tries to have
//! an ergonomic interface while being as fast as possible, to enable
//! bitbanging requiring fast switching, as well as simple applications.
//!
//! The core interface is defined using `GpioValue` and the `GpioOut` and
//! `GpioIn` traits. Concrete implementations are available in submodules.

extern crate core;
extern crate libc;
extern crate nix;

use core::ptr::{read_volatile, write_volatile};
use nix::sys::mman;
use std::{fs, io};
use std::io::Write;
use std::os::unix::io::AsRawFd;

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
    fn set_value<T: Into<GpioValue> + Copy>(&mut self, value: T) -> bool;

    #[inline(always)]
    fn set_low(&mut self) -> bool {
        self.set_value(GpioValue::Low)
    }

    #[inline(always)]
    fn set_high(&mut self) -> bool {
        self.set_value(GpioValue::High)
    }
}

#[derive(Debug)]
pub struct SysFsGpioOut {
    gpio_num: u16,
    sysfp: fs::File,
}

impl SysFsGpioOut {
    pub fn new(gpio_num: u16) -> io::Result<SysFsGpioOut> {
        // export port first
        {
            let mut export_fp = try!(fs::File::create("/sys/class/gpio/export"));
            try!(write!(export_fp, "{}", gpio_num));
        }

        {
            let mut output_fp = try!(fs::File::create(format!("/sys/class/gpio/gpio{}/direction",
                                                              gpio_num)));
            try!(write!(output_fp, "out"));
        }

        // ensure we're using 0 as low
        {
            let mut al_fp = try!(fs::File::create(format!("/sys/class/gpio/gpio{}/active_low",
                                                          gpio_num)));
            try!(write!(al_fp, "0"));
        }

        let sysfp = try!(fs::File::create(format!("/sys/class/gpio/gpio{}/value", gpio_num)));

        Ok(SysFsGpioOut {
               gpio_num: gpio_num,
               sysfp: sysfp,
           })
    }

    pub fn force_new(gpio_num: u16) -> io::Result<SysFsGpioOut> {
        // unexport first
        {
            let mut unexport_fp = try!(fs::File::create("/sys/class/gpio/unexport"));

            // ignore result from write
            write!(unexport_fp, "{}\n", gpio_num).ok();
        }

        Self::new(gpio_num)
    }
}

impl Drop for SysFsGpioOut {
    fn drop(&mut self) {
        let unexport_fp = fs::File::create("/sys/class/gpio/unexport");

        if let Ok(mut fp) = unexport_fp {
            // best effort
            write!(fp, "{}\n", self.gpio_num).ok();
        }
    }
}

impl GpioOut for SysFsGpioOut {
    #[inline(always)]
    fn set_value<T: Into<GpioValue> + Copy>(&mut self, value: T) -> bool {
        let val: GpioValue = value.into();

        if let Ok(_) = write!(self.sysfp,
                              "{}",
                              match val {
                                  GpioValue::Low => "0",
                                  GpioValue::High => "1",
                              }) {
            true
        } else {
            false
        }
    }
}

#[derive(Debug)]
pub struct RasPi1GpioOut(u8);


const RP1_PERIPH_BASE: usize = 0x2000_0000;
const RP1_GPIO_BASE: usize = RP1_PERIPH_BASE + 0x0020_0000;


impl RasPi1GpioOut {
    pub unsafe fn new(num: u8) -> RasPi1GpioOut {
        // set direction: output. warning: not thread-safe!
        let n = num as usize;

        let dir_addr = RP1_GPIO_BASE + (n / 10) * 4;
        // println!("volatile load: {:#x}", dir_addr);
        let mut tmp = read_volatile(dir_addr as *const u32);
        // println!("result: {:#x}", tmp);
        // FIXME: 0-out alternate function

        // zero out alternative use flags
        tmp &= !(7 << ((n % 10) * 3));

        // add output bit
        tmp |= 1 << ((n % 10) * 3);
        // println!("volatile store: {:#x} @ {:#x}", tmp, dir_addr);
        write_volatile(dir_addr as *mut u32, tmp);

        RasPi1GpioOut(num)
    }
}

impl GpioOut for RasPi1GpioOut {
    fn set_value<T: Into<GpioValue> + Copy>(&mut self, value: T) -> bool {
        // FIXME: rethink types here, allow setting of more than one gpio in
        // one go?

        let dest_addr = RP1_GPIO_BASE
                        + match value.into() {
                            // GPCLR0
                            GpioValue::Low => 0x0000_0028,
                            // GPSET0
                            GpioValue::High => 0x0000_001C,
                        }
                        // first or second register?
                        + 0x04 * (self.0 as usize / 32);
        let bit_num = self.0 % 32;

        unsafe {
            // println!("set v store: {:#x} @ {:#x}", 1 << bit_num, dest_addr);
            write_volatile(dest_addr as *mut u32, 1 << bit_num)
        };
        true
    }
}


pub fn init_rp1_gpio() {
    // FIXME: this may need O_SYNC as well
    // FIXME: remove unwrap
    let mem = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/mem")
        .unwrap();

    let fd = mem.as_raw_fd();
    println!("/dev/mem fd: {}", fd);

    // open /dev/mem
    mman::mmap(RP1_GPIO_BASE as *mut libc::c_void,
               1024*4,  // FIXME: size?
               mman::PROT_READ | mman::PROT_WRITE,
               mman::MAP_SHARED | mman::MAP_FIXED,
               fd,
               RP1_GPIO_BASE as i64,  // FIXME: i64/i32 is platform-dependant
              )
            .unwrap();
}
