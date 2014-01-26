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

#[feature(globs)];
#[feature(macro_rules)];

extern mod extra;
extern mod cgmath;
extern mod noise;

use extra::time::precise_time_ns;

use cgmath::vector::Vector;
use cgmath::vector::Vec3;

use terrain::Terrain;
use spiral::Spiral;

mod terrain;
mod spiral;

pub static N: uint = 4;
pub static CHUNK_SIZE: uint = 32;
pub static WORLD_SEED: u32 = 42;

fn main() {
    let mut spiral = Spiral::<f64>::new(N);

    let start_time = precise_time_ns();

    for v in spiral {
        let p = Vec3::new(v.x, 0.0, v.y).mul_s(CHUNK_SIZE as f64);
        Terrain::gen(WORLD_SEED, p);
    }

    let end_time = precise_time_ns();

    println!("average : {}us",
            (end_time - start_time)/(1000*N as u64));
}
