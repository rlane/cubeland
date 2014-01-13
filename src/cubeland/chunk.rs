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
use std::ptr;
use std::hashmap::HashMap;
use std;

use extra::time::precise_time_ns;

use gl::types::*;

use cgmath::vector::Vector;
use cgmath::vector::Vec3;

use noise::Perlin;

use CHUNK_SIZE;
use VISIBLE_RADIUS;
use GraphicsResources;

static MAX_CHUNKS : uint = (VISIBLE_RADIUS*2)*(VISIBLE_RADIUS*2)*2;

#[repr(u8)]
#[deriving(Eq)]
pub enum BlockType {
    BlockAir = 0,
    BlockGrass = 1,
    BlockStone = 2,
    BlockDirt = 3,
    BlockWater = 4,
}

pub struct ChunkLoader {
    seed : u32,
    cache : HashMap<(i64, i64, uint), ~Chunk>
}

impl ChunkLoader {
    pub fn new(seed : u32) -> ChunkLoader {
        ChunkLoader {
            seed: seed,
            cache: HashMap::new(),
        }
    }

    pub fn load(&mut self, cx : i64, cz: i64, lod: uint) {
        println!("loading chunk ({}, {}) lod={}", cx, cz, lod);
        let chunk = chunk_gen(self.seed, cx, cz, lod);
        self.cache.insert((cx, cz, lod), chunk);

        while self.cache.len() > MAX_CHUNKS {
            let (&k, _) = self.cache.iter().min_by(|&(_, chunk)| chunk.used_time).unwrap();
            self.cache.remove(&k);
        }
    }
}

pub struct Chunk {
    x: i64,
    z: i64,
    map: ~Map,
    mesh: ~Mesh,
    used_time: u64,
}

impl Chunk {
    pub fn touch(&mut self) {
        self.used_time = extra::time::precise_time_ns();
    }
}

struct Block {
    blocktype: BlockType,
}

struct Map {
    blocks: [[[Block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE],
}

struct Mesh {
    vertex_buffer: GLuint,
    normal_buffer: GLuint,
    blocktype_buffer: GLuint,
    element_buffer: GLuint,
    num_elements: uint,
}

impl Mesh {
    pub fn bind_arrays(&self, res: &GraphicsResources) {
        unsafe {
            let vert_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(res.program, ptr));
            assert!(vert_attr as u32 != gl::INVALID_VALUE);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vertex_buffer);
            gl::EnableVertexAttribArray(vert_attr as GLuint);
            gl::VertexAttribPointer(vert_attr as GLuint, 3, gl::FLOAT,
                                    gl::FALSE as GLboolean, 0, ptr::null());

            let normal_attr = "normal".with_c_str(|ptr| gl::GetAttribLocation(res.program, ptr));
            assert!(normal_attr as u32 != gl::INVALID_VALUE);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.normal_buffer);
            gl::EnableVertexAttribArray(normal_attr as GLuint);
            gl::VertexAttribPointer(normal_attr as GLuint, 3, gl::FLOAT,
                                    gl::FALSE as GLboolean, 0, ptr::null());

            let blocktype_attr = "blocktype".with_c_str(|ptr| gl::GetAttribLocation(res.program, ptr));
            assert!(blocktype_attr as u32 != gl::INVALID_VALUE);
            gl::BindBuffer(gl::ARRAY_BUFFER, self.blocktype_buffer);
            gl::EnableVertexAttribArray(blocktype_attr as GLuint);
            gl::VertexAttribPointer(blocktype_attr as GLuint, 1, gl::FLOAT,
                                    gl::FALSE as GLboolean, 0, ptr::null());

            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, self.element_buffer);
        }
    }
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteBuffers(1, &self.normal_buffer);
            gl::DeleteBuffers(1, &self.blocktype_buffer);
            gl::DeleteBuffers(1, &self.element_buffer);
        }
    }
}

struct Face {
    normal: Vec3<f32>,
    vertices: [Vec3<f32>, ..4],
}

