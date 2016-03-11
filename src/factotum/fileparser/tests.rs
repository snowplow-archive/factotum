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
 
use factotum::fileparser::*;

#[inline]
fn resource(name:&str) -> String {
    format!("./tests/resources/{}", name)
}

#[test]
fn invalid_files_err() {
    let res = parse("asdhf;asdjhfasdf");
    if let Err(msg) = res {
        assert_eq!(msg, "Couldn't open 'asdhf;asdjhfasdf' for reading: No such file or directory (os error 2)".to_string())
    } else {
        panic!("the file doesn't exist - the test should have failed");
    }
}

#[test]
fn invalid_json_err() {
    let res = parse(&resource("invalid_json.factotum"));
    if let Err(msg) = res {
        assert_eq!(msg,format!("The factfile '{}' is not valid JSON: invalid syntax at line 1, column 3", resource("invalid_json.factotum")).to_string())
    } else {
        panic!("the file is invalid json - the test should have failed");
    }
}

#[test]
fn invalid_against_schema_err() {
    let invalid = resource("example_invalid_no_name.factotum");
    let res = parse(&invalid);
    if let Err(msg) = res {
        assert_eq!(msg,format!("'{}' is not a valid factotum factfile: '/data/name' - This property is required", invalid).to_string())
    } else {
        panic!("the file is invalid json - the test should have failed");
    }
}

#[test]
fn invalid_against_schema_wrong_type() {
    let invalid = resource("example_wrong_type.factotum");
    let res = parse(&invalid);
    if let Err(msg) = res {
        assert_eq!(msg,format!("'{}' is not a valid factotum factfile: '/data/tasks/0/onResult/terminateJobWithSuccess/0' - Type of the value is wrong (The value must be integer)", invalid).to_string())
    } else {
        panic!("the file is invalid json - the test should have failed");
    }
}

#[test]
fn invalid_ambiguous_on_result() {
    let invalid = resource("example_invalid_terminate_continue_same.factotum");
    let res = parse(&invalid);
    if let Err(msg) = res {
        assert_eq!(msg, format!("'{}' is not a valid factotum factfile: the task 'ambi' has conflicting actions.", invalid))
    } else {
        panic!("conflicting actions in onResult should fail");
    }
}

#[test]
fn invalid_must_continue() {
    let invalid = resource("example_invalid_no_continue.factotum");
    let res = parse(&invalid);
    if let Err(msg) = res {
        assert_eq!(msg, format!("'{}' is not a valid factotum factfile: the task 'continue' has no way to continue successfully.", invalid))
    } else {
        panic!("having no values in continue should fail");
    }
}

#[test]
fn valid_generates_factfile() {
    let valid = resource("example_ok.factotum");

    if let Ok(factfile) = parse(&valid) {
        let tasks = factfile.get_tasks_in_order();
        assert_eq!(factfile.name, "My First DAG");

        let task_one = tasks.get(0).unwrap().get(0).unwrap();
        assert_eq!(task_one.name, "EmrEtlRunner");
        assert_eq!(task_one.depends_on, Vec::<&str>::new());

        let task_two = tasks.get(1).unwrap().get(0).unwrap();
        assert_eq!(task_two.name, "StorageLoader");
        assert_eq!(task_two.depends_on, vec!["EmrEtlRunner"]);

        let task_three = tasks.get(2).unwrap().get(0).unwrap();
        assert_eq!(task_three.name, "SQL Runner");
        assert_eq!(task_three.depends_on, vec!["StorageLoader"]);
    } else {
        panic!("valid factfile example_ok.factotum should have parsed but didn't");
    }

}
