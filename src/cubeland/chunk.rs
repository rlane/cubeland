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
extern mod cgmath;

use std::hashmap::HashMap;

use extra::time::precise_time_ns;

use cgmath::vector::Vector;
use cgmath::vector::Vec3;

use CHUNK_SIZE;
use VISIBLE_RADIUS;
use WORLD_HEIGHT;
use terrain::Terrain;
use mesh::Mesh;

static MAX_CHUNKS : uint = (VISIBLE_RADIUS*2)*(VISIBLE_RADIUS*2)*WORLD_HEIGHT*2;

pub struct ChunkLoader {
    seed : u32,
    cache : HashMap<(i64, i64, i64), ~Chunk>,
    needed_chunks : ~[Vec3<i64>],
}

impl ChunkLoader {
    pub fn new(seed : u32) -> ChunkLoader {
        ChunkLoader {
            seed: seed,
            cache: HashMap::new(),
            needed_chunks: ~[],
        }
    }

    pub fn get<'a>(&'a self, c: Vec3<i64>) -> Option<&'a ~Chunk> {
        self.cache.find(&(c.x, c.y, c.z))
    }

    pub fn request(&mut self, coords: &[Vec3<i64>]) {
        self.needed_chunks.clear();

        for &c in coords.iter() {
            match self.cache.find_mut(&(c.x, c.y, c.z)) {
                Some(chunk) => {
                    chunk.touch();
                }
                None => {
                    self.needed_chunks.push(c);
                }
            }
        }
    }

    pub fn work(&mut self) {
        if self.needed_chunks.is_empty() {
            return;
        }

        let coord = self.needed_chunks.shift();
        println!("loading chunk ({}, {}, {})", coord.x, coord.y, coord.z);
        let chunk = chunk_gen(self.seed, coord);
        self.cache.insert((coord.x, coord.y, coord.z), chunk);

        while self.cache.len() > MAX_CHUNKS {
            let (&k, _) = self.cache.iter().min_by(|&(_, chunk)| chunk.used_time).unwrap();
            self.cache.remove(&k);
        }
    }
}

pub struct Chunk {
    coord: Vec3<i64>,
    terrain: ~Terrain,
    mesh: ~Mesh,
    used_time: u64,
}

impl Chunk {
    pub fn touch(&mut self) {
        self.used_time = extra::time::precise_time_ns();
    }
}

pub fn chunk_gen(seed: u32, coord: Vec3<i64>) -> ~Chunk {
    let p = Vec3::new(coord.x as f64, coord.y as f64, coord.z as f64).mul_s(CHUNK_SIZE as f64);

    let terrain = Terrain::gen(seed, p);

    let mesh = Mesh::gen(terrain);

    return ~Chunk {
        coord: coord,
        terrain: terrain,
        mesh: mesh,
        used_time: extra::time::precise_time_ns(),
    };
}
