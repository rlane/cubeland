// Copyright 2014 Rich Lane.

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

extern crate extra;
extern crate gl;
extern crate cgmath;
extern crate noise;

use std::cast;

use gl::types::*;

use noise::sources::Perlin;
use noise::Source;

pub fn make_noise_texture() -> GLuint {
    let start_time = extra::time::precise_time_ns();

    let mut pixels : ~[u8] = ~[];
    static length : i32 = 128;
    let perlin = Perlin {
        seed: 7,
        octaves: 1,
        frequency: 0.6,
        lacunarity: 2.0,
        persistence: 0.5,
        quality: noise::Best,
    };

    for x in range(0, length) {
        for y in range(0, length) {
            let noise = perlin.get(x as f64, y as f64, 0.0);
            let x = ((noise * 0.5 + 0.5) * 255.0) as u8;
            pixels.push(x);
        }
    }

    let mut tex : GLuint = 0;

    unsafe {
        gl::GenTextures(1, &mut tex);
    }

    gl::BindTexture(gl::TEXTURE_2D, tex);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::LINEAR_MIPMAP_LINEAR as GLint);
    gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);

    unsafe {
        gl::TexImage2D(
            gl::TEXTURE_2D, 0,
            1 as GLint,
            length, length, 0,
            gl::RED, gl::UNSIGNED_BYTE,
            cast::transmute(&pixels[0]));
    }

    gl::GenerateMipmap(gl::TEXTURE_2D);

    gl::BindTexture(gl::TEXTURE_2D, 0);

    let end_time = extra::time::precise_time_ns();
    println!("texture gen: {}us", (end_time - start_time)/1000);

    tex
}