pub fn chunk_gen(seed: u32, chunk_x: i64, chunk_z: i64, lod: uint) -> ~Chunk {
    let step = std::num::min(16u, 1 << lod);

    let def_block = Block { blocktype: BlockAir };
    let mut map = ~Map {
        blocks: [[[def_block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE],
    };

    terrain_gen(seed, chunk_x, chunk_z, step, map);

    let mesh = mesh_gen(chunk_x, chunk_z, map, step);

    return ~Chunk {
        x: chunk_x,
        z: chunk_z,
        map: map,
        mesh: mesh,
        used_time: extra::time::precise_time_ns(),
    };
}

fn block_exists(map: &Map, x: int, y: int, z: int) -> bool {
    if y < 0 {
        return true;
    }

    if x < 0 || x >= CHUNK_SIZE as int || y >= CHUNK_SIZE as int || z < 0 || z >= CHUNK_SIZE as int {
        return false;
    }

    map.blocks[x][y][z].blocktype != BlockAir
}

fn terrain_gen(seed: u32, chunk_x: i64, chunk_z: i64, step: uint, map: &mut Map) {
    let start_time = precise_time_ns();

    let perlin1 = Perlin::from_seed([seed as uint]);
    let perlin2 = Perlin::from_seed([seed as uint * 7]);
    let perlin3 = Perlin::from_seed([seed as uint * 13]);
    let perlin4 = Perlin::from_seed([seed as uint * 17]);

    for block_x in std::iter::range_step(0, CHUNK_SIZE, step) {
        for block_z in std::iter::range_step(0, CHUNK_SIZE, step) {
            let noise1 = perlin1.gen([
                (chunk_x + block_x as i64) as f64 * 0.07,
                (chunk_z + block_z as i64) as f64 * 0.04
            ]);
            let noise2 = perlin2.gen([
                (chunk_x + block_x as i64) as f64 * 0.05,
                (chunk_z + block_z as i64) as f64 * 0.05
            ]);
            let noise3 = perlin3.gen([
                (chunk_x + block_x as i64) as f64 * 0.005,
                (chunk_z + block_z as i64) as f64 * 0.005
            ]);
            let noise4 = perlin4.gen([
                (chunk_x + block_x as i64) as f64 * 0.001,
                (chunk_z + block_z as i64) as f64 * 0.001
            ]);

            let base_height = 15.0;
            let base_variance = 10.0;
            let height =
                std::num::max(
                    base_height +
                    noise4 * 10.0 +
                    base_variance *
                        std::num::pow(noise3 + 1.0, 2.5) *
                        noise1,
                    1.0) as uint;

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

                map.blocks[block_x][y][block_z] = Block { blocktype: blocktype };
            }

            let water_height = 10;
            for y in range(height, water_height) {
                map.blocks[block_x][y][block_z] = Block { blocktype: BlockWater };
            }
        }
    }

    let end_time = precise_time_ns();

    println!("terrain gen : {}us",
             (end_time - start_time)/1000);
}

fn mesh_gen(chunk_x: i64, chunk_z: i64, map: &Map, step: uint) -> ~Mesh {
    let start_time = precise_time_ns();

    let mut vertices : ~[Vec3<f32>] = ~[];
    let mut normals : ~[Vec3<f32>] = ~[];
    let mut blocktypes : ~[f32] = ~[];
    let mut elements : ~[GLuint] = ~[];

    static expected_vertices : uint = 70000;
    static expected_elements : uint = expected_vertices * 3 / 2;
    vertices.reserve(expected_vertices);
    normals.reserve(expected_vertices);
    blocktypes.reserve(expected_vertices);
    elements.reserve(expected_elements);

    let mut idx = 0;

    for x in std::iter::range_step(0, CHUNK_SIZE, step) {
        for y in std::iter::range_step(0, CHUNK_SIZE, step) {
            for z in std::iter::range_step(0, CHUNK_SIZE, step) {
                let block = &map.blocks[x][y][z];

                if (block.blocktype == BlockAir) {
                    continue;
                }

                let chunk_position = Vec3 {
                    x: chunk_x as f32,
                    y: 0.0f32,
                    z: chunk_z as f32,
                };

                let block_position = Vec3 {
                    x: x as f32,
                    y: y as f32,
                    z: z as f32,
                };

                for face in faces.iter() {
                    let neighbor_position = block_position.add_v(&face.normal.mul_s(step as f32));
                    if block_exists(map, neighbor_position.x as int, neighbor_position.y as int, neighbor_position.z as int) {
                        continue;
                    }

                    for v in face.vertices.iter() {
                        vertices.push(v.mul_s(step as f32).add_v(&block_position).add_v(&chunk_position));
                        normals.push(face.normal);
                        blocktypes.push(block.blocktype as f32);
                    }

                    for e in face_elements.iter() {
                        elements.push((idx * face.vertices.len()) as GLuint + *e);
                    }

                    idx += 1;
                }
            }
        }
    }

    let mut vertex_buffer = 0;
    let mut normal_buffer = 0;
    let mut blocktype_buffer = 0;
    let mut element_buffer = 0;

    unsafe {
        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vertex_buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (vertices.len() * std::mem::size_of::<Vec3<f32>>()) as GLsizeiptr,
                        cast::transmute(&vertices[0]),
                        gl::STATIC_DRAW);

        // Create a Vertex Buffer Object and copy the normal data to it
        gl::GenBuffers(1, &mut normal_buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, normal_buffer);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (normals.len() * std::mem::size_of::<Vec3<f32>>()) as GLsizeiptr,
                        cast::transmute(&normals[0]),
                        gl::STATIC_DRAW);

        // Create a Vertex Buffer Object and copy the blocktype data to it
        gl::GenBuffers(1, &mut blocktype_buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, blocktype_buffer);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (blocktypes.len() * std::mem::size_of::<f32>()) as GLsizeiptr,
                        cast::transmute(&blocktypes[0]),
                        gl::STATIC_DRAW);

        // Create a Vertex Buffer Object and copy the element data to it
        gl::GenBuffers(1, &mut element_buffer);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, element_buffer);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                        (elements.len() * std::mem::size_of::<GLuint>()) as GLsizeiptr,
                        cast::transmute(&elements[0]),
                        gl::STATIC_DRAW);
    }

    let end_time = precise_time_ns();

    println!("mesh gen : {}us; vertices={}; elements={}",
             (end_time - start_time)/1000,
             vertices.len(), elements.len())

    ~Mesh {
        vertex_buffer: vertex_buffer,
        normal_buffer: normal_buffer,
        blocktype_buffer: blocktype_buffer,
        element_buffer: element_buffer,
        num_elements: elements.len(),
    }
}

