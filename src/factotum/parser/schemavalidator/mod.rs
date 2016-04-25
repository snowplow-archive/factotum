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

use valico::json_schema;
use valico::common::error::*;
use rustc_serialize::json::{self, Json, error_str};

use std::error::Error;
use rustc_serialize::json::ParserError::{self, SyntaxError, IoError};

// there must be a way to do this normally
fn get_human_readable_parse_error(e:ParserError) -> String {
    match e {
        SyntaxError(code, line, col) => format!("{} at line {}, column {}", error_str(code), line, col),
        IoError(msg) => unreachable!("Unexpected IO error: {}", msg) 
    }
}

pub fn validate_schema(json:&str, schema:&str) -> Result<(), String> {
    let mut scope = json_schema::Scope::new();
    let json_schema = try!(Json::from_str(schema).map_err(|e| format!("Schema is invalid json: {:?}", e)));
    let compiled_schema = try!(scope.compile_and_return(json_schema,false).map_err(|e| format!("Failed to compile json schema: {:?}", e)));

    let json_tree = try!(Json::from_str(json).map_err(|e| format!("invalid JSON - {}", get_human_readable_parse_error(e))));
    info!("'{}' is valid JSON!", json);
    let json_schema_validation = compiled_schema.validate(&json_tree);    
    
    if json_schema_validation.is_valid() == true {
        Ok(())
    } else {
        let errors_str = json_schema_validation.errors.iter()
                                        .map(|e| format!("'{}' - {}{}", e.get_path(),
                                                                        e.get_title(),
                                                                        match  e.get_detail() { Some(str) => format!(" ({})", str), _ => "".to_string() } ))
                                        .collect::<Vec<String>>()
                                        .join("\n");
        Err(format!("{}",errors_str)) 
    } 
}

pub fn validate_against_factfile_schema(json:&str) -> Result<(), String> {
    let factotum_schema_str: &'static str = include_str!("jsonschemas/factotum.json");
     
    validate_schema(json, factotum_schema_str)
}

/*
//.map_err(|e| format!("The factfile '{}' is not valid JSON: {}", from_filename, get_human_readable_parse_error(e))));

 put as test
    let factotum_schema = if let Ok(fs) = Json::from_str(factotum_schema_str) {
        fs
    } else {
        unreachable!("The JSON schema inside factotum is not valid json");
    };
    */