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
