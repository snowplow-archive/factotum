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
mod templater;
pub mod schemavalidator;

use std::io::prelude::*;
use std::fs::File;
use rustc_serialize::json::{self, Json};
use super::factfile;

use std::error::Error;

pub struct TaskReturnCodeMapping {
    pub continue_job: Vec<i32>,
    pub terminate_early: Vec<i32>,
}

pub enum OverrideResultMappings {
    All(TaskReturnCodeMapping),
    None,
}

pub fn parse(factfile: &str,
             env: Option<String>,
             overrides: OverrideResultMappings)
             -> Result<factfile::Factfile, String> {
    info!("reading {} into memory", factfile);
    let mut fh = try!(File::open(&factfile)
        .map_err(|e| format!("Couldn't open '{}' for reading: {}", factfile, e)));
    let mut f = String::new();
    try!(fh.read_to_string(&mut f).map_err(|e| format!("Couldn't read '{}': {}", factfile, e)));
    info!("file {} was read successfully!", factfile);

    parse_str(&f, factfile, env, overrides)
}

pub fn inflate_env(env: &str) -> Result<Json, String> {
    Json::from_str(env).map_err(|err| {
        format!("Supplied environment/config '{}' is not valid JSON: {}",
                env,
                Error::description(&err))
    })
}

fn parse_str(json: &str,
             from_filename: &str,
             env: Option<String>,
             overrides: OverrideResultMappings)
             -> Result<factfile::Factfile, String> {
    info!("parsing json:\n{}", json);

    let validation_result = schemavalidator::validate_against_factfile_schema(json);

    match validation_result {        
        Ok(_) => {
            info!("'{}' matches the factotum schema definition!",
                  from_filename);

            let conf = if let Some(c) = env {
                info!("inflating config:\n{}", c);
                Some(try!(inflate_env(&c)))
            } else {
                info!("no config specified!");
                None
            };

            parse_valid_json(json, conf, overrides).map_err(|msg| {
                format!("'{}' is not a valid factotum factfile: {}",
                        from_filename,
                        msg)
            })
        }
        Err(msg) => {
            info!("'{}' failed to match factfile schema definition!",
                  from_filename);
            Err(format!("'{}' is not a valid factotum factfile: {}",
                        from_filename,
                        msg))
        }
    }
}


#[derive(RustcDecodable)]
#[allow(dead_code)]
struct SelfDescribingJson {
    schema: String,
    data: FactfileFormat,
}

#[derive(RustcDecodable)]
struct FactfileFormat {
    name: String,
    tasks: Vec<FactfileTaskFormat>,
}

#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct FactfileTaskFormat {
    name: String,
    executor: String,
    command: String,
    arguments: Vec<String>,
    dependsOn: Vec<String>,
    onResult: FactfileTaskResultFormat,
}

#[derive(RustcDecodable, Clone)]
#[allow(non_snake_case)]
struct FactfileTaskResultFormat {
    terminateJobWithSuccess: Vec<i32>,
    continueJob: Vec<i32>,
}

fn parse_valid_json(file: &str,
                    conf: Option<Json>,
                    overrides: OverrideResultMappings)
                    -> Result<factfile::Factfile, String> {
    let schema: SelfDescribingJson = try!(json::decode(file).map_err(|e| e.to_string()));
    let decoded_json = schema.data;
    let mut ff = factfile::Factfile::new(file, &decoded_json.name);

    for file_task in decoded_json.tasks.iter() {
        // TODO errs in here - ? add task should Result not panic!
        info!("adding task '{}'", file_task.name);

        if file_task.onResult.continueJob.len() == 0 {
            return Err(format!("the task '{}' has no way to continue successfully.",
                               file_task.name));
        } else {
            for cont in file_task.onResult.continueJob.iter() {
                if file_task.onResult
                    .terminateJobWithSuccess
                    .iter()
                    .any(|conflict| conflict == cont) {
                    return Err(format!("the task '{}' has conflicting actions.", file_task.name));
                }
            }
        }

        let mut decorated_args = vec![];
        if let Some(ref subs) = conf {
            info!("applying variables command and args of '{}'",
                  &file_task.name);

            info!("before:\n\tcommand: '{}'\n\targs: '{}'",
                  file_task.command,
                  file_task.arguments.join(" "));

            let decorated_command = try!(templater::decorate_str(&file_task.command, &subs));

            for arg in file_task.arguments.iter() {
                decorated_args.push(try!(templater::decorate_str(arg, &subs)))
            }

            info!("after:\n\tcommand: '{}'\n\targs: '{}'",
                  decorated_command,
                  decorated_args.join(" "));
        } else {
            info!("No config specified, writing args as undecorated strings");
            for arg in file_task.arguments.iter() {
                decorated_args.push(arg.to_string());
            }
        }

        let deps: Vec<&str> = file_task.dependsOn.iter().map(AsRef::as_ref).collect();
        let args: Vec<&str> = decorated_args.iter().map(AsRef::as_ref).collect();

        let (terminate_mappings, continue_mappings) = match overrides {
            OverrideResultMappings::All(ref with_value) => {
                (&with_value.terminate_early, &with_value.continue_job)
            }
            OverrideResultMappings::None => {
                (&file_task.onResult.terminateJobWithSuccess, &file_task.onResult.continueJob)
            }
        };

        ff.add_task(&file_task.name,
                    &deps,
                    &file_task.executor,
                    &file_task.command,
                    &args,
                    terminate_mappings,
                    continue_mappings);
    }
    Ok(ff)
}
