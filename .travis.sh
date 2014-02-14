#!/bin/bash -eux
rm -rf rust-gamedev-kit
git clone git://github.com/rlane/rust-gamedev-kit
pushd rust-gamedev-kit
git submodule init
git submodule deinit rust
git submodule update
./build-libraries.sh
popd

export PKG_CONFIG_PATH=./rust-gamedev-kit/install/lib/pkgconfig
rustc -C link-args="`pkg-config --libs glfw3`" -L ./rust-gamedev-kit/install/lib/rustlib/*/lib src/cubeland/main.rs
rustc -L ./rust-gamedev-kit/install/lib/rustlib/*/lib src/terrain-benchmark/main.rs
