//! Raspberry Pi 2 GPIO example
//!
//! Reads from one GPIO, while toggling output on the other

extern crate gpio;

use gpio::{GpioIn, GpioOut};

use std::{thread, time};

fn main() {
    let gpio23 = gpio::sysfs::SysFsGpioInput::open(23).expect("could not open GPIO23");
    let mut gpio24 = gpio::sysfs::SysFsGpioOutput::open(24).expect("could not open GPIO24");

    // start a thread to toggle the gpio on and off at a different rate
    let mut value = false;
    thread::spawn(move || loop {
        gpio24.set_value(value).expect("could not set gpio24");
        thread::sleep(time::Duration::from_millis(1000));
        value = !value;
    });

    loop {
        println!(
            "GPIO23: {:?}",
            gpio23.read_value().expect("could not read gpio23")
        );

        thread::sleep(time::Duration::from_millis(100));
    }
}
