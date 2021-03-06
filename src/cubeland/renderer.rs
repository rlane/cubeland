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

extern crate native;
extern crate gl;
extern crate cgmath;
extern crate noise;

use std;
use std::ptr;
use std::str;
use std::vec;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::matrix::Mat3;
use cgmath::matrix::Mat4;
use cgmath::matrix::ToMat4;
use cgmath::vector::EuclideanVector;
use cgmath::vector::Vector;
use cgmath::vector::Vec2;
use cgmath::vector::Vec3;
use cgmath::vector::Vec4;
use cgmath::angle::{rad, deg};
use cgmath::ptr::Ptr;

use check_gl;
use chunk;
use mesh;
use mesh::Mesh;
use chunk::Chunk;
use CHUNK_SIZE;
use texture;

static LIGHT_DIRECTION : Vec3<f32> = Vec3 { x: 0.408248, y: -0.816497, z: 0.408248 };

enum RenderMode {
    RenderModeNormal,
    RenderModeWireframe,
}

pub struct Renderer {
    res : Resources,
    window_size : Vec2<u32>,
    mode : RenderMode,
}

impl Renderer {
    pub fn new(window_size : Vec2<u32>) -> Renderer {
        let res = match Resources::load() {
            Ok(x) => x,
            Err(msg) => fail!("Error loading graphics resources: {}", msg),
        };

        check_gl("after loading graphics resources");

        Renderer {
            res: res,
            window_size: window_size,
            mode: RenderModeNormal,
        }
    }

    pub fn render(
            &self,
            chunks : &[&~chunk::Chunk],
            camera_position : Vec3<f32>,
            camera_angle : Vec2<f64>)
    {
        gl::Enable(gl::TEXTURE_2D);
        gl::Enable(gl::DEPTH_TEST);

        match self.mode {
            RenderModeNormal => {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
                gl::Enable(gl::CULL_FACE);
            },
            RenderModeWireframe => {
                gl::PolygonMode(gl::FRONT_AND_BACK, gl::LINE);
                gl::Disable(gl::CULL_FACE);
            },
        }

        gl::UseProgram(self.res.program);
        gl::ActiveTexture(gl::TEXTURE0);
        gl::Uniform1i(self.res.uniform_texture, 0);
        gl::BindTexture(gl::TEXTURE_2D, self.res.texture);
        gl::EnableVertexAttribArray(self.res.attr_position);
        gl::EnableVertexAttribArray(self.res.attr_blocktype);

        gl::Viewport(0, 0, self.window_size.x as GLint, self.window_size.y as GLint);

        gl::ClearColor(0.0, 0.75, 1.0, 1.0);
        gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

        let aspect_ratio = self.window_size.x as f32 / self.window_size.y as f32;

        let projection = cgmath::projection::perspective(
            deg(75.0f32),
            aspect_ratio,
            0.1f32, 1000.0f32);

        unsafe {
            gl::UniformMatrix4fv(self.res.uniform_projection, 1, gl::FALSE, projection.ptr());
        }

        unsafe {
            gl::Uniform3fv(self.res.uniform_light_direction, 1, LIGHT_DIRECTION.ptr());
        }

        let camera_translation = Mat4::<f32>::from_cols(
            Vec4::<f32>::unit_x(),
            Vec4::<f32>::unit_y(),
            Vec4::<f32>::unit_z(),
            camera_position.mul_s(-1.0f32).extend(1.0f32));
        let camera_rotation_x = Mat3::<f32>::from_angle_x(rad(camera_angle.x as f32)).to_mat4();
        let camera_rotation_y = Mat3::<f32>::from_angle_y(rad(camera_angle.y as f32)).to_mat4();
        let camera = camera_rotation_x.mul_m(&camera_rotation_y).mul_m(&camera_translation);

        unsafe {
            gl::Uniform3fv(self.res.uniform_camera_position, 1, camera_position.ptr());
            gl::UniformMatrix4fv(self.res.uniform_view, 1, gl::FALSE, camera.ptr());
        }

        let clip_transform = projection.mul_m(&camera);

        let camera_chunk_coord = Vec3::new(camera_position.x as i64,
                                           camera_position.y as i64,
                                           camera_position.z as i64).
                                       div_s(CHUNK_SIZE as i64);

        for chunk in chunks.iter() {
            let mut chunk_pos = Vec3::new(chunk.coord.x as f32,
                                          chunk.coord.y as f32,
                                          chunk.coord.z as f32).
                                      mul_s(CHUNK_SIZE as f32);

            /* Calculate drop due to surface curvature */
            static planet_radius : f32 = 6371000.0f32 / 5000.0f32;
            let horiz_dist = (Vec3 { x: camera_position.x, y: 0.0f32, z: camera_position.z }).
                sub_v(&Vec3::new(chunk_pos.x, 0.0f32, chunk_pos.z)).length();
            let adj_horiz_dist = (horiz_dist - 100f32).max(0.0f32);
            let drop = planet_radius - (planet_radius.powf(&2.0f32) - adj_horiz_dist.powf(&2.0f32)).sqrt();
            chunk_pos.y -= drop;

            if view_frustum_cull(&clip_transform, &chunk_pos.extend(0.0f32)) {
                continue;
            }

            let mesh : &Mesh = chunk.mesh;
            self.bind_mesh(mesh);

            unsafe {
                gl::Uniform3fv(self.res.uniform_chunk_position, 1, chunk_pos.ptr());
            }

            for face in mesh::faces.iter() {
                if !face_visible(face, chunk.coord, camera_chunk_coord) {
                    continue;
                }

                unsafe {
                    gl::Uniform3fv(self.res.uniform_normal, 1, face.normal.ptr());
                }

                let (offset, count) = mesh.face_ranges[face.index];
                unsafe {
                    gl::DrawElements(
                        gl::TRIANGLES,
                        count as i32,
                        gl::UNSIGNED_INT,
                        std::cast::transmute(
                            offset *
                            std::mem::size_of::<GLuint>()));
                }
            }
        }

        gl::UseProgram(0);
        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::PolygonMode(gl::FRONT_AND_BACK, gl::FILL);
        gl::Disable(gl::CULL_FACE);
        gl::Disable(gl::TEXTURE_2D);
        gl::Disable(gl::DEPTH_TEST);
    }

