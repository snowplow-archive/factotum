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

#[macro_use]
pub mod command;
pub mod server;
pub mod dispatcher;
pub mod persistence;
pub mod responder;

#[cfg(test)]
mod tests;

use std::collections::VecDeque;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::{Mutex, RwLock};
use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::thread;
use std::thread::JoinHandle;
use iron::prelude::*;
use iron::typemap::Key;
use logger::Logger;
use persistent::{Read, State};
use threadpool::ThreadPool;

use Args;
use factotum_server::command::{CommandStore, Execution};
use factotum_server::dispatcher::{Dispatch, Dispatcher, Query};
use factotum_server::persistence::{Persistence, ConsulPersistence, JobState};
use factotum_server::responder::{DispatcherStatus, JobStatus, WorkerStatus};
use factotum_server::server::{ServerManager, JobRequest};

#[derive(Debug, Copy, Clone)]
pub struct Server;
impl Key for Server {
    type Value = ServerManager;
}

#[derive(Debug, Copy, Clone)]
pub struct Storage;
impl Key for Storage {
    type Value = ConsulPersistence;
}

#[derive(Debug, Copy, Clone)]
pub struct Paths;
impl Key for Paths {
    type Value = RwLock<CommandStore>;
}

#[derive(Debug, Copy, Clone)]
pub struct Updates;
impl Key for Updates {
    type Value = Mutex<Sender<Dispatch>>;
}

pub fn start(args: Args) {
    let server = ServerManager::new(args.flag_ip, args.flag_port, args.flag_webhook, args.flag_no_colour);
    let persistence = ConsulPersistence::new(args.flag_consul_name, args.flag_consul_ip, args.flag_consul_port, args.flag_consul_namespace);
    let dispatcher = Dispatcher::new(args.flag_max_jobs, args.flag_max_workers);
    let command_store = commands![::FACTOTUM.to_string() => args.flag_factotum_bin];
    
    let address = SocketAddr::from_str(&format!("{}:{}", server.ip, server.port)).expect("Failed to parse socket address");

    let (requests_channel, _, _) = trigger_worker_manager(dispatcher, persistence.clone(), &command_store).unwrap();

    let router = router!(
        index:      get     "/"         =>  responder::api,
        help:       get     "/help"     =>  responder::api,
        status:     get     "/status"   =>  responder::status,
        settings:   post    "/settings" =>  responder::settings,
        submit:     post    "/submit"   =>  responder::submit,
        check:      post    "/check"    =>  responder::check
    );
    let (logger_before, logger_after) = Logger::new(None);

    let mut chain = Chain::new(router);
    chain.link_before(logger_before);
    chain.link(State::<Server>::both(server));
    chain.link(State::<Storage>::both(persistence));
    chain.link(Read::<Paths>::both(RwLock::new(command_store)));
    chain.link(Read::<Updates>::both(Mutex::new(requests_channel)));
    chain.link_after(logger_after);
    
    match Iron::new(chain).http(address) {
        Ok(listening) => {
            let socket_addr = listening.socket;
            let ip = socket_addr.ip();
            let port = socket_addr.port();
            let start_message = format!("Factotum Server version [{}] listening on [{}:{}]", ::VERSION, ip, port);
            info!("{}", start_message);
            println!("{}", start_message)
        }
        Err(e) => println!("Failed to start server - {}", e)
    }
}

// Concurrent dispatch

pub fn trigger_worker_manager<T: 'static + Clone + Persistence + Send>(dispatcher: Dispatcher, persistence: T, command_store: &CommandStore) -> Result<(Sender<Dispatch>, JoinHandle<String>, ThreadPool), String> {
    let (tx, rx) = mpsc::channel();
    let primary_pool = ThreadPool::new_with_name("primary_pool".to_string(), dispatcher.max_workers);

    let join_handle = spawn_worker_manager(tx.clone(), rx, dispatcher.requests_queue, dispatcher.max_jobs, primary_pool.clone(), persistence, command_store.clone());

    Ok((tx, join_handle, primary_pool))
}

fn spawn_worker_manager<T: 'static + Clone + Persistence + Send>(job_requests_tx: Sender<Dispatch>, job_requests_rx: Receiver<Dispatch>, requests_queue: VecDeque<JobRequest>, max_jobs: usize, primary_pool: ThreadPool, persistence: T, command_store: CommandStore) -> JoinHandle<String> {
    let mut requests_queue = requests_queue;
    let mut is_processing = true;
    thread::spawn(move || {
        while is_processing {
            let message = job_requests_rx.recv().expect("Error receiving message in channel");

            match message {
                Dispatch::StatusUpdate(query) => send_status_update(query, &mut requests_queue, max_jobs, primary_pool.clone()),
                Dispatch::CheckQueue(query) => is_queue_full(query, &mut requests_queue, max_jobs),
                Dispatch::NewRequest(request) => new_job_request(job_requests_tx.clone(), &mut requests_queue, primary_pool.clone(), persistence.clone(), request),
                Dispatch::ProcessRequest => process_job_request(job_requests_tx.clone(), &mut requests_queue, primary_pool.clone(), persistence.clone(), command_store.clone()),
                Dispatch::RequestComplete(request) => complete_job_request(job_requests_tx.clone(), persistence.clone(), request),
                Dispatch::RequestFailure(request) => failed_job_request(job_requests_tx.clone(), persistence.clone(), request),
                Dispatch::StopProcessing => is_processing = stop_processing(),
            }
        }
        String::from("EXITING WORKER MANAGER")
    })
}

