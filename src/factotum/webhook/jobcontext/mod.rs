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

use chrono::DateTime;
use chrono::UTC;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use uuid::Uuid;
use rustc_serialize::base64::{ToBase64, MIME};

#[derive(Clone, Debug)]
pub struct JobContext {
    pub job_name: String,
    pub job_reference: String,
    pub run_reference: String,
    pub factfile: String,
    pub factotum_version: String,
    pub start_time: DateTime<UTC>
}

impl JobContext {

    pub fn new<S: Into<String>>(job_name:S, factfile: &str) -> Self {
        let ff = factfile;
        let mut job_digest = Sha256::new();
        job_digest.input_str(&ff);
        let job_ref = job_digest.result_str();

        let mut run_digest = Sha256::new();
        run_digest.input_str(&format!("{}",Uuid::new_v4()));
        let run_ref = run_digest.result_str();

        let mut config = MIME;
        config.line_length = None;
        let b64_ff = ff.as_bytes().to_base64(config);

        JobContext {
            job_name: job_name.into(),
            job_reference: job_ref,
            run_reference: run_ref,
            factfile: b64_ff,
            factotum_version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: UTC::now()
        }
    }

}