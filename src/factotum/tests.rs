// Copyright (c) 2016 Snowplow Analytics Ltd. All rights reserved.
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

use factotum::factfile::Task;
use factotum::factfile::OnResult;

pub fn compare_tasks(expected: Vec<Vec<&str>>, actual: Vec<Vec<&Task>>) {
    for i in 0..expected.len() {
        for j in 0..expected[i].len() {
            let expected_row = expected.get(i).unwrap();
            let actual_row = actual.get(i).unwrap();
            assert_eq!(expected_row.len(), actual_row.len());
            assert_eq!(expected_row.get(j).unwrap(),
                       &actual_row.get(j).unwrap().name);
        }
    }
}

pub fn make_task(name: &str, depends_on: &Vec<&str>) -> Task {
    Task {
        name: name.to_string(),
        depends_on: depends_on.iter().map(|s| String::from(*s)).collect::<Vec<String>>(),
        executor: "".to_string(),
        command: "".to_string(),
        arguments: vec![],
        on_result: OnResult {
            terminate_job: vec![],
            continue_job: vec![],
        },
    }
}