fn send_status_update(query: Query<DispatcherStatus>, requests_queue: &mut VecDeque<JobRequest>, max_jobs: usize, primary_pool: ThreadPool) {
    let tx = query.status_tx;
    let total_workers = primary_pool.max_count();
    let active_workers = primary_pool.active_count();
    let result = DispatcherStatus {
        workers: WorkerStatus {
            total: total_workers,
            idle: total_workers - active_workers,
            active: active_workers,
        },
        jobs: JobStatus {
            max_queue_size: max_jobs,
            in_queue: requests_queue.len(),
            fail_count: 0,
            success_count: 0
        }
    };
    tx.send(result).expect("Server status channel receiver has been deallocated");
}

fn is_queue_full(query: Query<bool>, requests_queue: &mut VecDeque<JobRequest>, max_jobs: usize) {
    let tx = query.status_tx;
    let is_full = requests_queue.len() >= max_jobs;
    tx.send(is_full).expect("Queue query channel receiver has been deallocated");
}

fn new_job_request<T: Persistence>(requests_channel: Sender<Dispatch>, requests_queue: &mut VecDeque<JobRequest>, primary_pool: ThreadPool, persistence: T, request: JobRequest) {
    debug!("ADDING NEW JOB jobId:[{}]", request.job_id);
    requests_queue.push_back(request.clone());
    // Create entry in persistence storage
    persist_entry(&persistence, request.job_id.clone(), request.clone(), JobState::Queued);
    // Check queue size - return error if limit exceeded (not important right now)
    if primary_pool.active_count() < primary_pool.max_count() {
        requests_channel.send(Dispatch::ProcessRequest).expect("Job requests channel receiver has been deallocated");
    } else {
        info!("No threads available - waiting for a job to complete.")
    }
}

fn process_job_request<T: 'static + Persistence + Send>(requests_channel: Sender<Dispatch>, requests_queue: &mut VecDeque<JobRequest>, primary_pool: ThreadPool, persistence: T, command_store: CommandStore) {
    debug!("QUEUE SIZE = {}", requests_queue.len());
    match requests_queue.pop_front() {
        Some(request) => {
            primary_pool.execute(move || {
                debug!("PROCESSING JOB REQ jobId:[{}]", request.job_id);
                // Update status in persistence storage
                persist_entry(&persistence, request.job_id.clone(), request.clone(), JobState::Working);
                let cmd_path = match command_store.get_command(::FACTOTUM) {
                    Ok(path) => path,
                    Err(e) => {
                        error!("{}", e);
                        requests_channel.send(Dispatch::RequestFailure(request)).expect("Job requests channel receiver has been deallocated");
                        return
                    }
                };
                let mut cmd_args = vec!["run".to_string(), request.factfile_path.clone()];
                cmd_args.extend_from_slice(request.factfile_args.as_slice());
                match command_store.execute(cmd_path, cmd_args) {
                    Ok(output) => {
                        trace!("{}", output);
                        requests_channel.send(Dispatch::RequestComplete(request)).expect("Job requests channel receiver has been deallocated");
                    },
                    Err(e) => {
                        error!("{}", e);
                        requests_channel.send(Dispatch::RequestFailure(request)).expect("Job requests channel receiver has been deallocated");
                    }
                };
            });
        }
        None => debug!("QUEUE EMPTY")
    }
}

fn complete_job_request<T: Persistence>(requests_channel: Sender<Dispatch>, persistence: T, request: JobRequest) {
    info!("COMPLETED JOB REQ  jobId:[{}]", request.job_id);
    // Update completion in persistence storage
    persist_entry(&persistence, request.job_id.clone(), request, JobState::Done);
    requests_channel.send(Dispatch::ProcessRequest).expect("Job requests channel receiver has been deallocated");
}

fn failed_job_request<T: Persistence>(requests_channel: Sender<Dispatch>, persistence: T, request: JobRequest) {
    error!("FAILED JOB REQ jobId:[{}]", request.job_id);
    // Update failure in persistence storage
    persist_entry(&persistence, request.job_id.clone(), request, JobState::Done);
    requests_channel.send(Dispatch::ProcessRequest).expect("Job requests channel receiver has been deallocated");
}

fn stop_processing() -> bool {
    info!("STOPPING");
    false
}

fn persist_entry<T: Persistence>(persistence: &T, client_job_id: String, job_request: JobRequest, job_state: JobState) {
    let output = persistence::set_entry(persistence, client_job_id.clone(), job_request, job_state.clone());
    if output {
        debug!("Persist [{}]::[{}]", client_job_id, job_state);
    } else {
        error!("Persistence Error: Failed to update [{}] to [{}]", client_job_id, job_state);
    }
}
