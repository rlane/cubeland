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

use std;
use std::cast;
use std::hashmap::HashMap;

use extra::time::precise_time_ns;
use extra::bitv::BitvSet;

use gl::types::*;

use cgmath::vector::Vector;
use cgmath::vector::Vec3;

use CHUNK_SIZE;
use VISIBLE_RADIUS;
use terrain::Terrain;

static NUM_FACES : uint = 6;
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

// Layout of the vertex buffer sent to the GPU
pub struct VertexData {
    position : Vec3<f32>,
    blocktype : f32,
}

pub struct ChunkLoader {
    seed : u32,
    cache : HashMap<Vec3<i64>, ~Chunk>,
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
        self.cache.find(&c)
    }

    pub fn request(&mut self, c: Vec3<i64>) {
        match self.cache.find_mut(&c) {
            Some(chunk) => {
                chunk.touch();
            }
            None => {
                if !self.needed_chunks.contains(&c) {
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
        println!("loading chunk ({}, {})", coord.x, coord.y);
        let chunk = chunk_gen(self.seed, coord);
        self.cache.insert(coord, chunk);

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

pub struct Block {
    blocktype: BlockType,
}

impl Block {
    pub fn is_opaque(&self) -> bool {
        self.blocktype != BlockAir
    }
}

pub struct Mesh {
    vertex_buffer: GLuint,
    element_buffer: GLuint,
    face_ranges: [(uint, uint), ..NUM_FACES],
}

impl Drop for Mesh {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vertex_buffer);
            gl::DeleteBuffers(1, &self.element_buffer);
        }
    }
}

pub struct Face {
    index: uint,
    normal: Vec3<f32>,
    di: Vec3<uint>,
    dj: Vec3<uint>,
    dk: Vec3<uint>,
    vertices: [Vec3<f32>, ..4],
}

pub fn chunk_gen(seed: u32, coord: Vec3<i64>) -> ~Chunk {
    let p = Vec3::new(coord.x as f64, coord.y as f64, coord.z as f64).mul_s(CHUNK_SIZE as f64);

    let terrain = Terrain::gen(seed, p);

    let mesh = mesh_gen(terrain);

    return ~Chunk {
        coord: coord,
        terrain: terrain,
        mesh: mesh,
        used_time: extra::time::precise_time_ns(),
    };
}

fn block_exists(t: &Terrain, x: int, y: int, z: int) -> bool {
    if y < 0 {
        return true;
    }

    match t.index(x, y, z) {
        Some(block) => block.is_opaque(),
        None => false
    }
}

fn mesh_gen(t: &Terrain) -> ~Mesh {
    let start_time = precise_time_ns();

    let mut vertices : ~[VertexData] = ~[];
    let mut elements : ~[GLuint] = ~[];

    static expected_vertices : uint = 8000;
    static expected_elements : uint = expected_vertices * 3 / 2;
    vertices.reserve(expected_vertices);
    elements.reserve(expected_elements);

    let mut face_ranges = [(0, 0), ..6];

    for face in faces.iter() {
        let num_elements_start = elements.len();

        let face_normal_int = Vec3 { x: face.normal.x as int, y: face.normal.y as int, z: face.normal.z as int };

        let mut unmeshed_faces = BlockBitmap::new();
        for x in std::iter::range(0, CHUNK_SIZE) {
            for y in std::iter::range(0, CHUNK_SIZE) {
                for z in std::iter::range(0, CHUNK_SIZE) {
                    let block = &t.blocks[x][y][z];

                    if (block.blocktype == BlockAir) {
                        continue;
                    }

                    if block_exists(t,
                                    x as int + face_normal_int.x,
                                    y as int + face_normal_int.y,
                                    z as int + face_normal_int.z) {
                        continue;
                    }

                    unmeshed_faces.insert(x, y, z);
                }
            }
        }

        for i in std::iter::range(0, CHUNK_SIZE) {
            for j in std::iter::range(0, CHUNK_SIZE) {
                for k in std::iter::range(0, CHUNK_SIZE) {
                    let Vec3 { x: x, y: y, z: z } = face.di.mul_s(i).add_v(&face.dj.mul_s(j)).add_v(&face.dk.mul_s(k));
                    let block = &t.blocks[x][y][z];

                    if !unmeshed_faces.contains(x, y, z) {
                        continue;
                    }

                    let block_position = Vec3 {
                        x: x as f32,
                        y: y as f32,
                        z: z as f32,
                    };

                    let dim = expand_face(t, &unmeshed_faces, face, Vec3 { x: x, y: y, z: z });
                    let dim_f = Vec3 { x: dim.x as f32, y: dim.y as f32, z: dim.z as f32 };

                    for dx in range(0, dim.x) {
                        for dy in range(0, dim.y) {
                            for dz in range(0, dim.z) {
                                unmeshed_faces.remove(x + dx, y + dy, z + dz);
                            }
                        }
                    }

                    let vertex_offset = vertices.len();
                    for v in face.vertices.iter() {
                        vertices.push(VertexData {
                            position: v.mul_v(&dim_f).add_v(&block_position),
                            blocktype: block.blocktype as f32,
                        });
                    }

                    for e in face_elements.iter() {
                        elements.push(vertex_offset as GLuint + *e);
                    }
                }
            }
        }

        face_ranges[face.index] = (num_elements_start, elements.len() - num_elements_start);
    }

    let mut vertex_buffer = 0;
    let mut element_buffer = 0;

    if !elements.is_empty() {
        unsafe {
            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::GenBuffers(1, &mut vertex_buffer);
            gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
            gl::BufferData(gl::ARRAY_BUFFER,
                        (vertices.len() * std::mem::size_of::<VertexData>()) as GLsizeiptr,
                        cast::transmute(&vertices[0]),
                        gl::STATIC_DRAW);

            // Create a Vertex Buffer Object and copy the element data to it
            gl::GenBuffers(1, &mut element_buffer);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, element_buffer);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                        (elements.len() * std::mem::size_of::<GLuint>()) as GLsizeiptr,
                        cast::transmute(&elements[0]),
                        gl::STATIC_DRAW);
        }
    }

    let end_time = precise_time_ns();

    println!("mesh gen : {}us; vertices={}; elements={}",
             (end_time - start_time)/1000,
             vertices.len(), elements.len())

    ~Mesh {
        vertex_buffer: vertex_buffer,
        element_buffer: element_buffer,
        face_ranges: face_ranges,
    }
}

