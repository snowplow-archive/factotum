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

use super::*;
use factotum::parser::schemavalidator;
use factotum::executor::{ExecutionState, ExecutionUpdate, Transition,
                         JobTransition as ExecutorJobTransition,
                         TaskTransition as ExecutorTaskTransition};
use factotum::webhook::jobcontext::JobContext;
use chrono::UTC;
use factotum::tests::make_task;
use factotum::factfile::Factfile;
use factotum::executor::task_list::State;
use factotum::executor::{get_task_execution_list, get_task_snapshot};

#[test]
fn to_json_valid_against_schema_job_transition() {
    let schema = include_str!("../../../../tests/resources/job_update/job_transition_self_desc.\
                               json");
    let context = JobContext::new("hello", "world");
    let exec_update =
        ExecutionUpdate::new(ExecutionState::Finished,
                             vec![],
                             Transition::Job(ExecutorJobTransition::new(Some(ExecutionState::Running),
                                                                   ExecutionState::Finished)));
    let job_update = JobUpdate::new(&context, &exec_update);
    let json_wrapped = job_update.as_self_desc_json();
    println!("{}", json_wrapped);
    let result = schemavalidator::validate_schema(&json_wrapped, schema);
    match result {
        Ok(_) => (), // happy path
        Err(msg) => panic!("Failed to parse job update: {}", msg),
    }
}

