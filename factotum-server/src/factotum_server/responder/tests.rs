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
use factotum_server::persistence::ConsulPersistence;
use factotum_server::command::Execution;

#[test]
fn process_settings_fail_no_body() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(None);
    let mut server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);

    let (status, response) = process_settings(url, request_body, &mut server_manager);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Error: No body found in POST request"}"#);
}

#[test]
fn process_settings_fail_invalid_json() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Err(bodyparser::BodyError{
        detail: "dummy error".to_string(),
        cause: bodyparser::BodyErrorCause::IoError(::std::io::Error::new(::std::io::ErrorKind::Other, "bad stuff")),
    });
    let mut server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);

    let (status, response) = process_settings(url, request_body, &mut server_manager);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Error decoding JSON string: bad stuff"}"#);
}

#[test]
fn process_settings_fail_invalid_settings_request() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(Some(SettingsRequest::new("INVALID")));
    let mut server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);

    let (status, response) = process_settings(url, request_body, &mut server_manager);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Validation Error: Invalid 'state', must be one of (run|drain)"}"#);
}

#[test]
fn process_settings_success() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(Some(SettingsRequest::new("drain")));
    let mut server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);

    assert_eq!(server_manager.state, ::SERVER_STATE_RUN);

    let (status, response) = process_settings(url, request_body, &mut server_manager);

    assert_eq!(server_manager.state, ::SERVER_STATE_DRAIN);
    assert_eq!(status, status::Ok);
    assert_eq!(response, r#"{"message":"Update acknowledged: [state: drain]"}"#);
}

#[test]
fn process_submission_fail_no_body() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(None);
    let server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands![::FACTOTUM.to_string() => "/tmp/fake_command".to_string()];
    let (tx, _) = mpsc::channel();

    let (status, response) = process_submission(url, request_body, &server_manager, &persistence, &command_store, &tx);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Error: No body found in POST request"}"#);
}

#[test]
fn process_submission_fail_invalid_json() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Err(bodyparser::BodyError{
        detail: "dummy error".to_string(),
        cause: bodyparser::BodyErrorCause::IoError(::std::io::Error::new(::std::io::ErrorKind::Other, "bad stuff")),
    });
    let server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands![::FACTOTUM.to_string() => "/tmp/fake_command".to_string()];
    let (tx, _) = mpsc::channel();

    let (status, response) = process_submission(url, request_body, &server_manager, &persistence, &command_store, &tx);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Error decoding JSON string: bad stuff"}"#);
}

#[test]
fn process_submission_fail_server_in_drain_state() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(Some(JobRequest::new("1", "dummy", "/tmp/somewhere", vec!["--first-arg".to_string()])));
    let mut server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands![::FACTOTUM.to_string() => "/tmp/fake_command".to_string()];
    let (tx, _) = mpsc::channel();

    server_manager.state = ::SERVER_STATE_DRAIN.to_string();
    let (status, response) = process_submission(url, request_body, &server_manager, &persistence, &command_store, &tx);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Server in [drain] state - cannot submit job"}"#);
}

#[test]
fn process_submission_fail_invalid_job_request() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(Some(JobRequest::new("1", "", "/tmp/somewhere", vec!["--first-arg".to_string()])));
    let server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let command_store = commands![::FACTOTUM.to_string() => "/tmp/fake_command".to_string()];
    let (tx, _) = mpsc::channel();

    let (status, response) = process_submission(url, request_body, &server_manager, &persistence, &command_store, &tx);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Validation Error: No valid value found: field 'jobName' cannot be empty"}"#);
}

#[derive(Debug)]
struct NoopCommandMock;

impl Execution for NoopCommandMock {
    fn get_command(&self, _: &str) -> Result<String, String> {
        Ok("/noop/command".to_string())
    }

    fn execute(&self, _: String, _: Vec<String>) -> Result<String, String> {
        Ok("NOOP command".to_string())
    }
}

#[test]
#[ignore]
// Not able to test yet
fn process_submission_fail_job_already_run() {
    let url = Url::parse("http://not.a.real.address/").unwrap();
    let request_body = Ok(Some(JobRequest::new("1", "dummy", "/tmp", vec!["--no-colour".to_string()])));
    let server_manager = ServerManager::new(Some("0.0.0.0".to_string()), 8080, "http://dummy.test/".to_string(), false);
    let persistence = ConsulPersistence::new(None, None, None, None);
    let noop_command = NoopCommandMock;
    let (tx, rx) = mpsc::channel();

    let (status, response) = process_submission(url, request_body, &server_manager, &persistence, &noop_command, &tx);

    assert_eq!(status, status::BadRequest);
    assert_eq!(response, r#"{"message":"Job has already been run"}"#);
}