/* 
 * Copyright (c) 2016 Snowplow Analytics Ltd. All rights reserved.
 *
 * This program is licensed to you under the Apache License Version 2.0, and
 * you may not use this file except in compliance with the Apache License
 * Version 2.0.  You may obtain a copy of the Apache License Version 2.0 at
 * http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the Apache License Version 2.0 is distributed on an "AS
 * IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
 * implied.  See the Apache License Version 2.0 for the specific language
 * governing permissions and limitations there under.
 */

use super::*;
use factotum::factfile::Factfile;
use factotum::tests::make_task;
use std::io::prelude::*;
use std::fs::File;

#[inline]
fn resource(name:&str) -> String {
    format!("./tests/resources/{}", name)
}

fn file_contents(name:&str) -> String {
    let mut file = File::open(name).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();

    contents
}

#[test]
fn generate_graphviz_dot_good() {

    //  Expected graph: 
    //
    //        apple----------             turnip
    //       /     \         \
    //   orange    egg-----   \
    //      \              \   \
    //       \               potato
    //        \                 \
    //         --------------- chicken

    let example = file_contents(&resource("dot/example_apples.dot"));

    let mut ff = Factfile::new("N/A", "Sample job");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    print!("EXPECTED:\n{}\n", example);

    let actual = generate_graphviz_dot(&ff, None);

    print!("ACTUAL:\n{}\n", actual);

    assert_eq!(actual, example);
}

#[test]
fn generate_graphviz_dot_good_poly() {
    let example = file_contents(&resource("dot/example_apples_poly.dot"));

    let mut ff = Factfile::new("N/A", "Sample job #2");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));
    ff.add_task_obj(&make_task("milk",    &vec!["turnip"]));
    ff.add_task_obj(&make_task("cheese",  &vec!["milk"]));
    ff.add_task_obj(&make_task("cake",    &vec!["milk"]));

    print!("EXPECTED:\n{}\n", example);

    let actual = generate_graphviz_dot(&ff, None);

    print!("ACTUAL:\n{}\n", actual);

    assert_eq!(actual, example);
}

#[test]
fn generate_graphviz_dot_good_short() {
    let example = file_contents(&resource("dot/example_reduced.dot"));

    let mut ff = Factfile::new("N/A", "Sample job #3 (reduced run)");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));
    ff.add_task_obj(&make_task("milk",    &vec!["turnip"]));
    ff.add_task_obj(&make_task("cheese",  &vec!["milk"]));
    ff.add_task_obj(&make_task("cake",    &vec!["milk"]));

    print!("EXPECTED:\n{}\n", example);

    let actual = generate_graphviz_dot(&ff, Some("turnip".to_string()));

    print!("ACTUAL:\n{}\n", actual);

    assert_eq!(actual, example);
}