    pub fn reload_resources(&mut self) {
        match Resources::load() {
            Ok(res) => {
                self.res = res;
            },
            Err(msg) => println!("Error reloading graphics resources: {}", msg),
        }
    }

    pub fn toggle_wireframe_mode(&mut self) {
        self.mode = match self.mode {
            RenderModeWireframe => RenderModeNormal,
            _ => RenderModeWireframe
        };
    }

    pub fn set_window_size(&mut self, window_size: Vec2<u32>) {
        self.window_size = window_size;
    }

    fn bind_mesh(&self, mesh: &Mesh) {
        match (&mesh.vertex_buffer, &mesh.element_buffer) {
            (&Some(ref vbo), &Some(ref ebo)) => unsafe {
                vbo.bind();
                gl::VertexAttribPointer(self.res.attr_position, 3, gl::FLOAT,
                                        gl::FALSE as GLboolean,
                                        std::mem::size_of::<mesh::VertexData>() as GLint,
                                        ptr::null());
                gl::VertexAttribPointer(self.res.attr_blocktype, 1, gl::FLOAT,
                                        gl::FALSE as GLboolean,
                                        std::mem::size_of::<mesh::VertexData>() as GLint,
                                        std::cast::transmute(offset_of!(mesh::VertexData, blocktype)));

                ebo.bind();
            },
        _ => {}
        }
    }
}

struct Resources {
    program: GLuint,
    vertex_shader: GLuint,
    fragment_shader: GLuint,
    texture: GLuint,
    uniform_view: GLint,
    uniform_projection: GLint,
    uniform_camera_position: GLint,
    uniform_chunk_position: GLint,
    uniform_texture: GLint,
    uniform_light_direction: GLint,
    uniform_normal: GLint,
    attr_position: GLuint,
    attr_blocktype: GLuint,
}

