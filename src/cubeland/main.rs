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

extern mod native;
extern mod extra;
extern mod glfw;
extern mod gl;
extern mod cgmath;
extern mod noise;

use std::libc;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::matrix::Mat3;
use cgmath::matrix::ToMat4;
use cgmath::vector::Vector;
use cgmath::vector::Vec2;
use cgmath::vector::Vec3;
use cgmath::angle::rad;
use cgmath::ptr::Ptr;

use spiral::Spiral;
use chunk::Chunk;
use chunk::ChunkLoader;

#[cfg(target_os = "linux")]
#[link(name="GLU")]
extern {}

mod offset_of;
mod chunk;
mod ratelimiter;
mod texture;
mod spiral;
mod renderer;

pub static VISIBLE_RADIUS: uint = 12;
pub static CHUNK_SIZE: uint = 32;
pub static WORLD_SEED: u32 = 42;

static CAMERA_SPEED : f32 = 30.0f32;

static DEFAULT_WINDOW_SIZE : Vec2<u32> = Vec2 { x: 800, y: 600 };

#[start]
fn start(argc: int, argv: **u8) -> int {
    do native::start(argc, argv) {
        main();
    }
}

fn main() {
   glfw::set_error_callback(~ErrorContext);

    do glfw::start {
        glfw::window_hint::samples(8);

        let window = glfw::Window::create(
            DEFAULT_WINDOW_SIZE.x, DEFAULT_WINDOW_SIZE.y,
            "Cubeland", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        window.set_cursor_mode(glfw::CursorDisabled);
        window.make_context_current();

        gl::load_with(glfw::get_proc_address);

        glfw::set_swap_interval(1);

        let mut renderer = renderer::Renderer::new(DEFAULT_WINDOW_SIZE);

        let mut chunk_loader = ChunkLoader::new(WORLD_SEED);

        let (key_port, key_chan) = std::comm::Chan::new();
        window.set_key_callback(~KeyContext { chan: key_chan });

        let (fb_size_port, fb_size_chan): (Port<(u32,u32)>, Chan<(u32,u32)>) = std::comm::Chan::new();
        window.set_framebuffer_size_callback(~FramebufferSizeContext { chan: fb_size_chan });

        let mut fps_display_limiter = ratelimiter::RateLimiter::new(1000*1000*1000);
        let mut fps_frame_counter = 0;

        let mut camera_position = Vec3::<f32>::new(0.0f32, 30.0f32, 40.0f32);

        let mut last_tick = extra::time::precise_time_ns();

        let mut grabbed = true;

        let mut camera_angle_x = 0.0;
        let mut camera_angle_y = 0.0;

        while !window.should_close() {
            glfw::poll_events();

            loop {
                match fb_size_port.try_recv() {
                    Some((w,h)) => {
                        renderer.set_window_size(Vec2 { x: w, y: h });
                    }
                    None => break
                }
            }

            loop {
                match key_port.try_recv() {
                    Some((glfw::Press, glfw::KeyR)) => {
                        renderer.reload_resources();
                    },
                    Some((glfw::Press, glfw::KeyEscape)) => {
                        window.set_should_close(true);
                    },
                    Some((glfw::Press, glfw::KeyG)) => {
                        grabbed = !grabbed;
                        if grabbed {
                            window.set_cursor_mode(glfw::CursorDisabled);
                        } else {
                            window.set_cursor_mode(glfw::CursorNormal);
                        }
                    },
                    Some((glfw::Press, glfw::KeyL)) => {
                        renderer.toggle_wireframe_mode();
                    },
                    None => break,
                    _ => {}
                }
            }

            if grabbed {
                let (cursor_x, cursor_y) = window.get_cursor_pos();
                camera_angle_y = ((cursor_x * 0.0005) % 1.0) * std::f64::consts::PI * 2.0;
                camera_angle_x = ((cursor_y * 0.0005) % 1.0) * std::f64::consts::PI * 2.0;
            }

            let mut camera_velocity = Vec3::<f32>::new(0.0f32, 0.0f32, 0.0f32);

            match window.get_key(glfw::KeySpace) {
                glfw::Press => { camera_velocity.y += 1.0f32 }
                _ => {}
            }

            match window.get_key(glfw::KeyLeftControl) {
                glfw::Press => { camera_velocity.y += -1.0f32 }
                _ => {}
            }

            match window.get_key(glfw::KeyS) {
                glfw::Press => { camera_velocity.z += 1.0f32 }
                _ => {}
            }

            match window.get_key(glfw::KeyW) {
                glfw::Press => { camera_velocity.z += -1.0f32 }
                _ => {}
            }

            match window.get_key(glfw::KeyD) {
                glfw::Press => { camera_velocity.x += 1.0f32 }
                _ => {}
            }

            match window.get_key(glfw::KeyA) {
                glfw::Press => { camera_velocity.x += -1.0f32 }
                _ => {}
            }

            let now = extra::time::precise_time_ns();
            let tick_length = (now - last_tick) as f32 / (1000 * 1000 * 1000) as f32;
            last_tick = now;

            let inv_camera_rotation = Mat3::<f32>::from_euler(rad(-camera_angle_x as f32), rad(-camera_angle_y as f32), rad(0.0f32));
            let absolute_camera_velocity = inv_camera_rotation.mul_v(&camera_velocity).mul_s(CAMERA_SPEED).mul_s(tick_length);
            camera_position.add_self_v(&absolute_camera_velocity);

            let camera_position_i64 = Vec3 {
                x: camera_position.x as i64,
                y: 0,
                z: camera_position.z as i64
            };

            {
                let chunks = find_nearby_chunks(&chunk_loader, camera_position_i64);

                renderer.render(
                    chunks,
                    camera_position,
                    Vec2 { x: camera_angle_x, y: camera_angle_y });
            }

            window.swap_buffers();

            request_nearby_chunks(&mut chunk_loader, camera_position_i64);
            chunk_loader.work();

            check_gl("main loop");

            fps_frame_counter += 1;
            if fps_display_limiter.limit() {
                println!("{} frames per second", fps_frame_counter);
                fps_frame_counter = 0;
            }
        }
    }
}

fn nearby_chunk_coords(p: Vec3<i64>) -> ~[(i64, i64)] {
    static num_chunks : uint = (VISIBLE_RADIUS * 2 + 1) * (VISIBLE_RADIUS * 2 + 1);
    static mask : i64 = !(CHUNK_SIZE as i64 - 1);

    let chunk_coord = |v: Vec2<i64>| -> (i64, i64) {
        (
            (p.x & mask) + v.x*CHUNK_SIZE as i64,
            (p.z & mask) + v.y*CHUNK_SIZE as i64
        )
    };

    Spiral::<i64>::new(num_chunks).map(chunk_coord).to_owned_vec()
}

fn find_nearby_chunks<'a>(chunk_loader: &'a ChunkLoader, p: Vec3<i64>) -> ~[&'a ~Chunk] {
    let coords = nearby_chunk_coords(p);
    coords.iter().
        filter_map(|&c| chunk_loader.get(c)).
        to_owned_vec()
}

