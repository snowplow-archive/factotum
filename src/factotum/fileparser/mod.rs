/* 
 * Copyright (c) 2016 Snowplow Analytics Ltd. All rights reserved.
 *
 * This program is licensed to you under the Apache License Version 2.0, and
 * you may not use this file except in compliance with the Apache License
 * Version 2.0.  You may obtain a copy of the Apache License Version 2.0 at
 * http://www.apache.org/licenses/LICENSE-2.0.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the Apache License Version 2.0 is distributed on an "AS
 * IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or
 * implied.  See the Apache License Version 2.0 for the specific language
 * governing permissions and limitations there under.
 */
 
#[cfg(test)]
mod tests;

use std::io::prelude::*;
use std::fs::File;
use rustc_serialize::json::{self, Json, error_str};
use rustc_serialize::json::ParserError::{self, SyntaxError, IoError};
use super::factfile;
use valico::json_schema;
use valico::common::error::*;
use std::error::Error;

extern crate mustache;

pub fn parse(factfile:&str, env:Option<String>) -> Result<factfile::Factfile, String> {
    info!("reading {} into memory", factfile);
    let mut fh = try!(File::open(&factfile).map_err(|e| format!("Couldn't open '{}' for reading: {}", factfile, e)));
    let mut f = String::new();
    try!(fh.read_to_string(&mut f).map_err(|e| format!("Couldn't read '{}': {}", factfile, e))); 
    info!("file {} was read successfully!", factfile);
    
    parse_str(&f, factfile, env)
}

pub fn inflate_env(env:&str) -> Result<Json,String> {    
    Json::from_str(env).map_err(|err| format!("Supplied environment/config '{}' is not valid JSON: {}", env, Error::description(&err)))
}

// there must be a way to do this normally
fn get_human_readable_parse_error(e:ParserError) -> String {
    match e {
        SyntaxError(code, line, col) => format!("{} at line {}, column {}", error_str(code), line, col),
        IoError(msg) => unreachable!("Unexpected IO error: {}", msg) 
    }
}

fn parse_str(json:&str, from_filename:&str, env:Option<String>) -> Result<factfile::Factfile, String> {
    info!("parsing json:\n{}", json);
    let factotum_schema_str: &'static str = include_str!("./jsonschemas/factotum.json");
    let factotum_schema = if let Ok(fs) = Json::from_str(factotum_schema_str) {
        fs
    } else {
        unreachable!("The JSON schema inside factotum is not valid json");
    };

    let mut scope = json_schema::Scope::new();
    let schema = match scope.compile_and_return(factotum_schema.clone(),false) {
        Ok(s) => s,
        Err(msg) => { unreachable!("The JSON schema inside factotum could not be built! {:?}", msg) } 
    };

    let json_tree = try!(Json::from_str(json).map_err(|e| format!("The factfile '{}' is not valid JSON: {}", from_filename, get_human_readable_parse_error(e))));
    info!("'{}' is valid JSON!", from_filename);
    let json_schema_validation = schema.validate(&json_tree);
    
    if json_schema_validation.is_valid() {
        info!("'{}' matches the factotum schema definition!", from_filename);
        
        let conf = if let Some(c) = env {
            info!("inflating config:\n{}", c);
            Some(try!(inflate_env(&c)))
        } else {
            info!("no config specified!");  
            None          
        };

        parse_valid_json(json, conf).map_err(|msg| format!("'{}' is not a valid factotum factfile: {}", from_filename, msg))
    } else {
        let errors_str = json_schema_validation.errors.iter()
                                        .map(|e| format!("'{}' - {}{}", e.get_path(),
                                                                        e.get_title(),
                                                                        match  e.get_detail() { Some(str) => format!(" ({})", str), _ => "".to_string() } ))
                                        .collect::<Vec<String>>()
                                        .join("\n");
        Err(format!("'{}' is not a valid factotum factfile: {}", from_filename, errors_str))
    }
}


#[derive(RustcDecodable)]
#[allow(dead_code)]
struct SelfDescribingJson {
    schema: String,
    data: FactfileFormat
}

#[derive(RustcDecodable)]
struct FactfileFormat {
    name: String,
    tasks: Vec<FactfileTaskFormat>
}

#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct FactfileTaskFormat {
    name: String,
    executor: String,
    command: String,
    arguments: Vec<String>,
    dependsOn: Vec<String>,
    onResult: FactfileTaskResultFormat
}

#[derive(RustcDecodable)]
#[allow(non_snake_case)]
struct FactfileTaskResultFormat {
    terminateJobWithSuccess: Vec<i32>,
    continueJob: Vec<i32>
}

fn mustache_str(template:&str, env:&Json) -> Result<String,String> {
     let compiled_template = mustache::compile_str(&template);
     let mut bytes = vec![];
     try!(compiled_template.render(&mut bytes, &env).map_err(|e| format!("Error rendering template: {}", Error::description(&e))));
     String::from_utf8(bytes).map_err(|e| format!("Error inflating rendered template to utf8: {}", Error::description(&e)))
}

fn parse_valid_json(file:&str, conf:Option<Json>) -> Result<factfile::Factfile, String> {
    let schema: SelfDescribingJson = try!(json::decode(file).map_err(|e| e.to_string())); 
    let decoded_json = schema.data;
    let mut ff = factfile::Factfile::new(decoded_json.name);
    for file_task in decoded_json.tasks.iter() { // TODO errs in here - ? add task should Result not panic! 
        info!("adding task '{}'", file_task.name);
        
        if file_task.onResult.continueJob.len() == 0 {
            return Err(format!("the task '{}' has no way to continue successfully.", file_task.name));
        } else {
           for cont in file_task.onResult.continueJob.iter() {
               if file_task.onResult.terminateJobWithSuccess.iter().any(|conflict| conflict == cont) {
                   return Err(format!("the task '{}' has conflicting actions.", file_task.name));
               }
           }
        }
        
        let mut decorated_args = vec![];            
        if let Some(ref subs) = conf {
            info!("applying variables command and args of '{}'", &file_task.name);
            
            info!("before:\n\tcommand: '{}'\n\targs: '{}'", file_task.command, file_task.arguments.join(" "));
            
            let decorated_command = try!(mustache_str(&file_task.command, &subs));
            

            for arg in file_task.arguments.iter() {
                decorated_args.push( try!(mustache_str(arg, &subs)) )
            }
            
            info!("after:\n\tcommand: '{}'\n\targs: '{}'", decorated_command, decorated_args.join(" "));            
        } 

        let deps:Vec<&str> = file_task.dependsOn.iter().map(AsRef::as_ref).collect();
        let args:Vec<&str> = decorated_args.iter().map(AsRef::as_ref).collect();
                
        ff.add_task(&file_task.name,
                    &deps,
                    &file_task.executor, 
                    &file_task.command,
                    &args,
                    &file_task.onResult.terminateJobWithSuccess,
                    &file_task.onResult.continueJob);
    }
    Ok(ff)
}
