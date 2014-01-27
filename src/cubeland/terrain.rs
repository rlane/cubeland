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

#[repr(u8)]
#[deriving(Eq)]
pub enum BlockType {
    BlockAir = 0,
    BlockGrass = 1,
    BlockStone = 2,
    BlockDirt = 3,
    BlockWater = 4,
}

pub struct Block {
    blocktype: BlockType,
}

impl Block {
    pub fn is_opaque(&self) -> bool {
        self.blocktype != BlockAir
    }
}

pub struct Terrain {
    priv blocks: [[[Block, ..CHUNK_SIZE+2], ..CHUNK_SIZE+2], ..CHUNK_SIZE+2],
}

impl Terrain {
    pub fn gen(seed: u32, p: Vec3<f64>) -> ~Terrain {
        let def_block = Block { blocktype: BlockAir };
        let mut t = ~Terrain {
            blocks: [[[def_block, ..CHUNK_SIZE+2], ..CHUNK_SIZE+2], ..CHUNK_SIZE+2],
        };

        let start_time = precise_time_ns();

        let perlin1 = Perlin::from_seed([seed as uint]);
        let perlin2 = Perlin::from_seed([seed as uint * 7]);
        let perlin3 = Perlin::from_seed([seed as uint * 13]);
        let perlin4 = Perlin::from_seed([seed as uint * 17]);

        for block_x in std::iter::range(-1, CHUNK_SIZE+1) {
            for block_z in std::iter::range(-1, CHUNK_SIZE+1) {
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
                    1, CHUNK_SIZE - 1);

                for y in range(-1, height) {
                    let block = t.get_mut(block_x, y, block_z);

                    let dirt_height = (4.0 + noise2 * 8.0) as int;
                    if (height <= 20) && (y + dirt_height >= height) {
                        if y < height - 2 {
                            block.blocktype = BlockDirt;
                        } else {
                            block.blocktype = BlockGrass;
                        }
                    } else {
                        block.blocktype = BlockStone;
                    }
                }

                let water_height = 10;
                for y in range(height, water_height) {
                    let block = t.get_mut(block_x, y, block_z);
                    block.blocktype = BlockWater;
                }

                for block_y in std::iter::range(-1, CHUNK_SIZE+1) {
                    let block = t.get_mut(block_x, block_y, block_z);

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

    pub fn get<'a>(&'a self, x: int, y: int, z: int) -> Option<&'a Block> {
        if x < -1 || x >= CHUNK_SIZE+1 || y < -1 || y >= CHUNK_SIZE+1 || z < -1 || z >= CHUNK_SIZE+1 {
            None
        } else {
            Some(&self.blocks[x+1][y+1][z+1])
        }
    }

    pub fn get_mut<'a>(&'a mut self, x: int, y: int, z: int) -> &'a mut Block {
        &mut self.blocks[x+1][y+1][z+1]
    }

}
