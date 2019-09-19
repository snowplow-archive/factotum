// Copyright (c) 2016-2019 Snowplow Analytics Ltd. All rights reserved.
//
// This program is licensed to you under the Apache License Version 2.0, and
// you may not use this file except in compliance with the Apache License
// Version 2.0.  You may obtain a copy of the Apache License Version 2.0 at
// http://www.apache.org/licenses/LICENSE-2.0.
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the Apache License Version 2.0 is distributed on an "AS
// IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
// implied.  See the Apache License Version 2.0 for the specific language
// governing permissions and limitations there under.
//

extern crate mustache;

#[cfg(test)]
mod tests;

use std::error::Error;
use rustc_serialize::json::Json;

pub fn decorate_str(template: &str, env: &Json) -> Result<String, String> {
    let compiled_template = mustache::compile_str(&template);
    let mut bytes = vec![];
    try!(compiled_template.render(&mut bytes, &env)
        .map_err(|e| format!("Error rendering template: {}", Error::description(&e))));
    String::from_utf8(bytes).map_err(|e| {
        format!("Error inflating rendered template to utf8: {}",
                Error::description(&e))
    })
}
