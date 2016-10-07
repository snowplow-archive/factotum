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

use super::*;

use chrono::UTC;
use chrono::DateTime;
use rustc_serialize::base64::{ToBase64, MIME};
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use std::collections::HashMap;

#[test]
fn new_sets_name() {
    let context = JobContext::new("hello", "some_json");
    assert_eq!(context.job_name, "hello");
}

#[test]
fn new_sets_version_and_date() {
    let context = JobContext::new("hello", "some_json");
    assert_eq!(context.factotum_version, env!("CARGO_PKG_VERSION").to_string());
    assert_eq!(context.start_time.date(), UTC::now().date());
}

#[test]
fn new_sets_job_ref_to_hash_of_factfile() {
    let job_name = "hello";
    let factfile_sim = "{Blabla}";
    let mut digest = Sha256::new();
    digest.input_str(factfile_sim);
    let expected = digest.result_str();

    let context = JobContext::new(job_name, factfile_sim);

    assert_eq!(context.job_reference, expected);
}

#[test]
fn new_sets_run_ref_to_random_hash() {
    let job_name = "hello";
    let factfile_sim = "{stuff}";

    let mut map = HashMap::new();  

    for _ in 1..1000 {
        let context = JobContext::new(job_name, factfile_sim);
        if let Some(_) = map.get(&context.run_reference) {
            panic!("Dup run ref generated: {}", context.run_reference);
        }
        map.insert(context.run_reference, ());
    }
}

#[test]
fn factfile_is_b64_coded() {
    let job_name = "hello";
    let factfile_sim = include_str!("./tests.rs");
    let jc = JobContext::new(job_name, factfile_sim);
    let mut config = MIME;
    config.line_length = None;
    assert_eq!(factfile_sim.as_bytes().to_base64(config), jc.factfile);
}