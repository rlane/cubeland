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

extern mod cgmath;

use cgmath::vector::Vector;
use cgmath::vector::Vec2;

use std::num::{One, Zero};

/// An iterator that yields a spiral pattern
///
/// Example:
/// A-B-C-D-E-F-0
/// |
/// 9 4-5-6-7-8-9
/// | |         |
/// 8 3 6-7-8-9 A
/// | | |     | |
/// 7 2 5 0-1 A B
/// | | |   | | |
/// 6 1 4-3-2 B C
/// | |       | |
/// 5 0-F-E-D-C D
/// |           |
/// 4-3-2-1-0-F-E
pub struct Spiral<A> {
    next : Vec2<A>,
    dir : Vec2<A>,
    pos : uint,
    length : uint,
    segment : uint,
    count : uint,
    end : uint,
}

impl<A: Clone + Zero + One + Primitive> Spiral<A> {
    pub fn new(end: uint) -> Spiral<A> {
        Spiral {
            next: Vec2::new(Zero::zero(), Zero::zero()),
            dir: Vec2::new(One::one(), Zero::zero()),
            pos: 0,
            length: 1,
            segment: 2,
            count: 0,
            end: end,
        }
    }
}

impl<A: Add<A, A> + Neg<A> + One + Zero + Eq + Clone + Primitive> Iterator<Vec2<A>> for Spiral<A> {
    fn next(&mut self) -> Option<Vec2<A>> {
        if self.count == self.end {
            None
        } else {
            let ret = self.next.clone();
            self.count += 1;
            self.next.add_self_v(&self.dir);
            self.pos += 1;
            if self.pos == self.length {
                self.segment -= 1;
                if self.segment == 0 {
                    self.length += 1;
                    self.segment = 2;
                }
                self.pos = 0;
                self.dir = Vec2 { x: self.dir.y.clone(), y: -self.dir.x.clone() };
            }
            Some(ret)
        }
    }
}
#[test]
fn test_spiral() {
    let mut spiral = Spiral::new(10);
    assert_eq!(spiral.next(), Some(Vec2::new(0, 0)));
    assert_eq!(spiral.next(), Some(Vec2::new(1, 0)));
    assert_eq!(spiral.next(), Some(Vec2::new(1, -1)));
    assert_eq!(spiral.next(), Some(Vec2::new(0, -1)));
    assert_eq!(spiral.next(), Some(Vec2::new(-1, -1)));
    assert_eq!(spiral.next(), Some(Vec2::new(-1, 0)));
    assert_eq!(spiral.next(), Some(Vec2::new(-1, 1)));
    assert_eq!(spiral.next(), Some(Vec2::new(0, 1)));
    assert_eq!(spiral.next(), Some(Vec2::new(1, 1)));
    assert_eq!(spiral.next(), Some(Vec2::new(2, 1)));
    assert_eq!(spiral.next(), None);
}
