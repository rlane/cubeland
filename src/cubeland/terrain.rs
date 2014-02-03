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

extern mod cgmath;
extern mod noise;

use std;

use cgmath::array::Array;
use cgmath::vector::Vector;
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

pub struct TerrainGenerator {
    perlin1 : Perlin,
    perlin2 : Perlin,
    perlin3 : Perlin,
    perlin4 : Perlin,
}

pub struct Terrain {
    priv blocks: [[[Block, ..CHUNK_SIZE+2], ..CHUNK_SIZE+2], ..CHUNK_SIZE+2],
}

impl TerrainGenerator {
    pub fn new(seed: u32) -> TerrainGenerator {
        TerrainGenerator {
            perlin1: Perlin::from_seed([seed as uint]),
            perlin2: Perlin::from_seed([seed as uint * 7]),
            perlin3: Perlin::from_seed([seed as uint * 13]),
            perlin4: Perlin::from_seed([seed as uint * 17]),
        }
    }

    pub fn gen(&self, p: Vec3<f64>) -> ~Terrain {
        let def_block = Block { blocktype: BlockAir };
        let mut t = ~Terrain {
            blocks: [[[def_block, ..CHUNK_SIZE+2], ..CHUNK_SIZE+2], ..CHUNK_SIZE+2],
        };

        static S : int = 4;

        let mut density = [[[0.0, ..(CHUNK_SIZE/S)+3], ..(CHUNK_SIZE/S)+3], ..(CHUNK_SIZE/S)+3];
        for density_x in std::iter::range(-1, CHUNK_SIZE/S+1) {
            for density_y in std::iter::range(-1, CHUNK_SIZE/S+1) {
                for density_z in std::iter::range(-1, CHUNK_SIZE/S+1) {
                    let v = Vec3::new(p.x + (density_x * S) as f64,
                                      p.y + (density_y * S) as f64,
                                      p.z + (density_z * S) as f64);
                    let warp_v = v.mul_v(&Vec3::new(0.02, 0.03, 0.02));
                    let warp = Vec3::new(
                        self.perlin2.gen(warp_v.as_slice()),
                        self.perlin3.gen(warp_v.as_slice()),
                        self.perlin4.gen(warp_v.as_slice())).mul_s(2.0);
                    let v2 = v.mul_v(&Vec3::new(0.012, 0.020, 0.025)).add_v(&warp);
                    density[density_x+1][density_y+1][density_z+1] =
                        self.perlin1.gen(v2.as_slice()) * 0.5 + 0.5;
                }
            }
        }

        let water_height = -12.0;
        let base_variance = 10.0;

        for block_x in std::iter::range(-1, CHUNK_SIZE+1) {
            for block_z in std::iter::range(-1, CHUNK_SIZE+1) {
                let noise1 = self.perlin1.gen([
                    (p.x + block_x as f64) * 0.07,
                    (p.z + block_z as f64) * 0.04
                ]);
                let noise2 = self.perlin2.gen([
                    (p.x + block_x as f64) * 0.05,
                    (p.z + block_z as f64) * 0.05
                ]);
                let noise3 = self.perlin3.gen([
                    (p.x + block_x as f64) * 0.005,
                    (p.z + block_z as f64) * 0.005
                ]);
                let noise4 = self.perlin4.gen([
                    (p.x + block_x as f64) * 0.001,
                    (p.z + block_z as f64) * 0.001
                ]);

                let height =
                    noise4 * 10.0 +
                    base_variance *
                        (noise3 + 1.0).powf(&2.5) *
                        noise1;

                let dirt_height = 4.0 + noise2 * 8.0;

                for block_y in range(-1, CHUNK_SIZE+1) {
                    let mut blocktype = BlockAir;
                    let v = p.add_v(&Vec3::new(block_x as f64, block_y as f64, block_z as f64));

                    if v.y < height {
                        if v.y > height - dirt_height {
                            if v.y > height - 2.0 {
                                blocktype = BlockGrass;
                            } else {
                                blocktype = BlockDirt;
                            }
                        } else {
                            blocktype = BlockStone;
                        }
                    }

                    if blocktype == BlockAir && v.y < water_height {
                        blocktype = BlockWater;
                    }

                    if blocktype != BlockAir && blocktype != BlockWater {
                        /* Trilinear interpolation of lower-resolution density */
                        let fx = (block_x as f64 / S as f64).fract();
                        let fy = (block_y as f64 / S as f64).fract();
                        let fz = (block_z as f64 / S as f64).fract();
                        let x = (block_x+S)/S;
                        let y = (block_y+S)/S;
                        let z = (block_z+S)/S;
                        let dxyz = density[x][y][z];
                        let dxyZ = density[x][y][z+1];
                        let dxYz = density[x][y+1][z];
                        let dxYZ = density[x][y+1][z+1];
                        let dXyz = density[x+1][y][z];
                        let dXyZ = density[x+1][y][z+1];
                        let dXYz = density[x+1][y+1][z];
                        let dXYZ = density[x+1][y+1][z+1];

                        let d = dxyz * (1.0-fx) * (1.0-fy) * (1.0-fz) +
                                dxyZ * (1.0-fx) * (1.0-fy) * fz +
                                dxYz * (1.0-fx) * fy * (1.0-fz) +
                                dxYZ * (1.0-fx) * fy * fz +
                                dXyz * fx * (1.0-fy) * (1.0-fz) +
                                dXyZ * fx * (1.0-fy) * fz +
                                dXYz * fx * fy * (1.0-fz) +
                                dXYZ * fx * fy * fz;

                        if d < 0.25 {
                            blocktype = BlockAir;
                        }
                    }

                    if blocktype != BlockAir {
                        let block = t.get_mut(block_x, block_y, block_z);
                        block.blocktype = blocktype;
                    }
                }
            }
        }

        return t;
    }
}

impl Terrain {
    pub fn get<'a>(&'a self, x: int, y: int, z: int) -> &'a Block {
        &self.blocks[x+1][y+1][z+1]
    }

    pub fn get_mut<'a>(&'a mut self, x: int, y: int, z: int) -> &'a mut Block {
        &mut self.blocks[x+1][y+1][z+1]
    }
}