#[test]
fn to_json_valid_against_schema_task_transition_running_to_failed() {
    let schema = include_str!("../../../../tests/resources/job_update/task_transition_self_desc.\
                               json");

    let mut ff = Factfile::new("N/A", "test");
    ff.add_task_obj(&make_task("apple", &vec![]));
    ff.add_task_obj(&make_task("turnip", &vec![]));
    ff.add_task_obj(&make_task("orange", &vec!["apple"]));
    ff.add_task_obj(&make_task("egg", &vec!["apple"]));
    ff.add_task_obj(&make_task("potato", &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato", "orange"]));

    let mut tasks = get_task_snapshot(&get_task_execution_list(&ff, None));

    let context = JobContext::new("hello", "world");

    for mut task in tasks.iter_mut() {
        task.state = State::Failed("a reason".to_string());
    }

    let transitions = tasks.iter()
        .map(|task| {
            ExecutorTaskTransition::new(&task.name,
                                        State::Running,
                                        State::Failed("a reason".to_string()))
        })
        .collect();

    let exec_update = ExecutionUpdate::new(ExecutionState::Finished,
                                           tasks,
                                           Transition::Task(transitions));

    let job_update = JobUpdate::new(&context, &exec_update);
    let json_wrapped = job_update.as_self_desc_json();

    println!("{}", json_wrapped);
    let result = schemavalidator::validate_schema(&json_wrapped, schema);
    match result {
        Ok(_) => (), // happy path
        Err(msg) => panic!("Failed to parse job update: {}", msg),
    }
}


#[test]
fn to_json_valid_against_schema_task_transition_waiting_to_running() {
    let schema = include_str!("../../../../tests/resources/job_update/task_transition_self_desc.\
                               json");

    let mut ff = Factfile::new("N/A", "test");
    ff.add_task_obj(&make_task("apple", &vec![]));
    ff.add_task_obj(&make_task("turnip", &vec![]));
    ff.add_task_obj(&make_task("orange", &vec!["apple"]));
    ff.add_task_obj(&make_task("egg", &vec!["apple"]));
    ff.add_task_obj(&make_task("potato", &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato", "orange"]));

    let mut tasks = get_task_snapshot(&get_task_execution_list(&ff, None));

    let context = JobContext::new("hello", "world");

    for mut task in tasks.iter_mut() {
        task.state = State::Running;
    }

    let transitions = tasks.iter()
        .map(|task| ExecutorTaskTransition::new(&task.name, State::Waiting, State::Running))
        .collect();

    let exec_update = ExecutionUpdate::new(ExecutionState::Running,
                                           tasks,
                                           Transition::Task(transitions));

    let job_update = JobUpdate::new(&context, &exec_update);
    let json_wrapped = job_update.as_self_desc_json();
    println!("{}", json_wrapped);

    let result = schemavalidator::validate_schema(&json_wrapped, schema);
    match result {
        Ok(_) => (), // happy path
        Err(msg) => panic!("Failed to parse job update: {}", msg),
    }
}

#[test]
fn to_task_states_empty() {
    let empty = vec![];
    assert!(JobUpdate::to_task_states(&empty).is_empty());
}

#[test]
fn headers_correct() {
    let context = JobContext::new("hello", "world");
    let exec_update =
        ExecutionUpdate::new(ExecutionState::Finished,
                             vec![],
                             Transition::Job(ExecutorJobTransition::new(Some(ExecutionState::Running),
                                                                ExecutionState::Finished)));
    let job_update = JobUpdate::new(&context, &exec_update);

    assert_eq!(context.job_reference, job_update.jobReference);
    assert_eq!(context.run_reference, job_update.runReference);
    assert_eq!(context.job_name, job_update.jobName);
    assert_eq!(context.factfile, job_update.factfile);
    assert_eq!(context.factotum_version,
               job_update.applicationContext.version);
    assert_eq!(job_update.runState, JobRunState::SUCCEEDED);
    assert_eq!(job_update.startTime.len(), UTC::now().to_rfc3339().len());
    assert!(job_update.runDuration.contains("PT0"));
    assert!(job_update.taskStates.len() == 0);
}

#[test]
fn failed_headers_correct() {
    let mut ff = Factfile::new("N/A", "test");
    ff.add_task_obj(&make_task("apple", &vec![]));
    ff.add_task_obj(&make_task("turnip", &vec![]));
    ff.add_task_obj(&make_task("orange", &vec!["apple"]));
    ff.add_task_obj(&make_task("egg", &vec!["apple"]));
    ff.add_task_obj(&make_task("potato", &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato", "orange"]));

    let mut tasks = get_task_snapshot(&get_task_execution_list(&ff, None));

    for mut task in tasks.iter_mut() {
        task.state = State::Failed("a reason".to_string());
    }

    let context = JobContext::new("hello", "world");
    let exec_update =
        ExecutionUpdate::new(ExecutionState::Finished,
                             tasks,
                             Transition::Job(ExecutorJobTransition::new(Some(ExecutionState::Running),
                                                                ExecutionState::Finished)));
    let upd = JobUpdate::new(&context, &exec_update);

    assert_eq!(upd.runState, JobRunState::FAILED);
}

#[test]
fn task_states_converted_no_run_data() {
    use factotum::executor::task_list::Task;
    use factotum::tests::make_task;

    let example_tasks = vec![Task::new("chocolate", make_task("hello", &vec![]))];
    let start_sample =
        ExecutionUpdate::new(ExecutionState::Started,
                             example_tasks,
                             Transition::Job(ExecutorJobTransition::new(None,
                                                                        ExecutionState::Started)));

    let context = JobContext::new("hello", "world");
    let job_update = JobUpdate::new(&context, &start_sample);

    let expected_state = TaskUpdate {
        taskName: "chocolate".to_string(),
        state: TaskRunState::WAITING,
        started: None,
        duration: None,
        stdout: None,
        stderr: None,
        returnCode: None,
        errorMessage: None,
    };

    assert!(job_update.taskStates.is_empty() == false);
    assert_eq!(job_update.taskStates[0], expected_state);
}

#[test]
fn task_states_converted_with_run_data() {
    use factotum::executor::task_list::State;
    use factotum::executor::task_list::Task;
    use factotum::tests::make_task;
    use factotum::executor::execution_strategy::RunResult;
    use chrono::Duration;

    let mut example_tasks = vec![Task::new("chocolate", make_task("hello", &vec![])),
                                 Task::new("toffee", make_task("boop", &vec![]))];

    let now = UTC::now();

    example_tasks[0].state = State::Failed("broken".to_string());
    example_tasks[0].run_started = Some(now.clone());
    example_tasks[0].run_result = Some(RunResult {
        return_code: -1,
        task_execution_error: Some("some continue job stuff".to_string()),
        stderr: Some("banana".to_string()),
        stdout: Some("get".to_string()),
        duration: Duration::seconds(0).to_std().unwrap(),
    });

    example_tasks[1].state = State::Success;
    example_tasks[1].run_started = Some(now.clone());
    example_tasks[1].run_result = Some(RunResult {
        return_code: 0,
        task_execution_error: None,
        stderr: None,
        stdout: None,
        duration: Duration::seconds(1).to_std().unwrap(),
    });

    let start_sample =
        ExecutionUpdate::new(ExecutionState::Started,
                             example_tasks,
                             Transition::Job(ExecutorJobTransition::new(None,
                                                                        ExecutionState::Started)));

    let context = JobContext::new("hello", "world");
    let job_update = JobUpdate::new(&context, &start_sample);

    let expected_states = vec![TaskUpdate {
                                   taskName: "chocolate".to_string(),
                                   state: TaskRunState::FAILED,
                                   started: Some(now.to_rfc3339()),
                                   duration: Some(Duration::seconds(0).to_string()),
                                   stdout: Some("get".to_string()),
                                   stderr: Some("banana".to_string()),
                                   returnCode: Some(-1),
                                   errorMessage: Some("some continue job stuff".to_string()),
                               },
                               TaskUpdate {
                                   taskName: "toffee".to_string(),
                                   state: TaskRunState::SUCCEEDED,
                                   started: Some(now.to_rfc3339()),
                                   duration: Some(Duration::seconds(1).to_string()),
                                   stdout: None,
                                   stderr: None,
                                   returnCode: Some(0),
                                   errorMessage: None,
                               }];

    assert!(job_update.taskStates.is_empty() == false);
    assert_eq!(job_update.taskStates[0], expected_states[0]);
    assert_eq!(job_update.taskStates[1], expected_states[1]);
}
