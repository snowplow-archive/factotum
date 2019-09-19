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

use super::*;

#[test]
fn task_new_defaults_good() {
    let task = Task::<String>::new("hello", "world".to_string());
    assert_eq!(task.name, "hello");
    assert_eq!(task.state, State::Waiting);
    // assert_eq!(task.children.len(), 0);
    assert_eq!(task.task_spec, "world".to_string());
    assert!(task.run_result.is_some() == false)
}

#[test]
fn task_grp_dup_names_err() {
    let mut task_group = TaskGroup::<String>::new();
    task_group.push(Task::<String>::new("hello", "hello".to_string()));
    task_group.push(Task::<String>::new("world", "world".to_string()));
    let mut tl = TaskList::<String>::new();
    let expected_good = tl.add_group(task_group);
    assert!(expected_good.ok().is_some());


    let mut task_group_err = TaskGroup::<String>::new();
    task_group_err.push(Task::<String>::new("hello", "hello".to_string()));
    let expected_bad = tl.add_group(task_group_err);
    match expected_bad {
        Ok(()) => panic!("Duplicate values added to tasklist"),
        Err(msg) => {
            assert_eq!("Task 'hello' has been added already - task names must be unique",
                       msg)
        }
    }
}

#[test]
fn test_get_by_name() {
    let mut tl = TaskList::<String>::new();
    assert!(tl.get_task_by_name("banana").is_some() == false);

    tl.add_group(vec![Task::<String>::new("hello", "world".to_string()),
                        Task::<String>::new("yes", "world".to_string())])
        .ok()
        .unwrap();

    tl.add_group(vec![Task::<String>::new("thing", "world".to_string()),
                        Task::<String>::new("yah", "world".to_string())])
        .ok()
        .unwrap();

    assert!(tl.get_task_by_name("hello").is_some());
    assert!(tl.get_task_by_name("yes").is_some());
    assert!(tl.get_task_by_name("thing").is_some());
    assert!(tl.get_task_by_name("yah").is_some());
}

#[test]
fn set_child_no_parent_err() {
    let mut tl = TaskList::<&str>::new();
    tl.add_group(vec![
                    Task::<&str>::new("child", "world"),
                    ])
        .ok()
        .unwrap();
    let r = tl.set_child("parent", "child");
    assert!(r.ok().is_some() == false);
}

#[test]
fn set_child_no_child_err() {
    let mut tl = TaskList::<&str>::new();
    tl.add_group(vec![
                    Task::<&str>::new("parent", "world"),
                    ])
        .ok()
        .unwrap();
    let r = tl.set_child("parent", "child");
    assert!(r.ok().is_some() == false);
}

#[test]
fn set_child_good() {
    let mut tl = TaskList::<&str>::new();
    let tg = vec![Task::<&str>::new("parent", "world"), Task::<&str>::new("child", "world")];
    tl.add_group(tg).ok().unwrap();
    let r = tl.set_child("parent", "child");
    assert!(r.ok().is_some() == true);
}

#[test]
fn get_children() {
    let mut tl = TaskList::<&str>::new();
    let tg = vec![Task::<&str>::new("parent", "world"),
                  Task::<&str>::new("child", "world"),
                  Task::<&str>::new("grandchild", "world"),
                  Task::<&str>::new("grandchild2", "world")];
    tl.add_group(tg).ok().unwrap();
    tl.set_child("parent", "child").ok();
    tl.set_child("child", "grandchild").ok();
    tl.set_child("child", "grandchild2").ok();

    assert_eq!(vec!["grandchild", "grandchild2"],
               tl.get_descendants("child"));
    assert_eq!(vec!["child", "grandchild", "grandchild2"],
               tl.get_descendants("parent"));
    assert_eq!(Vec::<String>::new(), tl.get_descendants(""))
}

#[test]
fn get_children_dups_removed() {
    let mut tl = TaskList::<&str>::new();
    let tg = vec![Task::<&str>::new("parent", "world"),
                  Task::<&str>::new("child", "world"),
                  Task::<&str>::new("grandchild", "world"),
                  Task::<&str>::new("grandchild2", "world")];
    tl.add_group(tg).ok().unwrap();
    tl.set_child("parent", "child").ok();
    tl.set_child("child", "grandchild").ok();
    tl.set_child("child", "grandchild2").ok();
    tl.set_child("parent", "grandchild2").ok();

    assert_eq!(vec!["grandchild", "grandchild2"],
               tl.get_descendants("child"));
    assert_eq!(vec!["child", "grandchild", "grandchild2"],
               tl.get_descendants("parent"));
    assert_eq!(Vec::<String>::new(), tl.get_descendants(""))
}

#[test]
fn is_task_name_present_good() {
    let mut tl = TaskList::<&str>::new();
    let tg = vec![Task::<&str>::new("parent", "world"),
                  Task::<&str>::new("child", "world"),
                  Task::<&str>::new("grandchild", "world"),
                  Task::<&str>::new("grandchild2", "world")];
    tl.add_group(tg).ok().unwrap();

    assert!(tl.is_task_name_present("parent"));
    assert!(tl.is_task_name_present("grandchild2"));
    assert!(tl.is_task_name_present("banana") == false);
}
