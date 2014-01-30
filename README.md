cubeland
========

Infinite procedural landscape. Not a game yet.

Derived from [pong-rs](https://github.com/zokier/pong-rs).


Dependencies
============

* [glfw-rs](https://github.com/bjz/glfw-rs)
* [gl-rs](https://github.com/bjz/gl-rs)
* [cgmath-rs](https://github.com/bjz/cgmath-rs)
* [noise-rs](https://github.com/bjz/noise-rs)
* Rust 0.10-pre


Building
========

The easiest way to get compatible versions of Rust and all the libraries this
project uses is [rust-gamedev-kit][1]. Once the Rust compiler and libraries
have been installed:

    rustc --opt-level=3 src/cubeland/main.rs

This produces the executable `src/cubeland/main`.

[1]: https://github.com/rlane/rust-gamedev-kit


Screenshots
===========

![Screenshot](https://raw.github.com/rlane/cubeland/master/doc/screenshot.png)
