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

#[cfg(test)]
mod tests;

static JOB_UPDATE_SCHEMA_NAME: &'static str = "iglu:com.snowplowanalytics.\
                                               factotum/job_update/jsonschema/1-0-0";
static TASK_UPDATE_SCHEMA_NAME: &'static str = "iglu:com.snowplowanalytics.\
                                               factotum/task_update/jsonschema/1-0-0";

use factotum::executor::{ExecutionState, ExecutionUpdate, TaskSnapshot,
                         Transition as ExecutorTransition};
use super::jobcontext::JobContext;
use chrono::UTC;
use std::collections::BTreeMap;
use rustc_serialize::Encodable;
use rustc_serialize;
use rustc_serialize::json::{self, ToJson, Json};
use factotum::executor::task_list::State;

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq)]
pub enum JobRunState {
    RUNNING,
    WAITING,
    COMPLETED,
    FAILED,
}

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq)]
pub enum TaskRunState {
    RUNNING,
    WAITING,
    COMPLETED,
    FAILED,
    SKIPPED,
}

#[derive(RustcDecodable, Debug, PartialEq)]
#[allow(non_snake_case)]
pub struct TaskUpdate {
    taskName: String,
    state: TaskRunState,
    started: Option<String>,
    duration: Option<String>,
    stdout: Option<String>,
    stderr: Option<String>,
    returnCode: Option<i32>,
    errorMessage: Option<String>,
}

impl Encodable for TaskUpdate {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.to_json().encode(s)
    }
}

impl ToJson for TaskUpdate {
    fn to_json(&self) -> Json {

        let mut d = BTreeMap::new();

        // don't emit optional fields

        match self.errorMessage {
            Some(ref value) => {
                d.insert("errorMessage".to_string(), value.to_json());
            }
            None => {}
        }

        match self.returnCode {
            Some(ref value) => {
                d.insert("returnCode".to_string(), value.to_json());
            }
            None => {}
        }

        match self.stderr {
            Some(ref value) => {
                d.insert("stderr".to_string(), value.to_json());
            }
            None => {}
        }

        match self.stdout {
            Some(ref value) => {
                d.insert("stdout".to_string(), value.to_json());
            }
            None => {}
        }

        match self.duration {
            Some(ref value) => {
                d.insert("duration".to_string(), value.to_json());
            }
            None => {}
        }

        match self.started {
            Some(ref value) => {
                d.insert("started".to_string(), value.to_json());
            }
            None => {}
        }

        d.insert("taskName".to_string(), self.taskName.to_json());
        d.insert("state".to_string(),
                 Json::from_str(&json::encode(&self.state).unwrap()).unwrap());


        Json::Object(d)
    }
}

#[derive(RustcEncodable, Debug)]
pub struct SelfDescribingWrapper<'a> {
    pub schema: String,
    pub data: &'a JobUpdate,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ApplicationContext {
    version: String,
    name: String
}

