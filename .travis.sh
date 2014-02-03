#!/bin/bash -eux
rm -rf rust-gamedev-kit
git clone git://github.com/rlane/rust-gamedev-kit
git -C rust-gamedev-kit submodule init
git -C rust-gamedev-kit submodule deinit rust
git -C rust-gamedev-kit submodule update
./rust-gamedev-kit/build-libraries.sh

export PKG_CONFIG_PATH=./rust-gamedev-kit/install/lib/pkgconfig
rustc --link-args="`./rust-gamedev-kit/glfw-rs/build/etc/link-args`" -L ./rust-gamedev-kit/install/lib/rustlib/*/lib src/cubeland/main.rs
rustc -L ./rust-gamedev-kit/install/lib/rustlib/*/lib src/terrain-benchmark/main.rs