fn expand_face(t : &Terrain,
               unmeshed_faces : &BlockBitmap,
               face: &Face,
               p: Vec3<uint>) -> Vec3<uint> {

    let len_k = run_length(t, unmeshed_faces, p, face.dk);
    let len_j = range(0, len_k).
        map(|k| run_length(t, unmeshed_faces, p.add_v(&face.dk.mul_s(k)), face.dj)).
        min().unwrap();

    (Vec3 { x: 1, y: 1, z: 1 }).
        add_v(&face.dk.mul_s(len_k - 1)).
        add_v(&face.dj.mul_s(len_j - 1))
}

fn run_length(t : &Terrain,
              unmeshed_faces : &BlockBitmap,
              mut p: Vec3<uint>,
              dp: Vec3<uint>) -> uint {
    let block = &t.blocks[p.x][p.y][p.z];
    let mut len = 1;

    loop {
        p.add_self_v(&dp);
        if unmeshed_faces.contains(p.x, p.y, p.z) {
            match t.index(p.x as int, p.y as int, p.z as int) {
                Some(b) if b.blocktype == block.blocktype => {
                    len += 1;
                }
                _ => {
                    break;
                }
            }
        } else {
            break;
        }
    }

    len
}

struct BlockBitmap {
    set : BitvSet
}

impl BlockBitmap {
    pub fn new() -> BlockBitmap {
        BlockBitmap {
            set: BitvSet::new()
        }
    }

    pub fn contains(&self, x: uint, y: uint, z: uint) -> bool {
        self.set.contains(&BlockBitmap::index(x, y, z))
    }

