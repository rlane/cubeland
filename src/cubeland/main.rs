#[feature(globs)];

extern mod glfw;
extern mod gl;

use std::cast;
use std::ptr;
use std::str;
use std::vec;
use std::libc;

use gl::types::*;

static vertex_shader_src : &'static str = r"
#version 110

attribute vec4 position;

varying vec2 texcoord;

void main() {
    gl_Position = position;
    texcoord = position.xy;
}
";

static fragment_shader_src : &'static str = r"
#version 110

varying vec2 texcoord;

void main() {
    gl_FragColor = vec4(texcoord, 1.0, 1.0);
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

        let mut vao = 0;
        let mut vbo = 0;

        let VERTEX_DATA : ~[GLfloat] = ~[
            -1.0, -1.0, 0.0, 1.0,
            1.0, -1.0, 0.0, 1.0,
            -1.0, 1.0, 0.0, 1.0,
            1.0, 1.0, 0.0, 1.0,
        ];

        unsafe {
            // Create Vertex Array Object
            gl::GenVertexArrays(1, &mut vao);
            gl::BindVertexArray(vao);

            // Create a Vertex Buffer Object and copy the vertex data to it
            gl::GenBuffers(1, &mut vbo);
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
            gl::BufferData(gl::ARRAY_BUFFER,
                           (VERTEX_DATA.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                           cast::transmute(&VERTEX_DATA[0]),
                           gl::STATIC_DRAW);

            // Use shader program
            gl::UseProgram(program);

            // Specify the layout of the vertex data
            let vert_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(program, ptr));
            gl::EnableVertexAttribArray(vert_attr as GLuint);
            gl::VertexAttribPointer(vert_attr as GLuint, 4, gl::FLOAT,
                                    gl::FALSE as GLboolean, 0, ptr::null());
        }

        check_gl();

        window.set_cursor_pos_callback(~CursorPosContext);
        window.set_key_callback(~KeyContext);

        while !window.should_close() {
            glfw::poll_events();

            gl::Viewport(0,0, window_width as GLint, window_height as GLint);

            gl::ClearColor(0.8, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);
            gl::DrawArrays(gl::TRIANGLE_STRIP, 0, 4);

            window.swap_buffers();

            check_gl();
        }
    }
}

fn check_gl() {
    let err = gl::GetError();
    if (err != gl::NO_ERROR) {
        fail!("GL error");
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
