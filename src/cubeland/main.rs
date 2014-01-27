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

use extra::time::precise_time_ns;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::matrix::ToMat4;
use cgmath::vector::Vector;
use cgmath::vector::Vec2;
use cgmath::vector::Vec3;
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
mod camera;
mod terrain;
mod mesh;

pub static VISIBLE_RADIUS: uint = 12;
pub static WORLD_HEIGHT: uint = 4;
pub static CHUNK_SIZE: int = 32;
pub static WORLD_SEED: u32 = 42;

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

        let mut camera = camera::Camera::new(Vec3::new(0.0, 80.0, 40.0));

        let (key_port, key_chan) = std::comm::Chan::new();
        window.set_key_callback(~KeyContext { chan: key_chan });

        let (fb_size_port, fb_size_chan): (Port<(u32,u32)>, Chan<(u32,u32)>) = std::comm::Chan::new();
        window.set_framebuffer_size_callback(~FramebufferSizeContext { chan: fb_size_chan });

        let mut fps_display_limiter = ratelimiter::RateLimiter::new(1000*1000*1000);
        let mut fps_frame_counter = 0;

        let mut last_tick = extra::time::precise_time_ns();

        let mut grabbed = true;

        // Preload chunks
        {
            let deadline = precise_time_ns() + 1000*1000*100;
            let mut count = 0;
            request_nearby_chunks(&mut chunk_loader, camera.position);
            while precise_time_ns() < deadline {
                chunk_loader.work();
                count += 1;
            }
            println!("Preloaded {} chunks", count);
        }

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
                    // Camera movement
                    Some((glfw::Press, glfw::KeyW)) |
                    Some((glfw::Release, glfw::KeyS)) => {
                        camera.accelerate(Vec3::new(0.0, 0.0, -1.0));
                    },
                    Some((glfw::Press, glfw::KeyS)) |
                    Some((glfw::Release, glfw::KeyW)) => {
                        camera.accelerate(Vec3::new(0.0, 0.0, 1.0));
                    },
                    Some((glfw::Press, glfw::KeyA)) |
                    Some((glfw::Release, glfw::KeyD)) => {
                        camera.accelerate(Vec3::new(-1.0, 0.0, 0.0));
                    },
                    Some((glfw::Press, glfw::KeyD)) |
                    Some((glfw::Release, glfw::KeyA)) => {
                        camera.accelerate(Vec3::new(1.0, 0.0, 0.0));
                    },
                    Some((glfw::Press, glfw::KeyLeftControl)) |
                    Some((glfw::Release, glfw::KeySpace)) => {
                        camera.accelerate(Vec3::new(0.0, -1.0, 0.0));
                    },
                    Some((glfw::Press, glfw::KeySpace)) |
                    Some((glfw::Release, glfw::KeyLeftControl)) => {
                        camera.accelerate(Vec3::new(0.0, 1.0, 0.0));
                    },

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
                camera.look(Vec2 { x: cursor_x, y: cursor_y });
            }

            let now = extra::time::precise_time_ns();
            let tick_length = (now - last_tick) as f64 / (1000.0 * 1000.0 * 1000.0);
            last_tick = now;

            camera.tick(tick_length);

            {
                let chunks = find_nearby_chunks(&chunk_loader, camera.position);

                renderer.render(
                    chunks,
                    Vec3 { x: camera.position.x as f32, y: camera.position.y as f32, z: camera.position.z as f32 },
                    camera.angle)
            }

            window.swap_buffers();

            request_nearby_chunks(&mut chunk_loader, camera.position);
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

fn nearby_chunk_coords(p: Vec3<f64>) -> ~[Vec3<i64>] {
    static num_chunks : uint = (VISIBLE_RADIUS * 2 + 1) * (VISIBLE_RADIUS * 2 + 1);
    let cur_chunk_coord = Vec3::new(p.x as i64, 0, p.z as i64).div_s(CHUNK_SIZE as i64);

    let mut coords = ~[];

    for v in Spiral::<i64>::new(num_chunks) {
        if v.x*v.x + v.y*v.y > (VISIBLE_RADIUS*VISIBLE_RADIUS) as i64 {
            continue;
        }

        let mut c = cur_chunk_coord.add_v(&Vec3::new(v.x, 0, v.y));
        for y in range(0, WORLD_HEIGHT) {
            c.y = y as i64;
            coords.push(c);
        }
    }

    coords
}

fn find_nearby_chunks<'a>(chunk_loader: &'a ChunkLoader, p: Vec3<f64>) -> ~[&'a ~Chunk] {
    let coords = nearby_chunk_coords(p);
    coords.iter().
        filter_map(|&c| chunk_loader.get(c)).
        to_owned_vec()
}

fn request_nearby_chunks(chunk_loader: &mut ChunkLoader, p: Vec3<f64>) {
    let coords = nearby_chunk_coords(p);
    chunk_loader.request(coords);
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
