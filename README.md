Rust GPIO
=========

Deals with GPIO access on Linux and bare metal embedded systems, through sysfs
and direct memory access. Works on stable Rust.

Roadmap
-------

- [x] GPIO write support
- [x] Read support
- [ ] Interrupt support

Other libraries
---------------

Other libraries [can be found on crates.io](https://crates.io/search?q=gpio).
These include:

* `sysfs_gpio <https://github.com/rust-embedded/rust-sysfs-gpio>`_ handles GPIO
  only via SysFS, but exposes all features. Slightly lower level.
* `cylus <https://github.com/Vikaton/cylus>`_ Documentation is dead, does a few
  questionable things like unwrapping()
* `cupi <https://github.com/cuprumpi/cupi>`_ Most comprehensive GPIO library,
  includes almost all features planned for gpio. Does not use volatile.

  TODO: Benchmark