static face_elements : [GLuint, ..6] = [
    0, 1, 2, 3, 2, 1,
];

static faces : [Face, ..6] = [
    /* front */
    Face {
        normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* bottom left */
            Vec3 { x: 1.0, y: 0.0, z: 1.0 },  /* bottom right */
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* top left */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 },  /* top right */
        ],
    },

    /* back */
    Face {
        normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
        vertices: [
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* bottom right */
            Vec3 { x: 0.0, y: 0.0, z: 0.0 },  /* bottom left */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* top right */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },  /* top left */
        ],
    },

    /* right */
    Face {
        normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
        vertices: [
            Vec3 { x: 1.0, y: 0.0, z: 1.0 }, /* bottom front */
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* bottom back */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 }, /* top front */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* top back */
        ],
    },

    /* left */
    Face {
        normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 0.0 }, /* bottom back */
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* bottom front */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 }, /* top back */
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* top front */
        ],
    },

    /* top */
    Face {
        normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
        vertices: [
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* front left */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 }, /* front right */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 }, /* back left */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* back right */
        ],
    },

    /* bottom */
    Face {
        normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 0.0 }, /* back left */
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* back right */
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* front left */
            Vec3 { x: 1.0, y: 0.0, z: 1.0 }, /* front right */
        ],
    },
];
