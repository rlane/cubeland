#[feature(globs)];

extern mod extra;
extern mod glfw;
extern mod gl;
extern mod cgmath;

use std::cast;
use std::ptr;
use std::str;
use std::vec;
use std::libc;
use extra::time;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::matrix::Mat3;
use cgmath::matrix::Mat4;
use cgmath::matrix::ToMat4;
use cgmath::vector::Vec4;
use cgmath::angle::rad;
use cgmath::projection;

#[link(name="GLU")]
extern {}

static vertex_shader_src : &'static str = r"
#version 110

uniform mat4 transform;

attribute vec4 position;

varying vec3 color;

void main() {
    gl_Position = transform * position;
    color = (position.xyz + vec3(1.0, 1.0, 1.0)) * 0.5;
}
";

static fragment_shader_src : &'static str = r"
#version 110

varying vec3 color;

void main() {
    gl_FragColor = vec4(color, 1.0);
}
";

#[start]
fn start(argc: int, argv: **u8) -> int {
    std::rt::start_on_main_thread(argc, argv, main)
}

fn main() {
   glfw::set_error_callback(~ErrorContext);

   let window_width = 800;
   let window_height = 600;

    do glfw::start {
        let window = glfw::Window::create(window_width, window_height, "Hello, I am a window.", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        //window.set_cursor_mode(glfw::CursorDisabled);
        window.make_context_current();

        gl::load_with(glfw::get_proc_address);

        let vs = compile_shader(vertex_shader_src.as_bytes(), gl::VERTEX_SHADER);
        let fs = compile_shader(fragment_shader_src.as_bytes(), gl::FRAGMENT_SHADER);
        let program = link_program(vs, fs);

        check_gl("after link");

        let mut uniform_transform;

        unsafe {
            uniform_transform = "transform".with_c_str(|ptr| gl::GetUniformLocation(program, ptr));
        }

        check_gl("after uniform transform location");

        let mut vao = 0;
        let mut vbo = 0;
        let mut ebo = 0;

        let vertices : ~[GLfloat] = ~[
            -1.0, -1.0,  1.0, 1.0,
            1.0, -1.0,  1.0, 1.0,
            -1.0,  1.0,  1.0, 1.0,
            1.0,  1.0,  1.0, 1.0,
            -1.0, -1.0, -1.0, 1.0,
            1.0, -1.0, -1.0, 1.0,
            -1.0,  1.0, -1.0, 1.0,
            1.0,  1.0, -1.0, 1.0,
        ];

        let elements : ~[GLshort] = ~[
            0, 1, 2, 3, 7, 1, 5, 4, 7, 6, 2, 4, 0, 1
        ];

        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::Enable(gl::CULL_FACE);

            glfw::set_swap_interval(1);

            // Create Vertex Array Object
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER,
                           (vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                           cast::transmute(&vertices[0]),
                           gl::STATIC_DRAW);

            check_gl("after vertex buffer");

            // Create a Vertex Buffer Object and copy the element data to it
            gl::GenBuffers(1, &mut ebo);
            gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
            gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                           (elements.len() * std::mem::size_of::<GLshort>()) as GLsizeiptr,
                           cast::transmute(&elements[0]),
                           gl::STATIC_DRAW);

            check_gl("after element buffer");

            // Use shader program
            gl::UseProgram(program);

            // Specify the layout of the vertex data
            let vert_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(program, ptr));
            gl::EnableVertexAttribArray(vert_attr as GLuint);
            gl::VertexAttribPointer(vert_attr as GLuint, 4, gl::FLOAT,
                                    gl::FALSE as GLboolean, 0, ptr::null());

            check_gl("after vertex attrib pointer");
        }

        check_gl("after buffers");

        window.set_cursor_pos_callback(~CursorPosContext);
        window.set_key_callback(~KeyContext);

        let start_time = extra::time::precise_time_ns();
        let mut last_frame_time = start_time;
        let mut num_frames = 0;

        while !window.should_close() {
            glfw::poll_events();

            gl::Viewport(0,0, window_width as GLint, window_height as GLint);

            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let elapsed : u64 = extra::time::precise_time_ns() - start_time;
            let angle : f32 = elapsed as f32 / (1000*1000*1000) as f32;

            let translation = Mat4::<f32>::from_cols(
                Vec4::<f32>::unit_x(),
                Vec4::<f32>::unit_y(),
                Vec4::<f32>::unit_z(),
                Vec4::<f32>::new(0.0f32, 0.0f32, -5.0f32, 1.0f32));
            let rotation_x = Mat3::<f32>::from_angle_x(rad(0.5f32)).to_mat4();
            let rotation_y = Mat3::<f32>::from_angle_y(rad(angle)).to_mat4();
            let projection = cgmath::projection::perspective(rad(1.57 as f32), (4.0/3.0) as f32, 0.1 as f32, 10.0 as f32);
            let transform = projection.mul_m(&translation).mul_m(&rotation_x).mul_m(&rotation_y);

            unsafe {
                gl::UniformMatrix4fv(uniform_transform, 1, gl::FALSE, cast::transmute(&transform));

                check_gl("after uniform transform");

                gl::DrawElements(gl::TRIANGLE_STRIP, elements.len() as i32, gl::UNSIGNED_SHORT, ptr::null());
            }

            window.swap_buffers();

            check_gl("main loop");

            let cur_time = extra::time::precise_time_ns();
            num_frames += 1;
            if (cur_time - last_frame_time) > (1000*1000*1000) {
                println!("{} frames per second", num_frames);
                num_frames = 0;
                last_frame_time = cur_time;
            }
        }
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