impl ApplicationContext {
    pub fn new(context: &JobContext) -> Self {
        ApplicationContext {
            version: context.factotum_version.clone(),
            name: "factotum".to_string()
        }
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
#[allow(non_snake_case)]
pub struct JobTransition {
    previousState: Option<JobRunState>,
    currentState: JobRunState,
}

impl JobTransition {
    pub fn new(prev_state: &Option<ExecutionState>,
               current_state: &ExecutionState,
               task_snap: &TaskSnapshot)
               -> Self {
        JobTransition {
            previousState: match *prev_state { 
                Some(ref s) => Some(to_job_run_state(s, task_snap)),
                None => None,
            },
            currentState: to_job_run_state(current_state, task_snap),
        }
    }
}

fn to_job_run_state(state: &ExecutionState, tasks: &TaskSnapshot) -> JobRunState {
    match *state {
        ExecutionState::Started => JobRunState::WAITING,
        ExecutionState::Finished => {
            // if any tasks failed, set to failed
            let failed_tasks = tasks.iter()
                .any(|t| match t.state {
                    State::Failed(_) => true,
                    _ => false,
                });
            if failed_tasks {
                JobRunState::FAILED
            } else {
                JobRunState::COMPLETED
            }
        }
        _ => JobRunState::RUNNING,
    }
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
#[allow(non_snake_case)]
pub struct TaskTransition {
    previousState: TaskRunState,
    currentState: TaskRunState,
    taskName: String,
}

#[derive(RustcDecodable, Debug)]
#[allow(non_snake_case)]
pub struct JobUpdate {
    jobName: String,
    jobReference: String,
    runReference: String,
    factfile: String,
    applicationContext: ApplicationContext,
    runState: JobRunState,
    startTime: String,
    runDuration: String,
    transition: Option<JobTransition>,
    transitions: Option<Vec<TaskTransition>>,
    taskStates: Vec<TaskUpdate>,
}

impl JobUpdate {
    pub fn new(context: &JobContext, execution_update: &ExecutionUpdate) -> Self {
        JobUpdate {
            jobName: context.job_name.clone(),
            jobReference: context.job_reference.clone(),
            runReference: context.run_reference.clone(),
            factfile: context.factfile.clone(),
            applicationContext: ApplicationContext::new(&context),
            runState: to_job_run_state(&execution_update.execution_state,
                                       &execution_update.task_snapshot),
            startTime: context.start_time.to_rfc3339(),
            runDuration: (UTC::now() - context.start_time).to_string(),
            taskStates: JobUpdate::to_task_states(&execution_update.task_snapshot),
            transition: {
                match execution_update.transition {
                    ExecutorTransition::Job(ref j) => {
                        Some(JobTransition::new(&j.from, &j.to, &execution_update.task_snapshot))
                    }
                    _ => None,
                }
            },
            transitions: {
                match execution_update.transition {
                    ExecutorTransition::Task(ref tu) => {
                        let tasks = tu.iter()
                            .map(|t| {
                                TaskTransition {
                                    taskName: t.task_name.clone(),
                                    previousState: match t.from_state {
                                        State::Waiting => TaskRunState::WAITING,
                                        State::Running => TaskRunState::RUNNING,
                                        State::Skipped(_) => TaskRunState::SKIPPED,
                                        State::Success => TaskRunState::COMPLETED,
                                        State::SuccessNoop => TaskRunState::COMPLETED,
                                        State::Failed(_) => TaskRunState::FAILED,
                                    },
                                    currentState: match t.to_state {
                                        State::Waiting => TaskRunState::WAITING,
                                        State::Running => TaskRunState::RUNNING,
                                        State::Skipped(_) => TaskRunState::SKIPPED,
                                        State::Success => TaskRunState::COMPLETED,
                                        State::SuccessNoop => TaskRunState::COMPLETED,
                                        State::Failed(_) => TaskRunState::FAILED,
                                    },
                                }
                            })
                            .collect();
                        Some(tasks)
                    }
                    _ => None,
                }
            },
        }
    }

    pub fn as_self_desc_json(&self) -> String {
        let wrapped = SelfDescribingWrapper {
            schema: match self.transition {
                Some(_) => JOB_UPDATE_SCHEMA_NAME.to_string(),
                None => TASK_UPDATE_SCHEMA_NAME.to_string(),
            },
            data: &self,
        };
        json::encode(&wrapped).unwrap()
    }

    fn to_task_states(tasks: &TaskSnapshot) -> Vec<TaskUpdate> {
        use chrono::duration::Duration as ChronoDuration;

        tasks.iter()
            .map(|task| {
                TaskUpdate {
                    taskName: task.name.clone(),
                    state: match task.state {
                        State::Waiting => TaskRunState::WAITING,
                        State::Running => TaskRunState::RUNNING,
                        State::Skipped(_) => TaskRunState::SKIPPED,
                        State::Success => TaskRunState::COMPLETED,
                        State::SuccessNoop => TaskRunState::COMPLETED,
                        State::Failed(_) => TaskRunState::FAILED,
                    },
                    started: if let Some(ref r) = task.run_started {
                        Some(r.to_rfc3339())
                    } else {
                        None
                    },
                    duration: if let Some(ref r) = task.run_result {
                        Some(ChronoDuration::from_std(r.duration).unwrap().to_string())
                    } else {
                        None
                    },
                    stdout: if let Some(ref r) = task.run_result {
                        if let Some(ref stdout) = r.stdout {
                            Some(stdout.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    },
                    stderr: if let Some(ref r) = task.run_result {
                        if let Some(ref stderr) = r.stderr {
                            Some(stderr.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    },
                    returnCode: if let Some(ref r) = task.run_result {
                        Some(r.return_code)
                    } else {
                        None
                    },
                    errorMessage: if let Some(ref r) = task.run_result {
                        if let Some(ref err) = r.task_execution_error {
                            Some(err.clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    },
                }
            })
            .collect()
    }
}

impl Encodable for JobUpdate {
    fn encode<S: rustc_serialize::Encoder>(&self, s: &mut S) -> Result<(), S::Error> {
        self.to_json().encode(s)
    }
}

impl ToJson for JobUpdate {
    fn to_json(&self) -> Json {

        let mut d = BTreeMap::new();

        d.insert("jobName".to_string(), self.jobName.to_json());
        d.insert("jobReference".to_string(), self.jobReference.to_json());
        d.insert("runReference".to_string(), self.runReference.to_json());
        d.insert("factfile".to_string(), self.factfile.to_json());

        d.insert("applicationContext".to_string(),
                 Json::from_str(&json::encode(&self.applicationContext).unwrap()).unwrap());

        d.insert("runState".to_string(),
                 Json::from_str(&json::encode(&self.runState).unwrap()).unwrap());

        d.insert("startTime".to_string(), self.startTime.to_json());
        d.insert("runDuration".to_string(), self.runDuration.to_json());

        match self.transition {
            Some(ref job_transition) => {
                d.insert("jobTransition".to_string(),
                         Json::from_str(&json::encode(&job_transition).unwrap()).unwrap());
            }
            None => {}
        }

        match self.transitions {
            Some(ref task_transition) => {
                d.insert("taskTransitions".to_string(),
                         Json::from_str(&json::encode(&task_transition).unwrap()).unwrap());
            } 
            None => {}
        }

        d.insert("taskStates".to_string(),
                 Json::from_str(&json::encode(&self.taskStates).unwrap()).unwrap());

        Json::Object(d)
    }
}
