//! Raspberry Pi 2 GPIO example
//!
//! Reads from one GPIO, while toggling output on the other

extern crate gpio;

use gpio::{GpioEdge, GpioIn, GpioOut};
use std::{thread, time};

fn main() {
    let mut gpio17 = gpio::sysfs::SysFsGpioInput::open(17).unwrap();
    let mut gpio27 = gpio::sysfs::SysFsGpioOutput::open(27).unwrap();

    // GPIO27 will be toggled every second in the background by a different thread
    let mut value = false;
    thread::spawn(move || loop {
        gpio27.set_value(value).expect("could not set gpio27");
        println!("GPIO27 set to {:?}", value);
        thread::sleep(time::Duration::from_millis(1000));
        value = !value;
    });

    // GPIO17 waits for falling edges and displays the value
    gpio17.set_edge(GpioEdge::Rising).expect(
        "set edge on gpio17",
    );
    for result in gpio::sysfs::SysFsGpioEdgeIter::new()
        .expect("create iterator")
        .add(&gpio17)
        .expect("add gpio 17 to iter")
    {
        println!("GPIO17: {:?}", result.unwrap().gpio_num());
    }
}
