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

use std::collections::HashMap;
use std::error;
use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use chrono::{DateTime, UTC};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use getopts::Options;
use serde_json;

use factotum_server::command::Execution;

#[cfg(test)]
mod tests;

#[derive(Debug)]
pub struct ServerManager {
    pub ip: String,
    pub port: u32,
    pub state: String,
    pub start_time: DateTime<UTC>,
    pub webhook_uri: String,
    pub no_colour: bool,
}

impl ServerManager {
    pub fn new(wrapped_ip: Option<String>, port: u32, webhook_uri: String, no_colour: bool) -> ServerManager {
        ServerManager {
            ip: if let Some(ip) = wrapped_ip { ip } else { ::IP_DEFAULT.to_string() },
            port: if port > 0 && port <= 65535 { port } else { ::PORT_DEFAULT },
            state: ::SERVER_STATE_RUN.to_string(),
            start_time: UTC::now(),
            webhook_uri: webhook_uri.to_string(),
            no_colour: no_colour,
        }
    }

    pub fn is_running(&self) -> bool {
        self.state == ::SERVER_STATE_RUN
    }

    pub fn get_start_time(&self) -> String {
        self.start_time.format("%F %T %Z").to_string()
    }

    pub fn get_uptime(&self) -> String {
        let uptime = UTC::now().signed_duration_since(self.start_time);
        let seconds = uptime.num_seconds() % 60;
        let minutes = uptime.num_minutes() % 60;
        let hours = uptime.num_hours() % 24;
        let days = uptime.num_days();
        format!("{} Days, {} Hours, {} Minutes, {} Seconds", days, hours, minutes, seconds)
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobRequest {
    #[serde(skip_deserializing)]
    pub job_id: String,
    pub job_name: String,
    pub factfile_path: String,
    pub factfile_args: Vec<String>
}

impl JobRequest {
    pub fn new(job_id: &str, job_name: &str, factfile_path: &str, factfile_args: Vec<String>) -> JobRequest {
        JobRequest {
            job_id: job_id.to_owned(),
            job_name: job_name.to_owned(),
            factfile_path: factfile_path.to_owned(),
            factfile_args: factfile_args,
        }
    }

    pub fn validate<U: Execution>(request: JobRequest, command_store: &U) -> Result<JobRequest, ValidationError> {
        // check job name not empty
        // check factfile path not empty
        // check factfile args not empty
        if request.job_name == "" {
            let message = "No valid value found: field 'jobName' cannot be empty".to_string();
            error!("{}", message);
            return Err(ValidationError::no_output(message))
        } else if request.factfile_path == "" {
            let message = "No valid value found: field 'factfilePath' cannot be empty".to_string();
            error!("{}", message);
            return Err(ValidationError::no_output(message))
        }
        // check valid factfile path exists
        if !Path::new(&request.factfile_path).exists() {
            let message = format!("Value does not exist on host for 'factfilePath':'{}'", request.factfile_path);
            error!("{}", message);
            return Err(ValidationError::no_output(message))
        }
        // attempt dry run
        let cmd_path = try!(command_store.get_command(::FACTOTUM));
        let mut cmd_args = vec!["run".to_string(), request.factfile_path.clone(), "--dry-run".to_string()];
        cmd_args.extend_from_slice(request.factfile_args.as_slice());
        match command_store.execute(cmd_path, cmd_args) {
            Ok(_) => {
                debug!("Dry run success");
            },
            Err(e) => {
                error!("{}", e);
                return Err(ValidationError::no_output(e))
            }
        }
        // generate unique job id
        let mut request = request;
        let tags = match extract_tags(&request.factfile_args) {
            Ok(extracted) => extracted,
            Err(e) => {
                error!("{}", e);
                return Err(ValidationError::no_output(e))
            }
        };
        request.job_id = match generate_id(&request.factfile_path, tags) {
            Ok(id) => id,
            Err(e) => {
                error!("{}", e);
                return Err(ValidationError::no_output(e))
            }
        };
        Ok(request)
    }

    pub fn append_job_args(server: &ServerManager, job: &mut JobRequest) {
        if server.webhook_uri != "" {
            job.factfile_args.push("--webhook".to_string());
            job.factfile_args.push(server.webhook_uri.clone());
        }
        if server.no_colour {
            job.factfile_args.push("--no-colour".to_string());
        }
    }
}

impl PartialEq for JobRequest {
    fn eq(&self, other: &JobRequest) -> bool {
        self.job_id        == other.job_id &&
        self.job_name      == other.job_name &&
        self.factfile_path == other.factfile_path &&
        self.factfile_args == other.factfile_args
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsRequest {
    pub state: String
}

impl PartialEq for SettingsRequest {
    fn eq(&self, other: &SettingsRequest) -> bool {
        self.state == other.state
    }
}

impl SettingsRequest {
    pub fn new(state: &str) -> SettingsRequest {
        SettingsRequest {
            state: state.to_owned()
        }
    }

    pub fn validate(request: SettingsRequest) -> Result<SettingsRequest, ValidationError> {
        match request.state.as_ref() {
            ::SERVER_STATE_RUN | ::SERVER_STATE_DRAIN => Ok(request),
            _ => Err(ValidationError::no_output(format!("Invalid 'state', must be one of ({}|{})", ::SERVER_STATE_RUN, ::SERVER_STATE_DRAIN)))
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ValidationError {
    pub error: String,
    pub stdout: String,
    pub stderr: String
}

impl ValidationError {
    pub fn new(error: String, stdout: String, stderr: String) -> ValidationError {
        ValidationError {
            error: error,
            stdout: stdout,
            stderr: stderr,
        }
    }

    pub fn no_output(error: String) -> ValidationError {
        ValidationError::new(error, String::new(), String::new())
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Validation Error: {}", self.error)
    }
}

impl error::Error for ValidationError {
    fn description(&self) -> &str {
        &self.error
    }
}

impl From<String> for ValidationError {
    fn from(err: String) -> ValidationError {
        ValidationError::no_output(err)
    }
}

fn generate_id(factfile: &str, tags: Option<HashMap<String, String>>) -> Result<String, String> {
    let mut fh = try!(File::open(factfile)
        .map_err(|e| format!("Could not open '{}' for reading: {}", factfile, e)));
    let mut file = String::new();
    try!(fh.read_to_string(&mut file).map_err(|e| format!("Could not read '{}': {}", factfile, e)));
    let schema: serde_json::Value = try!(serde_json::from_str(&file).map_err(|e| e.to_string()));
    let ff: String = try!(serde_json::to_string(&schema).map_err(|e| e.to_string()));
    let mut job_digest = Sha256::new();
    job_digest.input_str(&ff);

    if let Some(ref tags_map) = tags {
        let mut sorted_keys:Vec<_> = tags_map.keys().collect();
        sorted_keys.sort();
        for key in sorted_keys {
            job_digest.input_str(key);
            job_digest.input_str(&tags_map[key]);
        }
    }

    Ok(job_digest.result_str())
}

fn extract_tags(factfile_args: &Vec<String>) -> Result<Option<HashMap<String, String>>, String> {
    let mut opts = Options::new();
    opts.optmulti("", "tag", "Add Factotum job metadata (tags).", "TAG");
    opts.optmulti("", "constraint", "Checks for an external constraint that will prevent execution; allowed constraints (host).", "CONSTRAINT");
    opts.optopt("", "start", "Begin at specified task.", "TASK");
    opts.optopt("", "env", "Supply JSON to define mustache variables in Factfile.", "ENV");
    opts.optopt("", "webhook", "Post updates on job execution to the specified URL.", "URL");
    opts.optopt("", "output", "File to print output to. Used with `dot`.", "FILE");
    opts.optflag("", "dry-run", "Pretend to execute a Factfile, showing the commands that would be executed. Can be used with other options.");
    opts.optflag("", "no-colour", "Turn off ANSI terminal colours/formatting in output.");
    opts.optflag("", "overwrite", "Overwrite the output file if it exists.");

    let matches = match opts.parse(factfile_args) {
        Ok(m) => m,
        Err(e) => {
            return Err(format!("Error parsing tags in factfile args: {:?}", e))
        },
    };

    if matches.opt_present("tag") {
        let tag_map = get_tag_map(&matches.opt_strs("tag"));
        trace!("Extracted tags: {:?}", tag_map);
        Ok(Some(tag_map))
    } else {
        trace!("NO TAGS FOUND");
        Ok(None)
    }
}

fn get_tag_map(args: &Vec<String>) -> HashMap<String, String> {
    let mut arg_map: HashMap<String, String> = HashMap::new();

    for arg in args.iter() {
        let split = arg.split(",").collect::<Vec<&str>>();
        if split.len() >= 2 && split[0].trim().is_empty() == false {
            let key = split[0].trim().to_string();
            let value = split[1..].join("").trim().to_string();
            arg_map.insert(key, value);
        } else if split.len() == 1 && split[0].trim().is_empty() == false {
            let key = split[0].trim().to_string();
            let value = "".to_string();
            arg_map.insert(key, value);
        }
    }

    arg_map
}
