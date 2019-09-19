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

use factotum::parser::templater::*;
use rustc_serialize::json::Json;

fn from_json(json: &str) -> Json {
    Json::from_str(json).unwrap()
}

#[test]
fn decorated_string_works() {
    assert_eq!("hello Ed!".to_string(),
               decorate_str("hello {{name}}!", &from_json("{\"name\":\"Ed\"}")).unwrap());
}

#[test]
fn decorated_nested_string_works() {
    assert_eq!("hello Ted!".to_string(),
               decorate_str("hello {{person.name}}!",
                            &from_json("{\"person\": { \"name\":\"Ted\" } }"))
                   .unwrap())
}
