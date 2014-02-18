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

extern crate gl;

use std;

use gl::types::*;

pub struct FBO {
    color_tex: GLuint,
    depth_tex: GLuint,
    fbo: GLuint,
}

impl FBO {
    pub fn new(size: GLint) -> FBO {
        //RGBA8 2D texture
        let mut color_tex = 0;
        unsafe { gl::GenTextures(1, &mut color_tex); }
        gl::BindTexture(gl::TEXTURE_2D, color_tex);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        unsafe { gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGBA8 as GLint, size, size, 0, gl::BGRA, gl::UNSIGNED_BYTE, std::ptr::null()); }

        // 24 bit depth texture
        let mut depth_tex = 0;
        unsafe { gl::GenTextures(1, &mut depth_tex); }
        gl::BindTexture(gl::TEXTURE_2D, depth_tex);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::REPEAT as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::DEPTH_TEXTURE_MODE, gl::INTENSITY as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_MODE, gl::COMPARE_R_TO_TEXTURE as GLint);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_COMPARE_FUNC, gl::LEQUAL as GLint);
        unsafe { gl::TexImage2D(gl::TEXTURE_2D, 0, gl::DEPTH_COMPONENT24 as GLint, size, size, 0, gl::DEPTH_COMPONENT, gl::UNSIGNED_BYTE, std::ptr::null()); }

        let mut fbo = 0;
        unsafe { gl::GenFramebuffers(1, &mut fbo); }
        gl::BindFramebuffer(gl::FRAMEBUFFER, fbo);

        //Attach 2D texture to this FBO
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::COLOR_ATTACHMENT0, gl::TEXTURE_2D, color_tex, 0/*mipmap level*/);
        //Attach depth texture to FBO
        gl::FramebufferTexture2D(gl::FRAMEBUFFER, gl::DEPTH_ATTACHMENT, gl::TEXTURE_2D, depth_tex, 0/*mipmap level*/);

        //Does the GPU support current FBO configuration?
        match gl::CheckFramebufferStatus(gl::FRAMEBUFFER) {
            gl::FRAMEBUFFER_COMPLETE => {
                println!("Framebuffer complete");
            },
            _ => {
                println!("Framebuffer error");
            }
        }

        gl::BindTexture(gl::TEXTURE_2D, 0);
        gl::BindFramebuffer(gl::FRAMEBUFFER, 0);

        FBO {
            color_tex: color_tex,
            depth_tex: depth_tex,
            fbo: fbo,
        }
    }
}

impl Drop for FBO {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteTextures(1, &self.color_tex);
            gl::DeleteTextures(1, &self.depth_tex);
            gl::DeleteFramebuffers(1, &self.fbo);
        }
    }
}
