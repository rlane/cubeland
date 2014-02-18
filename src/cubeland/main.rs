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

extern crate native;
extern crate extra;
extern crate collections;
extern crate sync;
extern crate glfw = "glfw-rs";
extern crate gl;
extern crate cgmath;
extern crate noise;

use extra::time::precise_time_ns;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::vector::Vector;
use cgmath::vector::Vec2;
use cgmath::vector::Vec3;

use chunk::Chunk;
use chunk::ChunkLoader;

#[cfg(target_os = "linux")]
#[link(name="GLU")]
#[link(name="glfw")]
extern {}

mod offset_of;
mod chunk;
mod ratelimiter;
mod texture;
mod renderer;
mod camera;
mod terrain;
mod mesh;

pub static VISIBLE_RADIUS: uint = 8;
pub static CHUNK_SIZE: int = 32;
pub static WORLD_SEED: u32 = 42;

static DEFAULT_WINDOW_SIZE : Vec2<u32> = Vec2 { x: 800, y: 600 };

#[start]
fn start(argc: int, argv: **u8) -> int {
    native::start(argc, argv, main)
}

fn main() {
   glfw::set_error_callback(~ErrorContext);

    glfw::start(proc() {
        glfw::window_hint::samples(8);

        let window = glfw::Window::create(
            DEFAULT_WINDOW_SIZE.x, DEFAULT_WINDOW_SIZE.y,
            "Cubeland", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        window.set_cursor_mode(glfw::CursorDisabled);
        window.set_all_polling(true);
        window.make_context_current();

        gl::load_with(glfw::get_proc_address);

        glfw::set_swap_interval(1);

        let mut renderer = renderer::Renderer::new(DEFAULT_WINDOW_SIZE);

        let mut chunk_loader = ChunkLoader::new(WORLD_SEED);

        let mut camera = camera::Camera::new(Vec3::new(0.0, 20.0, 00.0));

        let mut fps_display_limiter = ratelimiter::RateLimiter::new(1000*1000*1000);
        let mut fps_frame_counter = 0;

        let mut last_tick = extra::time::precise_time_ns();

        let mut grabbed = true;

        // Preload chunks
        {
            let deadline = precise_time_ns() + 1000*1000*100;
            request_nearby_chunks(&mut chunk_loader, camera.position);
            while precise_time_ns() < deadline {
                chunk_loader.work();
                std::task::deschedule();
            }
            println!("Preloaded {} chunks", chunk_loader.cache.len());
        }

        while !window.should_close() {
            glfw::poll_events();
            for (_, event) in window.flush_events() {
                match event {
                    glfw::FramebufferSizeEvent(w, h) => {
                        renderer.set_window_size(Vec2 { x: w as u32, y: h as u32 });
                    },
                    glfw::KeyEvent(key, _, action, _) => {
                        match (action, key) {
                            // Camera movement
                            (glfw::Press, glfw::KeyW) |
                            (glfw::Release, glfw::KeyS) => {
                                camera.accelerate(Vec3::new(0.0, 0.0, -1.0));
                            },
                            (glfw::Press, glfw::KeyS) |
                            (glfw::Release, glfw::KeyW) => {
                                camera.accelerate(Vec3::new(0.0, 0.0, 1.0));
                            },
                            (glfw::Press, glfw::KeyA) |
                            (glfw::Release, glfw::KeyD) => {
                                camera.accelerate(Vec3::new(-1.0, 0.0, 0.0));
                            },
                            (glfw::Press, glfw::KeyD) |
                            (glfw::Release, glfw::KeyA) => {
                                camera.accelerate(Vec3::new(1.0, 0.0, 0.0));
                            },
                            (glfw::Press, glfw::KeyLeftControl) |
                            (glfw::Release, glfw::KeySpace) => {
                                camera.accelerate(Vec3::new(0.0, -1.0, 0.0));
                            },
                            (glfw::Press, glfw::KeySpace) |
                            (glfw::Release, glfw::KeyLeftControl) => {
                                camera.accelerate(Vec3::new(0.0, 1.0, 0.0));
                            },
                            (glfw::Press, glfw::KeyLeftShift) => camera.fast(true),
                            (glfw::Release, glfw::KeyLeftShift) => camera.fast(false),

                            (glfw::Press, glfw::KeyR) => {
                                renderer.reload_resources();
                            },
                            (glfw::Press, glfw::KeyEscape) => {
                                window.set_should_close(true);
                            },
                            (glfw::Press, glfw::KeyG) => {
                                grabbed = !grabbed;
                                if grabbed {
                                    window.set_cursor_mode(glfw::CursorDisabled);
                                } else {
                                    window.set_cursor_mode(glfw::CursorNormal);
                                }
                            },
                            (glfw::Press, glfw::KeyL) => {
                                renderer.toggle_wireframe_mode();
                            },
                            _ => {},
                        }
                    },
                    _ => {},
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
    });
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
    if err != gl::NO_ERROR {
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
