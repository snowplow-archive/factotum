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

use factotum::tests::make_task;
use factotum::factfile::*;
use factotum::executor::*;

#[test]
fn get_task_execution_list_good() {
    let mut ff = Factfile::new("N/A", "test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    //        apple----------             turnip
    //       /     \         \
    //   orange    egg-----   \
    //      \              \   \
    //       \               potato
    //        \                 \
    //         --------------- chicken

    let tl = get_task_execution_list(&ff, None);

    let expected = vec![ vec!["turnip", "apple"],
                         vec!["egg", "orange"],
                         vec!["potato"],
                         vec!["chicken"] ];

    let actual:Vec<Vec<String>> = tl.tasks.iter()
                                          .map(|task_group| task_group.iter().map(|task| task.name.clone()).collect())
                                          .collect();

    assert_eq!(actual, expected);

    // check the children are correctly mapped

    assert_eq!(tl.get_descendants("turnip"), Vec::<String>::new());
    assert_eq!(tl.get_descendants("apple"),  vec!["chicken", "egg", "orange", "potato"]);
    assert_eq!(tl.get_descendants("egg"),    vec!["chicken", "potato"]);
    assert_eq!(tl.get_descendants("orange"), vec!["chicken"]);
}

#[test]
fn get_task_execution_list_good_reduced() {
    let mut ff = Factfile::new("N/A","test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    let tl = get_task_execution_list(&ff, Some("potato".to_string()));
    assert!(tl.tasks[0].len()==1);
    assert_eq!(tl.get_descendants("potato"), vec!["chicken"]);
}

#[test]
fn get_formatted_args() {
    let args_list = format_args("echo", &vec!["hello".to_string(), 
                                              "world".to_string(), 
                                              "abc abc".to_string()]);
    assert_eq!(args_list, "echo \"hello\" \"world\" \"abc abc\"");
}

#[test]
fn get_task_snapshot_clones() {

    use factotum::executor::task_list::*;
    use factotum::executor::execution_strategy::*;
    use chrono::UTC;
    use chrono::duration::Duration;

    let mut ff = Factfile::new("N/A","test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec!["apple"]));

    let mut tl = get_task_execution_list(&ff, None);

    println!("{:?}", tl);

    tl.tasks[0][0].state = State::Success;
    tl.tasks[0][0].run_result = Some(RunResult {
                                                 return_code: 0,
                                                 stderr: Some("hello world".to_string()),
                                                 stdout: Some("hello world".to_string()),
                                                 duration: Duration::seconds(0).to_std().ok().unwrap(),
                                                 run_started: UTC::now(),
                                                 task_execution_error: None
                                               } );

    let snapshot = get_task_snapshot(&tl);

    assert_eq!(snapshot.len(), 2);

    assert_eq!(snapshot[0].state, State::Success);
    assert_eq!(snapshot[0].name, "apple");
    assert_eq!(snapshot[0].run_result, tl.tasks[0][0].run_result);
    assert_eq!(&snapshot[0].task_spec, tl.tasks[0][0].task_spec);

    assert_eq!(snapshot[1].state, State::Waiting);
    assert_eq!(snapshot[1].name, "turnip");
    assert_eq!(snapshot[1].run_result, None);
    assert_eq!(&snapshot[1].task_spec, tl.tasks[1][0].task_spec);

}

#[test]
fn execute_sends_started_msg() {
    use factotum::executor::task_list::State;
    use std::sync::mpsc;
    use std::time::Duration;

    let mut ff = Factfile::new("N/A","test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    let task_count_in_factfile = 6;

    let (tx,rx) = mpsc::channel::<ExecutionState>();

    execute_factfile(&ff, None, execution_strategy::execute_simulation, Some(tx.clone()));

    let expected_starting = rx.recv_timeout(Duration::from_millis(300)).unwrap();

    match expected_starting {
        ExecutionState::Started(ts) => {
            assert!(ts.iter().all(|t| t.state == State::Waiting));
            assert_eq!(ts.len(), task_count_in_factfile);
        },
        _ => panic!("Failed! Didn't receive the correct event type")
    }
}

#[test]
fn execute_sends_running_messages() {
    use factotum::executor::task_list::State;
    use std::sync::mpsc;
    use std::time::Duration;
    use factotum::factfile::{Task as FactfileTask};

    let mut ff = Factfile::new("N/A","test");

    let tasks:Vec<FactfileTask> = vec![
                            make_task("apple",   &vec![]),
                            make_task("turnip",  &vec![]),
                            make_task("orange",  &vec!["apple"]),
                            make_task("egg",     &vec!["apple"]),
                            make_task("potato",  &vec!["apple", "egg"]),
                            make_task("chicken", &vec!["potato","orange"])
                        ];

    for mut task in tasks.into_iter() {
        task.on_result.continue_job.push(0);
        ff.add_task_obj(&task);
    }

    let task_count_in_factfile = 6;

    let (tx,rx) = mpsc::channel::<ExecutionState>();

    let expected_running_message_count = get_task_execution_list(&ff, None).tasks.len();
    let expected_completed_message_count = task_count_in_factfile;
    let count_starting_msg = 1;
    let count_done_msg = 1;

    let total_expected_task_updates = count_starting_msg + count_done_msg + expected_running_message_count + expected_completed_message_count;    

    execute_factfile(&ff, None, execution_strategy::execute_simulation, Some(tx.clone()));

    println!("Expecting {} messages..", total_expected_task_updates);

    for i in 1..total_expected_task_updates {
        let new_msg = rx.recv_timeout(Duration::from_millis(300)).unwrap();

        println!("Received message: ");
        print!("***\n{:?}\n***\n", new_msg);

        if i == 1 {
            assert!(match new_msg { 
                        ExecutionState::Started(tasks) => {
                            assert!(tasks.iter().all(|t| t.state == State::Waiting));
                            true
                        },
                        _ => false })
        } else if i == total_expected_task_updates {
            assert!(match new_msg { 
                        ExecutionState::Finished(tasks) => {
                            assert!(tasks.iter().all(|t| t.state == State::Success));
                            true
                         },
                        _ => false })
        } else if i > 0 && i < total_expected_task_updates {
            assert!(match new_msg { 
                        ExecutionState::Running(tasks) => {
                            assert!(tasks.iter().all(|t| t.state == State::Success || t.state == State::Waiting || t.state == State::Running));
                            true
                        },
                        _ => false })
        } else if i > total_expected_task_updates {
            panic!("Too many messages received")
        } else {
            unreachable!("Uncaught message");
        }
    } 
}

#[test]
fn execute_sends_failed_skipped_messages() {
    use factotum::executor::task_list::State;
    use std::sync::mpsc;
    use std::time::Duration;
    use factotum::factfile::{Task as FactfileTask};

    let mut ff = Factfile::new("N/A","test");

    let tasks:Vec<FactfileTask> = vec![
                            make_task("apple",   &vec![]),
                            make_task("turnip",  &vec!["apple"])
                        ];

    for mut task in tasks.into_iter() {
        task.on_result.continue_job.push(1);
        ff.add_task_obj(&task);
    }

    let (tx,rx) = mpsc::channel::<ExecutionState>();

    execute_factfile(&ff, None, execution_strategy::execute_simulation, Some(tx.clone()));

    for i in 1..5 {
        let new_msg = rx.recv_timeout(Duration::from_millis(300)).unwrap();

        println!("Received message: ");
        print!("***\n{}: {:?}\n***\n", i, new_msg);

        if i == 1 {
            assert!(match new_msg { 
                        ExecutionState::Started(tasks) => {
                            assert!(tasks.iter().all(|t| t.state == State::Waiting));
                            true
                        },
                        _ => false })
        } else if i == 4 {
            assert!(match new_msg { 
                        ExecutionState::Finished(tasks) => {
                            assert!(match tasks.iter().find(|t| t.name == "apple").unwrap().state { State::Failed(_) => true, _ => false } );
                            assert!(match tasks.iter().find(|t| t.name == "turnip").unwrap().state { State::Skipped(_) => true, _ => false } );
                            true
                         },
                        _ => false })
        } else if i == 3 { // it should send a message saying apple failed
            assert!(match new_msg { 
                        ExecutionState::Running(tasks) => {
                            assert!(match tasks.iter().find(|t| t.name == "apple").unwrap().state { State::Failed(_) => true, _ => false  });
                            assert!(match tasks.iter().find(|t| t.name == "turnip").unwrap().state { State::Skipped(_) => true, _ => false });
                            true
                        },
                        _ => false })
        } else if i == 2 { // it should start running task "apple" here which will fail
            assert!(match new_msg { 
                        ExecutionState::Running(tasks) => {
                            assert!(tasks.iter().find(|t| t.name == "apple").unwrap().state == State::Running);
                            true
                        },
                        _ => false })
        } else {
            unreachable!("Uncaught message");
        }
    } 
}

#[test]
fn execute_sends_noop_skipped_messages() {
    use factotum::executor::task_list::State;
    use std::sync::mpsc;
    use std::time::Duration;
    use factotum::factfile::{Task as FactfileTask};

    let mut ff = Factfile::new("N/A","test");

    let tasks:Vec<FactfileTask> = vec![
                            make_task("apple",   &vec![]),
                            make_task("turnip",  &vec!["apple"]),
                            make_task("egg",  &vec!["turnip"])
                        ];

    for mut task in tasks.into_iter() {
        task.on_result.terminate_job.push(0);
        ff.add_task_obj(&task);
    }

    let (tx,rx) = mpsc::channel::<ExecutionState>();

    execute_factfile(&ff, None, execution_strategy::execute_simulation, Some(tx.clone()));

    for i in 1..5 {
        let new_msg = rx.recv_timeout(Duration::from_millis(300)).unwrap();

        println!("Received message: ");
        print!("***\n{:?}\n***\n", new_msg);

        if i == 1 {
            assert!(match new_msg { 
                        ExecutionState::Started(tasks) => {
                            assert!(tasks.iter().all(|t| t.state == State::Waiting));
                            true
                        },
                        _ => false })
        } else if i == 4 {
            assert!(match new_msg { 
                        ExecutionState::Finished(tasks) => {
                            assert!(match tasks.iter().find(|t| t.name == "apple").unwrap().state { State::SuccessNoop => true, _ => false } );
                            assert!(match tasks.iter().find(|t| t.name == "turnip").unwrap().state { State::Skipped(_) => true, _ => false } );
                            assert!(match tasks.iter().find(|t| t.name == "egg").unwrap().state { State::Skipped(_) => true, _ => false } );
                            true
                         },
                        _ => false })
        } else if i == 3 { // task apple should noop
            assert!(match new_msg { 
                        ExecutionState::Running(tasks) => {
                            assert!(tasks.iter().find(|t| t.name == "apple").unwrap().state == State::SuccessNoop);
                            assert!(match tasks.iter().find(|t| t.name == "turnip").unwrap().state { State::Skipped(_) => true, _ => false });
                            assert!(match tasks.iter().find(|t| t.name == "egg").unwrap().state { State::Skipped(_) => true, _ => false });
                            true
                        },
                        _ => false })
        } else if i == 2 { // it should start running task "apple" here which will cause a noop
            assert!(match new_msg { 
                        ExecutionState::Running(tasks) => {
                            assert!(tasks.iter().find(|t| t.name == "apple").unwrap().state == State::Running);
                            true
                        },
                        _ => false })
        } else {
            unreachable!("Uncaught message");
        }
    } 
}

// todo write test for rejecting non "shell" execution types
