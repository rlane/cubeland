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

use terrain::TerrainGenerator;

mod terrain;

pub static NUM_ITERS: uint = 1;
pub static VISIBLE_RADIUS: uint = 8;
pub static CHUNK_SIZE: int = 32;
pub static WORLD_SEED: u32 = 42;

fn main() {
    let mut times = ~[];
    let terrain_generator = TerrainGenerator::new(WORLD_SEED);

    let camera_position = Vec3::new(0.0, 0.0, 0.0);
    for &c in nearby_chunk_coords(camera_position).iter() {
        let p = Vec3::new(c.x as f64, c.y as f64, c.z as f64).mul_s(CHUNK_SIZE as f64);
        let start_time = precise_time_ns();
        terrain_generator.gen(p);
        let end_time = precise_time_ns();
        times.push((end_time - start_time)/1000);
    }

    times.sort();

    println!("{} chunks generated", times.len());
    println!("minimum : {}us", times[0]);
    println!("median  : {}us", times[times.len()/2]);
    println!("maximum : {}us", times[times.len()-1]);
}

fn nearby_chunk_coords(p: Vec3<f64>) -> ~[Vec3<i64>] {
    let cur_chunk_coord = Vec3::new(p.x as i64, p.y as i64, p.z as i64).div_s(CHUNK_SIZE as i64);
    let r = VISIBLE_RADIUS as i64;

    let mut coords = ~[];

    for x in range(-r, r+1) {
        for y in range(-r, r+1) {
            for z in range(-r, r+1) {
                let c = Vec3::new(x, y, z);
                if c.dot(&c) < r*r {
                    coords.push(c);
                }
            }
        }
    }

    coords.sort_by(|b,a| b.dot(b).cmp(&a.dot(a)));

    for c in coords.mut_iter() {
        c.add_self_v(&cur_chunk_coord);
    }

    coords
}
