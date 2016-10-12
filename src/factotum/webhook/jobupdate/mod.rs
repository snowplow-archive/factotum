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

use factotum::executor::ExecutionState;
use rustc_serialize::json;
use super::jobcontext::JobContext;
use chrono::UTC;

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

#[derive(RustcDecodable, RustcEncodable, Debug, PartialEq)]
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

#[derive(RustcEncodable, Debug)]
pub struct SelfDescribingWrapper<'a> {
    pub schema: String,
    pub data: &'a JobUpdate,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
pub struct ApplicationContext {
    version: String,
}

#[derive(RustcDecodable, RustcEncodable, Debug)]
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
    taskStates: Vec<TaskUpdate>,
}

impl JobUpdate {
    pub fn new(context: &JobContext, execution_state: &ExecutionState) -> Self {
        JobUpdate {
            jobName: context.job_name.clone(),
            jobReference: context.job_reference.clone(),
            runReference: context.run_reference.clone(),
            factfile: context.factfile.clone(),
            applicationContext: ApplicationContext { version: context.factotum_version.clone() },
            runState: match *execution_state {
                ExecutionState::Started(_) => JobRunState::WAITING,
                ExecutionState::Finished(_) => JobRunState::COMPLETED,
                _ => JobRunState::RUNNING,
            },
            startTime: context.start_time.to_rfc3339(),
            runDuration: (UTC::now() - context.start_time).to_string(),
            taskStates: JobUpdate::to_task_states(execution_state),
        }
    }

    pub fn as_self_desc_json(&self) -> String {
        let wrapped = SelfDescribingWrapper {
            schema: JOB_UPDATE_SCHEMA_NAME.to_string(),
            data: &self,
        };
        json::encode(&wrapped).unwrap()
    }

    fn to_task_states(execution_state: &ExecutionState) -> Vec<TaskUpdate> {

        use factotum::executor::task_list::State;
        use chrono::duration::Duration as ChronoDuration;

        let tasks = match *execution_state {
            ExecutionState::Running(ref t) => t,
            ExecutionState::Started(ref t) => t,
            ExecutionState::Finished(ref t) => t, 
        };

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