fn request_nearby_chunks(chunk_loader: &mut ChunkLoader, p: Vec3<i64>) {
    let coords = nearby_chunk_coords(p);
    for &c in coords.iter() {
        chunk_loader.request(c);
    }
}

extern "C" {
    fn gluErrorString(error: GLenum) -> *GLubyte;
}

fn check_gl(message : &str) {
    let err = gl::GetError();
    if (err != gl::NO_ERROR) {
        unsafe {
            let err = std::str::raw::from_c_str(gluErrorString(err) as *i8);
            fail!("GL error {} at {}", err, message);
        }
    }
}

struct ErrorContext;
impl glfw::ErrorCallback for ErrorContext {
    fn call(&self, _: glfw::Error, description: ~str) {
        fail!("GLFW Error: {:s}", description);
    }
}

struct KeyContext {
    chan : Chan<(glfw::Action, glfw::Key)>,
}
impl glfw::KeyCallback for KeyContext {
    fn call(&self, _: &glfw::Window, key: glfw::Key, _: libc::c_int, action: glfw::Action, _: glfw::Modifiers) {
        self.chan.send((action, key));
    }
}

struct FramebufferSizeContext {
    chan: Chan<(u32,u32)>
}
impl glfw::FramebufferSizeCallback for FramebufferSizeContext {
    fn call(&self, _: &glfw::Window, width: i32, height: i32) {
        self.chan.send((width as u32,height as u32));
    }
}
