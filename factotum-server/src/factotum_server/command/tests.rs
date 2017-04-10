// Copyright (c) 2016-2017 Snowplow Analytics Ltd. All rights reserved.
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

use super::*;

#[test]
fn create_command_store_macro() {
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];
    assert_eq!(command_store.command_map.contains_key("dummy"), true);
    assert_eq!(command_store.command_map.contains_key("other_dummy"), false);
}

#[test]
fn command_store_get_command_success() {
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];
    assert_eq!(command_store.get_command("dummy"), Ok("/tmp/fake_command".to_string()));
}

#[test]
fn command_store_get_command_error() {
    let command_store = CommandStore::new(HashMap::new());
    assert_eq!(command_store.get_command("dummy"), Err("Command <dummy> not found in map.".to_string()));
}

#[test]
fn command_store_execute_fail() {
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];
    let output = command_store.execute("/tmp/fake_command".to_string(), vec!["--random_arg".to_string()]).unwrap_err();
    assert_eq!(output, "Failed to execute command: [/tmp/fake_command --random_arg] - No such file or directory (os error 2)");
}

#[test]
fn command_store_execute_illegal_option() {
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];
    let output = command_store.execute("pwd".to_string(), vec!["--random_arg".to_string()]).unwrap_err();
    assert_eq!(output, "Failed to execute command: [pwd --random_arg] - pwd: unrecognized option \'--random_arg\'\nTry \'pwd --help\' for more information.\n");
}
