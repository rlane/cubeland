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

extern mod gl;

use gl::types::*;

pub struct GLTimer {
    before_query : GLuint,
    after_query : GLuint,
}

impl GLTimer {
    pub fn new() -> GLTimer {
        let mut queries : [GLuint, ..2] = [0, 0];
        unsafe { gl::GenQueries(2, &mut queries[0]) };
        GLTimer { before_query: queries[0], after_query: queries[1] }
    }

    pub fn start(&self) {
        gl::QueryCounter(gl::TIMESTAMP, self.before_query);
    }

    pub fn finish(&self) {
        gl::QueryCounter(gl::TIMESTAMP, self.after_query);
    }

    pub fn elapsed(&self) -> GLuint {
        let mut before_time : GLuint = 0;
        let mut after_time : GLuint = 0;
        unsafe {
            gl::GetQueryObjectuiv(self.before_query, gl::QUERY_RESULT, &mut before_time);
            gl::GetQueryObjectuiv(self.after_query, gl::QUERY_RESULT, &mut after_time);
        }
        before_time - after_time
    }
}
