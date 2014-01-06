#[feature(globs)];

extern mod extra;
extern mod glfw;
extern mod gl;
extern mod cgmath;
extern mod noise;

use std::cast;
use std::ptr;
use std::str;
use std::vec;
use std::libc;

use gl::types::*;

use cgmath::matrix::Matrix;
use cgmath::matrix::Mat3;
use cgmath::matrix::Mat4;
use cgmath::matrix::ToMat4;
use cgmath::vector::Vector;
use cgmath::vector::Vec3;
use cgmath::vector::Vec4;
use cgmath::angle::rad;

use noise::Perlin;

#[link(name="GLU")]
extern {}

static vertex_shader_src : &'static str = r"
#version 110

uniform mat4 transform;

attribute vec4 position;
attribute vec3 normal;

varying vec4 frag_position;
varying vec3 frag_normal;

void main() {
    frag_position = position;
    frag_normal = normal;
    gl_Position = transform * frag_position;
}
";

static fragment_shader_src : &'static str = r"
#version 110

varying vec4 frag_position;
varying vec3 frag_normal;

const vec4 obj_diffuse = vec4(0.2, 0.6, 0.2, 1.0);

const vec3 light_direction = vec3(0.408248, -0.816497, 0.408248);
const vec4 light_diffuse = vec4(0.8, 0.8, 0.8, 0.0);
const vec4 light_ambient = vec4(0.2, 0.2, 0.2, 1.0);

void main() {
    vec3 mv_light_direction = light_direction;

    vec4 diffuse_factor
        = max(-dot(frag_normal, mv_light_direction), 0.0) * light_diffuse;
    vec4 ambient_diffuse_factor = diffuse_factor + light_ambient;

    gl_FragColor = ambient_diffuse_factor * obj_diffuse;
}
";

static CHUNK_SIZE: uint = 16;

