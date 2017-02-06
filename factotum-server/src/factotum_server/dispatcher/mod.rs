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

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::sync::mpsc::Sender;
use factotum_server::server::JobRequest;
use factotum_server::responder::DispatcherStatus;

#[derive(Debug, PartialEq)]
pub enum Dispatch {
    StatusUpdate(Query<DispatcherStatus>),
    CheckQueue(Query<bool>),
    NewRequest(JobRequest),
    ProcessRequest,
    RequestComplete(JobRequest),
    RequestFailure(JobRequest),
    StopProcessing,
}

#[derive(Debug)]
pub struct Dispatcher {
    pub max_jobs: usize,
    pub max_workers: usize,
    pub requests_queue: VecDeque<JobRequest>,
}

impl Dispatcher {
    pub fn new(queue_size: usize, workers_size: usize) -> Dispatcher {
        Dispatcher {
            max_jobs: if queue_size > 0 { queue_size } else { ::MAX_JOBS_DEFAULT },
            max_workers: if workers_size > 0 { workers_size } else { ::MAX_WORKERS_DEFAULT },
            requests_queue: VecDeque::with_capacity(queue_size),
        }
    }
}

#[derive(Debug)]
pub struct Query<T> {
    pub name: String,
    pub status_tx: Sender<T>,
}

impl<T> Query<T> {
    pub fn new(name: String, status_tx: Sender<T>) -> Query<T> {
        Query {
            name: name,
            status_tx: status_tx,
        }
    }
}

impl<T> PartialEq for Query<T> {
    fn eq(&self, other: &Query<T>) -> bool {
        self.name == other.name
    }
}
