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

#[macro_escape];

macro_rules! offset_of(
    ($typename:ty, $fieldname:ident) => (
        {
            let ptr = std::cast::transmute::<*$typename, &$typename>(std::ptr::null::<$typename>());
            let offset : uint = std::cast::transmute(&ptr.$fieldname);
            offset
        }
    );
)

#[test]
fn test_offset_of() {
    struct Foo {
        a: i32,
        b: u8,
        c: u16,
    };

    assert_eq!(offset_of!(Foo, a), 0);
    assert_eq!(offset_of!(Foo, b), 4);
    assert_eq!(offset_of!(Foo, c), 6);
}
