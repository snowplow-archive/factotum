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
use std::sync::mpsc;
use factotum::executor::{ExecutionState, TaskSnapshot, JobTransition, Transition, ExecutionUpdate};
use std::time::Duration;

fn mock_200_ok(_: &str, _: &str) -> Result<u32, (u32, String)> {
    Ok(200)
}

fn mock_500_err(_: &str, _: &str) -> Result<u32, (u32, String)> {
    Err((500, "Internal Server Error".to_string()))
}

fn zero_backoff() -> Duration {
    Duration::from_secs(0)
}

#[test]
#[ignore] // this fails occasionally
fn backoff_retry_1min_good() {
    let min = Duration::from_secs(0);
    let max = Duration::from_secs(60);
    let mut prev = Duration::from_secs(0);
    for _ in 0..1000 {
        let test_d = backoff_rand_1_minute();
        assert!(test_d >= min);
        assert!(test_d <= max);
        assert!(prev != test_d);
        prev = test_d;
    }
}

#[test]
fn webhook_object_constructed_good() {
    let wh = Webhook::new("job_name", "hello", "https://goodplace.com", None, None);
    assert_eq!("hello", wh.factfile_json);
    assert_eq!("https://goodplace.com", wh.endpoint);
    assert_eq!("job_name", wh.factfile_job_name);
}

#[test]
fn finish_stops_thread() {
    let mut wh = Webhook::new("job_name", "hello", "https://goodplace.com", None, None);
    let (tx, rx) = mpsc::channel::<ExecutionUpdate>();
    let jh = wh.connect_webhook(rx, mock_200_ok, zero_backoff);
    let sent_state =
        ExecutionUpdate::new(ExecutionState::Finished,
                             TaskSnapshot::new(),
                             Transition::Job(JobTransition::new(Some(ExecutionState::Running),
                                                                ExecutionState::Finished)));
    tx.send(sent_state.clone()).unwrap();
    let result = jh.join();
    assert_eq!(result.ok().unwrap(),
               WebhookResult::new(1, 0, 1, vec![Ok(Attempt::new(Some(200), "OK", sent_state))]));
}

fn make_mock_run() -> Vec<ExecutionUpdate> {

    vec![ 
        ExecutionUpdate::new(ExecutionState::Started, 
                            TaskSnapshot::new(),
                            Transition::Job(JobTransition::new(None, ExecutionState::Started))),

        ExecutionUpdate::new(ExecutionState::Running, 
                            TaskSnapshot::new(),
                            Transition::Job(JobTransition::new(Some(ExecutionState::Started), ExecutionState::Running))),

        ExecutionUpdate::new(ExecutionState::Running, 
                            TaskSnapshot::new(),
                            Transition::Job(JobTransition::new(Some(ExecutionState::Running), ExecutionState::Running))),

        ExecutionUpdate::new(ExecutionState::Finished, 
                            TaskSnapshot::new(),
                            Transition::Job(JobTransition::new(Some(ExecutionState::Running), ExecutionState::Finished))),
    ]
}

#[test]
fn multiple_messages_sent() {
    let mut wh = Webhook::new("job_name", "hello", "https://goodplace.com", None, None);
    let (tx, rx) = mpsc::channel::<ExecutionUpdate>();
    let jh = wh.connect_webhook(rx, mock_200_ok, zero_backoff);

    let sent_states = make_mock_run();

    for state in sent_states.iter() {
        tx.send(state.clone()).unwrap();
    }

    let expected_results = vec![Ok(Attempt::new(Some(200), "OK", sent_states[0].clone())),
                                Ok(Attempt::new(Some(200), "OK", sent_states[1].clone())),
                                Ok(Attempt::new(Some(200), "OK", sent_states[2].clone())),
                                Ok(Attempt::new(Some(200), "OK", sent_states[3].clone()))];

    let result = jh.join();

    assert_eq!(result.ok().unwrap(),
               WebhookResult::new(sent_states.len() as u32, 0, 4, expected_results));
}

#[test]
fn failures_tried_three_times() {
    let mut wh = Webhook::new("job_name", "hello", "https://goodplace.com", None, None);
    let (tx, rx) = mpsc::channel::<ExecutionUpdate>();
    let jh = wh.connect_webhook(rx, mock_500_err, zero_backoff);

    let sent_states = make_mock_run();

    for state in sent_states.iter() {
        tx.send(state.clone()).unwrap();
    }

    let expected_results =
        vec![Err(Attempt::new(Some(500), "Internal Server Error", sent_states[0].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[0].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[0].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[1].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[1].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[1].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[2].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[2].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[2].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[3].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[3].clone())),
             Err(Attempt::new(Some(500), "Internal Server Error", sent_states[3].clone()))];

    let result = jh.join();

    assert_eq!(result.ok().unwrap(),
               WebhookResult::new(sent_states.len() as u32,
                                  expected_results.len() as u32,
                                  0,
                                  expected_results));
}

#[test]
#[ignore] // this only makes sense if you have ngrok set up
fn test_webhook_post() {
    let r = Webhook::http_post("***", r#"{"hello":"world"}"#);

    match r {
        Ok(code) => assert_eq!(code, 200),
        Err((code, msg)) => panic!("sending post failed with {}, \"{}\"", code, msg), 
    }
}

#[test]
fn bad_urls_post_rejects() {
    let r = Webhook::http_post("http://****/?", r#"{"hello":"world"}"#);

    match r {
        Ok(_) => unreachable!("Test returned good for invalid url"),
        Err((code, msg)) => {
            assert_eq!(code, 0);
            assert!(msg.contains("failed to lookup address information:"));
        }
    }
}