fn compile_shader(src: &[u8], ty: GLenum) -> GLuint {
    let shader = gl::CreateShader(ty);
    unsafe {
        // Attempt to compile the shader
        //transmute is used here because `as` causes ICE
        //wait a sec, is `src` null-terminated properly?
        gl::ShaderSource(shader, 1, std::cast::transmute(std::ptr::to_unsafe_ptr(&src.as_ptr())), ptr::null());
        gl::CompileShader(shader);

        // Get the compile status
        let mut status = gl::FALSE as GLint;
        gl::GetShaderiv(shader, gl::COMPILE_STATUS, &mut status);

        // Fail on error
        if status != (gl::TRUE as GLint) {
            let mut len = 0;
            gl::GetShaderiv(shader, gl::INFO_LOG_LENGTH, &mut len);
            let mut buf = vec::from_elem(len as uint - 1, 0u8);     // subtract 1 to skip the trailing null character
            gl::GetShaderInfoLog(shader, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!(str::raw::from_utf8(buf).to_owned());
        }
    }
    shader
}

fn link_program(vs: GLuint, fs: GLuint) -> GLuint {
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
            let mut buf = vec::from_elem(len as uint - 1, 0u8);     // subtract 1 to skip the trailing null character
            gl::GetProgramInfoLog(program, len, ptr::mut_null(), buf.as_mut_ptr() as *mut GLchar);
            fail!(str::raw::from_utf8(buf).to_owned());
        }
    }
    program
}

struct ErrorContext;
impl glfw::ErrorCallback for ErrorContext {
    fn call(&self, _: glfw::Error, description: ~str) {
        fail!("GLFW Error: {:s}", description);
    }
}

struct CursorPosContext;
impl glfw::CursorPosCallback for CursorPosContext {
    fn call(&self, _: &glfw::Window, _: f64, _: f64) {
    }
}

struct KeyContext;
impl glfw::KeyCallback for KeyContext {
    fn call(&self, window: &glfw::Window, key: glfw::Key, _: libc::c_int, action: glfw::Action, _: glfw::Modifiers) {
        match (action, key) {
            (glfw::Press, glfw::KeyEscape) => window.set_should_close(true),
            _ => println!("unexpected key callback for action {:?} key {:?}", action, key)
        }
    }
}
