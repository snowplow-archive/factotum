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

pub mod jobcontext;
mod jobupdate;
#[cfg(test)]
mod tests;


use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc::Receiver;
use factotum::executor::{ExecutionState, ExecutionUpdate};
use std::time::Duration;
use rand;
use factotum::webhook::jobcontext::JobContext;
use std::collections::HashMap;

const MAX_RETRIES: usize = 3;

pub fn backoff_rand_1_minute() -> Duration {
    let max_duration_millis = 60 * 1000;
    let random_ms = rand::random::<u64>();
    Duration::from_millis(random_ms % max_duration_millis)
}

#[derive(Debug,Clone,PartialEq)]
pub struct Attempt {
    code: Option<u32>,
    message: String,
    execution_update: ExecutionUpdate,
}

impl Attempt {
    pub fn new<S: Into<String>>(code: Option<u32>,
                                message: S,
                                execution_update: ExecutionUpdate)
                                -> Self {
        Attempt {
            code: code,
            message: message.into(),
            execution_update: execution_update,
        }
    }
}

pub type WebhookAttemptResult = Result<Attempt, Attempt>;

#[derive(Debug,Clone,PartialEq)]
pub struct WebhookResult {
    pub events_received: u32,
    pub failed_count: u32,
    pub success_count: u32,
    pub results: Vec<WebhookAttemptResult>,
}

impl WebhookResult {
    pub fn new(events_received: u32,
               fail_count: u32,
               success_count: u32,
               results: Vec<WebhookAttemptResult>)
               -> Self {
        WebhookResult {
            events_received: events_received,
            failed_count: fail_count,
            success_count: success_count,
            results: results,
        }
    }
}

pub struct Webhook {
    pub factfile_job_name: String,
    pub factfile_json: String,
    pub endpoint: String,
    job_context: JobContext,
    pub max_stdouterr_size: usize,
}

impl Webhook {
    pub fn http_post(url: &str, data: &str) -> Result<u32, (u32, String)> {
        use hyper::Client;
        use hyper::header::{Headers, ContentType};
        use hyper::mime::{Mime, TopLevel, SubLevel, Attr, Value};
        use hyper::status;

        let client = Client::new();
        let mut headers = Headers::new();
        headers.set(ContentType(Mime(TopLevel::Application,
                                     SubLevel::Json,
                                     vec![(Attr::Charset, Value::Utf8)])));

        let res = client.post(url)
            .headers(headers)
            .body(data)
            .send();
        match res {
            Ok(g) => {
                if g.status == status::StatusCode::Ok {
                    Ok(g.status_raw().0 as u32)
                } else {
                    Err((g.status_raw().0 as u32, format!("{}", g.status)))
                }
            }
            Err(e) => Err((0, format!("{}", e))),
        }
    }

    pub fn new<S: Into<String>>(factfile_job_name: S, factfile_json: S, endpoint: S, job_tags:Option<HashMap<String,String>>, max_stdouterr_size:Option<usize>) -> Self {
        let ff_name: String = factfile_job_name.into();
        let ff_json: String = factfile_json.into();
        let jc = jobcontext::JobContext::new(ff_name.clone(), &ff_json, job_tags);

        let max_stdouterr_size_bytes: usize = if let Some(max_bytes) = max_stdouterr_size {
            max_bytes
        } else {
            10_000
        };

        Webhook {
            job_context: jc,
            factfile_job_name: ff_name,
            factfile_json: ff_json,
            endpoint: endpoint.into(),
            max_stdouterr_size: max_stdouterr_size_bytes,
        }
    }

    pub fn connect_webhook<F, G>(&mut self,
                                 updates_channel: Receiver<ExecutionUpdate>,
                                 emitter_func: F,
                                 backoff_retry_period: G)
                                 -> JoinHandle<WebhookResult>
        where F: Fn(&str, &str) -> Result<u32, (u32, String)> + Send + Sync + 'static + Copy,
              G: Fn() -> Duration + Send + Sync + 'static
    {

        let endpoint = self.endpoint.clone();
        let job_context = self.job_context.clone();
        let max_stdouterr_size = self.max_stdouterr_size.clone();

        thread::spawn(move || {

            let mut attempts = vec![];
            let mut fail_count = 0;
            let mut success_count = 0;
            let mut done = false;
            let mut events_recv = 0;

            while done == false {

                let message: ExecutionUpdate = updates_channel.recv().unwrap();
                events_recv += 1;

                if ExecutionState::Finished == message.execution_state {
                    done = true;
                }

                let job_update = jobupdate::JobUpdate::new(&job_context, &message, &max_stdouterr_size);
                let json_post_data = job_update.as_self_desc_json();

                for _ in 0..MAX_RETRIES {
                    let mut good = false;

                    let attempt = match emitter_func(&endpoint, &json_post_data) {
                        Ok(code) => {
                            success_count = success_count + 1;
                            good = true;
                            Ok(Attempt::new(Some(code), "OK", message.clone()))
                        }
                        Err((code, r)) => {
                            fail_count = fail_count + 1;
                            warn!("Failed to send webhook update to '{}': {}",
                                  &endpoint,
                                  &json_post_data);
                            warn!("Reason: {}, {}", code, r);
                            Err(Attempt::new(Some(code), r, message.clone()))
                        }
                    };

                    attempts.push(attempt);

                    if good {
                        break;
                    } else {
                        thread::sleep(backoff_retry_period());
                    }
                }
            }

            WebhookResult::new(events_recv, fail_count, success_count, attempts)
        })
    }
}