struct Chunk {
    x: i64,
    z: i64,
    blocks: [[[Block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE],
}

struct Block {
    color: Vec4<f32>,
}

struct GraphicsResources {
    program: GLuint,
    vao: GLuint,
    uniform_transform: GLint,
}

#[start]
fn start(argc: int, argv: **u8) -> int {
    std::rt::start_on_main_thread(argc, argv, main)
}

fn main() {
   glfw::set_error_callback(~ErrorContext);

    do glfw::start {
        let mut window_width = 800;
        let mut window_height = 600;

        let window = glfw::Window::create(window_width, window_height, "Hello, I am a window.", glfw::Windowed)
            .expect("Failed to create GLFW window.");

        window.set_cursor_mode(glfw::CursorDisabled);
        window.make_context_current();

        gl::load_with(glfw::get_proc_address);

        gl::Enable(gl::DEPTH_TEST);
        gl::Enable(gl::CULL_FACE);

        glfw::set_swap_interval(1);

        let graphics_resources = load_graphics_resources();

        gl::UseProgram(graphics_resources.program);
        gl::BindVertexArray(graphics_resources.vao);

        let chunks = ~[
            chunk_gen(42, 0, 0),
            chunk_gen(42, -16, 0),
            chunk_gen(42, 0, -16),
            chunk_gen(42, -16, -16),
        ];

        window.set_key_callback(~KeyContext);

        let (fb_size_port, fb_size_chan): (Port<(u32,u32)>, Chan<(u32,u32)>) = std::comm::Chan::new();
        window.set_framebuffer_size_callback(~FramebufferSizeContext { chan: fb_size_chan });

        let start_time = extra::time::precise_time_ns();
        let mut last_frame_time = start_time;
        let mut num_frames = 0;

        let mut camera_position = Vec3::<f32>::new(0.0f32, 20.0f32, 40.0f32);

        while !window.should_close() {
            glfw::poll_events();

            loop {
                match fb_size_port.try_recv() {
                    Some((w,h)) => {
                        window_width = w;
                        window_height = h;
                    }
                    None => break
                }
            }

            let (cursor_x, cursor_y) = window.get_cursor_pos();
            let camera_angle_y = ((cursor_x * 0.0005) % 1.0) * std::f64::consts::PI * 2.0;
            let camera_angle_x = ((cursor_y * 0.0005) % 1.0) * std::f64::consts::PI * 2.0;

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

            gl::Viewport(0,0, window_width as GLint, window_height as GLint);

            gl::ClearColor(0.0, 0.0, 0.0, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

            let projection = cgmath::projection::perspective(
                rad(1.57f32),
                window_width as f32 / window_height as f32,
                0.1f32, 100.0f32);

            let camera_translation = Mat4::<f32>::from_cols(
                Vec4::<f32>::unit_x(),
                Vec4::<f32>::unit_y(),
                Vec4::<f32>::unit_z(),
                camera_position.mul_s(-1.0f32).extend(1.0f32));
            let camera_rotation_x = Mat3::<f32>::from_angle_x(rad(camera_angle_x as f32)).to_mat4();
            let camera_rotation_y = Mat3::<f32>::from_angle_y(rad(camera_angle_y as f32)).to_mat4();
            let camera = camera_rotation_x.mul_m(&camera_rotation_y).mul_m(&camera_translation);

            let inv_camera_rotation = Mat3::<f32>::from_euler(rad(-camera_angle_x as f32), rad(-camera_angle_y as f32), rad(0.0f32));
            let absolute_camera_velocity = inv_camera_rotation.mul_v(&camera_velocity);
            camera_position.add_self_v(&absolute_camera_velocity);

            for chunk in chunks.iter() {
                let chunk_transform = Mat4::<f32>::from_cols(
                    Vec4::<f32>::unit_x(),
                    Vec4::<f32>::unit_y(),
                    Vec4::<f32>::unit_z(),
                    Vec4::<f32>::new(chunk.x as f32 * 1.1f32, 0.0f32, chunk.z as f32 * 1.1f32, 1.0f32));

                for x in range(0, CHUNK_SIZE) {
                    for y in range(0, CHUNK_SIZE) {
                        for z in range(0, CHUNK_SIZE) {
                            let block = &chunk.blocks[x][y][z];

                            if (block.color.w < 0.5f32) {
                                continue;
                            }

                            let block_transform = Mat4::<f32>::from_cols(
                                Vec4::<f32>::unit_x(),
                                Vec4::<f32>::unit_y(),
                                Vec4::<f32>::unit_z(),
                                Vec4::<f32>::new(x as f32, y as f32, z as f32, 1.0f32));

                            let transform = projection.mul_m(&camera).mul_m(&chunk_transform).mul_m(&block_transform);

                            unsafe {
                                gl::UniformMatrix4fv(graphics_resources.uniform_transform, 1, gl::FALSE, cast::transmute(&transform));
                                gl::DrawElements(gl::TRIANGLES, cube_elements.len() as i32, gl::UNSIGNED_SHORT, ptr::null());
                            }
                        }
                    }
                }
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

fn chunk_gen(seed: u32, chunk_x: i64, chunk_z: i64) -> Chunk {
    let def_block = Block { color: Vec4::<f32>::new(0.0, 0.0, 0.0, 0.0) };
    let mut blocks: [[[Block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE] = [[[def_block, ..CHUNK_SIZE], ..CHUNK_SIZE], ..CHUNK_SIZE];

    let perlin = Perlin::from_seed([seed as uint]);

    for block_x in range(0, CHUNK_SIZE) {
        for block_z in range(0, CHUNK_SIZE) {
            let noise = perlin.gen([
                (chunk_x + block_x as i64) as f64 * 0.1,
                (chunk_z + block_z as i64) as f64 * 0.1
            ]);
            let height = ((noise + 1.0) * (CHUNK_SIZE as f64 / 2.0)) as uint;
            for y in range(0, height) {
                let color = Vec4::<f32>::new(0.2, 0.8, 0.2, 1.0);
                blocks[block_x][y][block_z] = Block { color: color };
            }
        }
    }

    return Chunk { x: chunk_x, z: chunk_z, blocks: blocks };
}

static cube_vertices : [GLfloat, ..96] = [
    /* Front face */
    -1.0, -1.0,  1.0, 1.0, /* bottom left */
    1.0, -1.0,  1.0, 1.0,  /* bottom right */
    -1.0,  1.0,  1.0, 1.0, /* top left */
    1.0,  1.0,  1.0, 1.0,  /* top right */

    /* Back face */
    1.0, -1.0, -1.0, 1.0, /* bottom right */
    -1.0, -1.0, -1.0, 1.0,  /* bottom left */
    1.0, 1.0, -1.0, 1.0, /* top right */
    -1.0, 1.0, -1.0, 1.0,  /* top left */

    /* Right face */
    1.0, -1.0, 1.0, 1.0, /* bottom front */
    1.0, -1.0, -1.0, 1.0, /* bottom back */
    1.0, 1.0, 1.0, 1.0, /* top front */
    1.0, 1.0, -1.0, 1.0, /* top back */

    /* Left face */
    -1.0, -1.0, -1.0, 1.0, /* bottom back */
    -1.0, -1.0, 1.0, 1.0, /* bottom front */
    -1.0, 1.0, -1.0, 1.0, /* top back */
    -1.0, 1.0, 1.0, 1.0, /* top front */

    /* Top face */
    -1.0, 1.0, 1.0, 1.0, /* front left */
    1.0, 1.0, 1.0, 1.0, /* front right */
    -1.0, 1.0, -1.0, 1.0, /* back left */
    1.0, 1.0, -1.0, 1.0, /* back right */

    /* Bottom face */
    -1.0, -1.0, -1.0, 1.0, /* back left */
    1.0, -1.0, -1.0, 1.0, /* back right */
    -1.0, -1.0, 1.0, 1.0, /* front left */
    1.0, -1.0, 1.0, 1.0, /* front right */
];

static cube_normals : [GLfloat, ..72] = [
    /* Front face */
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,
    0.0, 0.0, 1.0,

    /* Back face */
    0.0, 0.0, -1.0,
    0.0, 0.0, -1.0,
    0.0, 0.0, -1.0,
    0.0, 0.0, -1.0,

    /* Right face */
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,
    1.0, 0.0, 0.0,

    /* Left face */
    -1.0, 0.0, 0.0,
    -1.0, 0.0, 0.0,
    -1.0, 0.0, 0.0,
    -1.0, 0.0, 0.0,

    /* Top face */
    0.0, 1.0, 0.0,
    0.0, 1.0, 0.0,
    0.0, 1.0, 0.0,
    0.0, 1.0, 0.0,

    /* Bottom face */
    0.0, -1.0, 0.0,
    0.0, -1.0, 0.0,
    0.0, -1.0, 0.0,
    0.0, -1.0, 0.0,
];

static cube_elements : [GLshort, ..36] = [
    0, 1, 2, 3, 2, 1,
    4, 5, 6, 7, 6, 5,
    8, 9, 10, 11, 10, 9,
    12, 13, 14, 15, 14, 13,
    16, 17, 18, 19, 18, 17,
    20, 21, 22, 23, 22, 21,
];

fn load_graphics_resources() -> GraphicsResources {
    let vs = compile_shader(vertex_shader_src.as_bytes(), gl::VERTEX_SHADER);
    let fs = compile_shader(fragment_shader_src.as_bytes(), gl::FRAGMENT_SHADER);
    let program = link_program(vs, fs);

    let uniform_transform = unsafe { "transform".with_c_str(|ptr| gl::GetUniformLocation(program, ptr)) };

    let mut vao = 0;
    let mut vertex_buffer = 0;
    let mut normal_buffer = 0;
    let mut element_buffer = 0;

    unsafe {
        // Create Vertex Array Object
        gl::GenVertexArrays(1, &mut vao);
        gl::BindVertexArray(vao);

        // Create a Vertex Buffer Object and copy the vertex data to it
        gl::GenBuffers(1, &mut vertex_buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (cube_vertices.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                        cast::transmute(&cube_vertices[0]),
                        gl::STATIC_DRAW);

        // Create a Vertex Buffer Object and copy the normal data to it
        gl::GenBuffers(1, &mut normal_buffer);
        gl::BindBuffer(gl::ARRAY_BUFFER, normal_buffer);
        gl::BufferData(gl::ARRAY_BUFFER,
                        (cube_normals.len() * std::mem::size_of::<GLfloat>()) as GLsizeiptr,
                        cast::transmute(&cube_normals[0]),
                        gl::STATIC_DRAW);

        // Create a Vertex Buffer Object and copy the element data to it
        gl::GenBuffers(1, &mut element_buffer);
        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, element_buffer);
        gl::BufferData(gl::ELEMENT_ARRAY_BUFFER,
                        (cube_elements.len() * std::mem::size_of::<GLshort>()) as GLsizeiptr,
                        cast::transmute(&cube_elements[0]),
                        gl::STATIC_DRAW);

        gl::UseProgram(program);

        // Specify the layout of the vertex data
        let vert_attr = "position".with_c_str(|ptr| gl::GetAttribLocation(program, ptr));
        assert!(vert_attr as u32 != gl::INVALID_VALUE);
        gl::BindBuffer(gl::ARRAY_BUFFER, vertex_buffer);
        gl::EnableVertexAttribArray(vert_attr as GLuint);
        gl::VertexAttribPointer(vert_attr as GLuint, 4, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());

        let normal_attr = "normal".with_c_str(|ptr| gl::GetAttribLocation(program, ptr));
        assert!(normal_attr as u32 != gl::INVALID_VALUE);
        gl::BindBuffer(gl::ARRAY_BUFFER, normal_buffer);
        gl::EnableVertexAttribArray(normal_attr as GLuint);
        gl::VertexAttribPointer(normal_attr as GLuint, 3, gl::FLOAT,
                                gl::FALSE as GLboolean, 0, ptr::null());
    }

    gl::UseProgram(0);

    gl::BindVertexArray(0);

    return GraphicsResources {
        program: program,
        vao: vao,
        uniform_transform: uniform_transform,
    };

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

struct KeyContext;
impl glfw::KeyCallback for KeyContext {
    fn call(&self, window: &glfw::Window, key: glfw::Key, _: libc::c_int, action: glfw::Action, _: glfw::Modifiers) {
        match (action, key) {
            (glfw::Press, glfw::KeyEscape) => window.set_should_close(true),
            _ => {}
        }
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
