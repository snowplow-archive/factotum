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
use std::time::Duration;

#[test]
fn worker_manager_spawn_and_exit() {
    let (tx, rx) = mpsc::channel();
    let pool = ThreadPool::new(2);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];

    let handle = spawn_worker_manager(tx.clone(), rx, VecDeque::new(), 2, pool.clone(), persistence, command_store);
    tx.send(Dispatch::StopProcessing).unwrap();

    let output = handle.join().unwrap();
    assert_eq!("EXITING WORKER MANAGER", output);
}

#[test]
fn send_status_update_success() {
    let (tx, rx) = mpsc::channel();
    let query = Query::new("status_query".to_string(), tx);
    let pool = ThreadPool::new(2);
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);
    let mut requests_queue = VecDeque::new();
    requests_queue.push_back(job_request);

    send_status_update(query, &mut requests_queue, 10, pool);

    let actual = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    let expected = DispatcherStatus {
        workers: WorkerStatus {
            total: 2,
            idle: 2,
            active: 0,
        },
        jobs: JobStatus {
            max_queue_size: 10,
            in_queue: 1,
            fail_count: 0,
            success_count: 0
        }
    };
    assert_eq!(expected, actual);
}

#[test]
fn is_queue_full_true() {
    let (tx, rx) = mpsc::channel();
    let query = Query::new("queue_query".to_string(), tx);
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);
    let mut requests_queue = VecDeque::new();
    requests_queue.push_back(job_request.clone());
    requests_queue.push_back(job_request.clone());

    is_queue_full(query, &mut requests_queue, 2);

    let result = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(true, result);
}

#[test]
fn is_queue_full_false() {
    let (tx, rx) = mpsc::channel();
    let query = Query::new("queue_query".to_string(), tx);
    let mut requests_queue = VecDeque::new();

    is_queue_full(query, &mut requests_queue, 2);

    let result = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(false, result);
}

#[test]
fn new_job_request_success() {
    let (tx, rx) = mpsc::channel();
    let pool = ThreadPool::new(2);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);
    let mut requests_queue = VecDeque::new();

    new_job_request(tx.clone(), &mut requests_queue, pool.clone(), persistence, job_request.clone());
    
    let output = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(Dispatch::ProcessRequest, output);
    assert!(requests_queue.contains(&job_request));
}

#[test]
fn process_job_request_failure() {
    let (tx, rx) = mpsc::channel();
    let pool = ThreadPool::new(2);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands!["dummy".to_string() => "/tmp/fake_command".to_string()];
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);
    let mut requests_queue = VecDeque::new();
    requests_queue.push_back(job_request.clone());

    process_job_request(tx.clone(), &mut requests_queue, pool.clone(), persistence, command_store);

    let output = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(Dispatch::RequestFailure(job_request), output);
}

#[test]
fn complete_job_request_success() {
    let (tx, rx) = mpsc::channel();
    let persistence = ConsulPersistence::new(None, None, None, None);
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);

    complete_job_request(tx, persistence, job_request);

    let output = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(Dispatch::ProcessRequest, output);
}

#[test]
fn failed_job_request_success() {
    let (tx, rx) = mpsc::channel();
    let persistence = ConsulPersistence::new(None, None, None, None);
    let job_request = JobRequest::new("1", "dummy", "/tmp/somewhere", vec![]);

    failed_job_request(tx, persistence, job_request);

    let output = rx.recv_timeout(Duration::from_millis(1000)).unwrap();
    assert_eq!(Dispatch::ProcessRequest, output);
}