    pub fn insert(&mut self, x: uint, y: uint, z: uint) {
        self.set.insert(BlockBitmap::index(x, y, z));
    }

    pub fn remove(&mut self, x: uint, y: uint, z: uint) {
        self.set.remove(&BlockBitmap::index(x, y, z));
    }

    fn index(x: uint, y: uint, z: uint) -> uint {
        x*CHUNK_SIZE*CHUNK_SIZE + y*CHUNK_SIZE + z
    }
}

static face_elements : [GLuint, ..6] = [
    0, 1, 2, 3, 2, 1,
];

pub static faces : [Face, ..NUM_FACES] = [
    /* front */
    Face {
        index: 0,
        normal: Vec3 { x: 0.0, y: 0.0, z: 1.0 },
        di: Vec3 { x: 0, y: 0, z: 1 },
        dj: Vec3 { x: 1, y: 0, z: 0 },
        dk: Vec3 { x: 0, y: 1, z: 0 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* bottom left */
            Vec3 { x: 1.0, y: 0.0, z: 1.0 },  /* bottom right */
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* top left */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 },  /* top right */
        ],
    },

    /* back */
    Face {
        index: 1,
        normal: Vec3 { x: 0.0, y: 0.0, z: -1.0 },
        di: Vec3 { x: 0, y: 0, z: 1 },
        dj: Vec3 { x: 1, y: 0, z: 0 },
        dk: Vec3 { x: 0, y: 1, z: 0 },
        vertices: [
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* bottom right */
            Vec3 { x: 0.0, y: 0.0, z: 0.0 },  /* bottom left */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* top right */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 },  /* top left */
        ],
    },

    /* right */
    Face {
        index: 2,
        normal: Vec3 { x: 1.0, y: 0.0, z: 0.0 },
        di: Vec3 { x: 1, y: 0, z: 0 },
        dj: Vec3 { x: 0, y: 1, z: 0 },
        dk: Vec3 { x: 0, y: 0, z: 1 },
        vertices: [
            Vec3 { x: 1.0, y: 0.0, z: 1.0 }, /* bottom front */
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* bottom back */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 }, /* top front */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* top back */
        ],
    },

    /* left */
    Face {
        index: 3,
        normal: Vec3 { x: -1.0, y: 0.0, z: 0.0 },
        di: Vec3 { x: 1, y: 0, z: 0 },
        dj: Vec3 { x: 0, y: 1, z: 0 },
        dk: Vec3 { x: 0, y: 0, z: 1 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 0.0 }, /* bottom back */
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* bottom front */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 }, /* top back */
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* top front */
        ],
    },

    /* top */
    Face {
        index: 4,
        normal: Vec3 { x: 0.0, y: 1.0, z: 0.0 },
        di: Vec3 { x: 0, y: 1, z: 0 },
        dj: Vec3 { x: 1, y: 0, z: 0 },
        dk: Vec3 { x: 0, y: 0, z: 1 },
        vertices: [
            Vec3 { x: 0.0, y: 1.0, z: 1.0 }, /* front left */
            Vec3 { x: 1.0, y: 1.0, z: 1.0 }, /* front right */
            Vec3 { x: 0.0, y: 1.0, z: 0.0 }, /* back left */
            Vec3 { x: 1.0, y: 1.0, z: 0.0 }, /* back right */
        ],
    },

    /* bottom */
    Face {
        index: 5,
        normal: Vec3 { x: 0.0, y: -1.0, z: 0.0 },
        di: Vec3 { x: 0, y: 1, z: 0 },
        dj: Vec3 { x: 1, y: 0, z: 0 },
        dk: Vec3 { x: 0, y: 0, z: 1 },
        vertices: [
            Vec3 { x: 0.0, y: 0.0, z: 0.0 }, /* back left */
            Vec3 { x: 1.0, y: 0.0, z: 0.0 }, /* back right */
            Vec3 { x: 0.0, y: 0.0, z: 1.0 }, /* front left */
            Vec3 { x: 1.0, y: 0.0, z: 1.0 }, /* front right */
        ],
    },
];
