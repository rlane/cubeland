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

extern mod extra;
extern mod glfw;
extern mod gl;
extern mod cgmath;
extern mod noise;

use std::cast;

use gl::types::*;

use cgmath::vector::Vec3;

use noise::Perlin;

use CHUNK_SIZE;

pub fn make_noise_texture() -> GLuint {
    let start_time = extra::time::precise_time_ns();

    let mut pixels : ~[Vec3<u8>] = ~[];
    static length : i32 = CHUNK_SIZE as i32 * 16;
    let perlin = Perlin::from_seed([43 as uint]);

    for x in range(0, length) {
        for y in range(0, length) {
            let noise = perlin.gen([
                x as f64 * 0.6,
                y as f64 * 0.6
            ]);
            let g = ((noise * 0.3 + 0.4) * 255.0) as u8;
            pixels.push(Vec3 { x: 0, y: g, z: 0 });
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
            gl::RGB as GLint,
            length, length, 0,
            gl::RGB, gl::UNSIGNED_BYTE,
            cast::transmute(&pixels[0]));
    }

    gl::GenerateMipmap(gl::TEXTURE_2D);

    gl::BindTexture(gl::TEXTURE_2D, 0);

    let end_time = extra::time::precise_time_ns();
    println!("texture gen: {}us", (end_time - start_time)/1000);

    tex
}
