// Copyright (c) 2016-2019 Snowplow Analytics Ltd. All rights reserved.
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


#[derive(RustcEncodable, RustcDecodable)]
#[allow(dead_code)]
struct SelfDescribingJson {
    schema: String,
    data: FactfileFormat,
}

#[derive(RustcEncodable, RustcDecodable)]
struct FactfileFormat {
    name: String,
    tasks: Vec<FactfileTaskFormat>,
}

#[derive(RustcEncodable, RustcDecodable)]
#[allow(non_snake_case)]
struct FactfileTaskFormat {
    name: String,
    executor: String,
    command: String,
    arguments: Vec<String>,
    dependsOn: Vec<String>,
    onResult: FactfileTaskResultFormat,
}

#[derive(RustcEncodable, RustcDecodable, Clone)]
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
    let compact_json:String = try!(json::encode(&schema).map_err(|e| e.to_string()));
    let decoded_json = schema.data;
    let mut ff = factfile::Factfile::new(compact_json, decoded_json.name.clone());

    for file_task in decoded_json.tasks.iter() {
        let final_name = if let Some(ref subs) = conf {
            try!(templater::decorate_str(&file_task.name, &subs))
        } else {
            file_task.name.clone()
        }.to_string();

        // TODO errs in here - ? add task should Result not panic!
        info!("adding task '{}'", final_name);

        if file_task.onResult.continueJob.len() == 0 {
            return Err(format!("the task '{}' has no way to continue successfully.",
                               final_name));
        } else {
            for cont in file_task.onResult.continueJob.iter() {
                if file_task.onResult
                    .terminateJobWithSuccess
                    .iter()
                    .any(|conflict| conflict == cont) {
                    return Err(format!("the task '{}' has conflicting actions.", final_name));
                }
            }
        }

        let mut decorated_args = vec![];
        let mut decorated_deps = vec![];
        if let Some(ref subs) = conf {
            info!("applying variables command and args of '{}'",
                  &final_name);

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

            for dep in file_task.dependsOn.iter() {
                decorated_deps.push(try!(templater::decorate_str(dep, &subs)))
            }

            info!("after:\n\tcommand: '{}'\n\tdeps: '{}'",
                  decorated_command,
                  decorated_deps.join(" "));
        } else {
            info!("No config specified, writing args & deps as undecorated strings");
            for arg in file_task.arguments.iter() {
                decorated_args.push(arg.to_string());
            }
            for dep in file_task.dependsOn.iter() {
                decorated_deps.push(dep.to_string());
            }
        }

        let deps: Vec<&str> = decorated_deps.iter().map(AsRef::as_ref).collect();
        let args: Vec<&str> = decorated_args.iter().map(AsRef::as_ref).collect();

        let (terminate_mappings, continue_mappings) = match overrides {
            OverrideResultMappings::All(ref with_value) => {
                (&with_value.terminate_early, &with_value.continue_job)
            }
            OverrideResultMappings::None => {
                (&file_task.onResult.terminateJobWithSuccess, &file_task.onResult.continueJob)
            }
        };

        ff.add_task(&final_name,
                    &deps,
                    &file_task.executor,
                    &file_task.command,
                    &args,
                    terminate_mappings,
                    continue_mappings);
    }
    Ok(ff)
}
