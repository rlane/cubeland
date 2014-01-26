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
extern mod noise;

use std;
use std::num::clamp;

use extra::time::precise_time_ns;

use cgmath::vector::Vec3;

use noise::Perlin;

use CHUNK_SIZE;
use chunk::Block;
use chunk::{BlockAir,BlockDirt,BlockStone,BlockWater,BlockGrass};

pub struct Terrain {
    blocks: [[[Block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE],
}

impl Terrain {
    pub fn gen(seed: u32, p: Vec3<f64>) -> ~Terrain {
        let def_block = Block { blocktype: BlockAir };
        let mut t = ~Terrain {
            blocks: [[[def_block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE],
        };

        let start_time = precise_time_ns();

        let perlin1 = Perlin::from_seed([seed as uint]);
        let perlin2 = Perlin::from_seed([seed as uint * 7]);
        let perlin3 = Perlin::from_seed([seed as uint * 13]);
        let perlin4 = Perlin::from_seed([seed as uint * 17]);

        for block_x in std::iter::range(0, CHUNK_SIZE) {
            for block_z in std::iter::range(0, CHUNK_SIZE) {
                let noise1 = perlin1.gen([
                    (p.x + block_x as f64) * 0.07,
                    (p.z + block_z as f64) * 0.04
                ]);
                let noise2 = perlin2.gen([
                    (p.x + block_x as f64) * 0.05,
                    (p.z + block_z as f64) * 0.05
                ]);
                let noise3 = perlin3.gen([
                    (p.x + block_x as f64) * 0.005,
                    (p.z + block_z as f64) * 0.005
                ]);
                let noise4 = perlin4.gen([
                    (p.x + block_x as f64) * 0.001,
                    (p.z + block_z as f64) * 0.001
                ]);

                let base_height = 15.0;
                let base_variance = 10.0;
                let height = clamp(
                    (
                        base_height +
                        noise4 * 10.0 +
                        base_variance *
                            std::num::pow(noise3 + 1.0, 2.5) *
                            noise1
                    ) as int,
                    1, CHUNK_SIZE as int - 1) as uint;

                for y in range(0, height) {
                    let mut blocktype = BlockStone;

                    let dirt_height = (4.0 + noise2 * 8.0) as uint;
                    if (height <= 20) && (y + dirt_height >= height) {
                        if y < height - 2 {
                            blocktype = BlockDirt;
                        } else {
                            blocktype = BlockGrass;
                        }
                    }

                    t.blocks[block_x][y][block_z] = Block { blocktype: blocktype };
                }

                let water_height = 10;
                for y in range(height, water_height) {
                    t.blocks[block_x][y][block_z] = Block { blocktype: BlockWater };
                }

                for block_y in std::iter::range(0, CHUNK_SIZE) {
                    let block = &mut t.blocks[block_x][block_y][block_z];

                    if (p.y + block_y as f64) <= 1.0 ||
                       block.blocktype == BlockAir {
                        continue;
                    }

                    let cave = perlin1.gen([
                        (p.x + block_x as f64) * 0.05,
                        (p.y + block_y as f64) * 0.1,
                        (p.z + block_z as f64) * 0.05]);

                    if cave > 0.5 {
                        block.blocktype = BlockAir;
                    }
                }

            }
        }

        let end_time = precise_time_ns();

        println!("terrain gen : {}us",
                (end_time - start_time)/1000);

        return t;
    }

    pub fn index<'a>(&'a self, x: int, y: int, z: int) -> Option<&'a Block> {
        if x < 0 || x >= CHUNK_SIZE as int || y < 0 || y >= CHUNK_SIZE as int || z < 0 || z >= CHUNK_SIZE as int {
            None
        } else {
            Some(&self.blocks[x][y][z])
        }
    }
}