impl Resources {
    fn load() -> Result<Resources, ~str> {
        let vs_src = std::io::fs::File::open_mode(&std::path::Path::new("shaders/main.vs.glsl"), std::io::Open, std::io::Read).unwrap().read_to_end().unwrap();
        let vs = match compile_shader(vs_src, gl::VERTEX_SHADER) {
            Ok(vs) => vs,
            Err(msg) => { return Err("vertex shader " + msg) },
        };

        let fs_src = std::io::fs::File::open_mode(&std::path::Path::new("shaders/main.fs.glsl"), std::io::Open, std::io::Read).unwrap().read_to_end().unwrap();
        let fs = match compile_shader(fs_src, gl::FRAGMENT_SHADER) {
            Ok(fs) => fs,
            Err(msg) => { return Err("fragment shader " + msg) },
        };

        let program = match link_program(vs, fs) {
            Ok(program) => program,
            Err(msg) => { return Err("linking " + msg) },
        };

        let texture = texture::make_noise_texture();

        let uniform_view = unsafe { "view".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_projection = unsafe { "projection".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_camera_position = unsafe { "camera_position".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_chunk_position = unsafe { "chunk_position".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_texture = unsafe { "texture".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_light_direction = unsafe { "light_direction".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };
        let uniform_normal = unsafe { "normal".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };

        let attr_position = unsafe { "position".with_c_str(|ptr| gl::GetAttribLocation(program, ptr) as GLuint) };
        assert!(attr_position as u32 != gl::INVALID_VALUE);
        let attr_blocktype = unsafe { "blocktype".with_c_str(|ptr| gl::GetAttribLocation(program, ptr) as GLuint) };
        assert!(attr_blocktype as u32 != gl::INVALID_VALUE);

        Ok(Resources {
            program: program,
            vertex_shader: vs,
            fragment_shader: fs,
            texture: texture,
            uniform_view: uniform_view,
            uniform_projection: uniform_projection,
            uniform_camera_position: uniform_camera_position,
            uniform_chunk_position: uniform_chunk_position,
            uniform_texture: uniform_texture,
            uniform_light_direction: uniform_light_direction,
            uniform_normal: uniform_normal,
            attr_position: attr_position,
            attr_blocktype: attr_blocktype,
        })
    }
}

impl Drop for Resources {
    fn drop(&mut self) {
        unsafe { gl::DeleteTextures(1, &self.texture); }
        gl::DeleteProgram(self.program);
        gl::DeleteShader(self.vertex_shader);
        gl::DeleteShader(self.fragment_shader);
    }
}

fn view_frustum_cull(m : &Mat4<f32>, p: &Vec4<f32>) -> bool {
    static L : f32 = CHUNK_SIZE as f32;

    static vertices : [Vec4<f32>, ..8] = [
        Vec4 { x: 0.0, y: 0.0, z: L,   w: 1.0 }, /* front bottom left */
        Vec4 { x: L,   y: 0.0, z: L,   w: 1.0 }, /* front bottom right */
        Vec4 { x: 0.0, y: L,   z: L,   w: 1.0 }, /* front top left */
        Vec4 { x: L,   y: L,   z: L,   w: 1.0 }, /* front top right */
        Vec4 { x: L,   y: 0.0, z: 0.0, w: 1.0 }, /* back bottom right */
        Vec4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 }, /* back bottom left */
        Vec4 { x: L,   y: L,   z: 0.0, w: 1.0 }, /* back top right */
        Vec4 { x: 0.0, y: L,   z: 0.0, w: 1.0 }, /* back top left */
    ];

    let clip_vertices: ~[Vec4<f32>] = vertices.iter().map(|v| m.mul_v(&p.add_v(v))).collect();

    if clip_vertices.iter().all(|v| v.x < -v.w) {
        return true;
    }

    if clip_vertices.iter().all(|v| v.x > v.w) {
        return true;
    }

    if clip_vertices.iter().all(|v| v.y < -v.w) {
        return true;
    }

    if clip_vertices.iter().all(|v| v.y > v.w) {
        return true;
    }

    if clip_vertices.iter().all(|v| v.z < -v.w) {
        return true;
    }

    if clip_vertices.iter().all(|v| v.z > v.w) {
        return true;
    }

    return false;
}

fn face_visible(face : &mesh::Face, a: Vec3<i64>, b: Vec3<i64>) -> bool {
    let dp = b.sub_v(&a);

    match face.index {
        0 => dp.z >= 0,
        1 => dp.z <= 1,
        2 => dp.x >= 0,
        3 => dp.x <= 1,
        4 => dp.y >= 0,
        5 => dp.y <= 1,
        _ => unreachable!()
    }
}

fn compile_shader(src: &[u8], ty: GLenum) -> Result<GLuint,~str> {
    let shader = gl::CreateShader(ty);
    unsafe {
        // Attempt to compile the shader
        let length = src.len() as GLint;
        let ptr : *i8 = std::cast::transmute(src.unsafe_ref(0));
        gl::ShaderSource(shader, 1, &ptr, &length);
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec::Vec::from_elem(len as uint - 1, 0u8);     // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            return Err(str::raw::from_utf8(buf.slice_from(0)).to_owned());
        }
    }
    Ok(shader)
}

fn link_program(vs: GLuint, fs: GLuint) -> Result<GLuint, ~str> {
    let program = gl::CreateProgram();
    gl::AttachShader(program, vs);
    gl::AttachShader(program, fs);
    gl::LinkProgram(program);
    unsafe {
        // Get the link status
        let mut status = gl::FALSE as GLint;
        gl::GetProgramiv(program, gl::LINK_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len: GLint = 0;
            gl::GetProgramiv(program, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec::Vec::from_elem(len as uint - 1, 0u8);     // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            return Err(str::raw::from_utf8(buf.slice_from(0)).to_owned());
        }
    }
    Ok(program)
}
