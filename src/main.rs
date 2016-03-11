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
 
#[macro_use]
extern crate log;
extern crate log4rs;
extern crate getopts;
extern crate daggy;
extern crate rustc_serialize;
extern crate valico;
extern crate time;

use getopts::Options;
use std::env;
use std::fs;


mod factotum;

use factotum::runner::ExecutionResult;

fn print_usage(program:&str, opts:Options) {
    let brief = format!("Usage: {} FILE", program);
    print!("{}", opts.usage(&brief))
}

fn parse_file(factfile:&str) -> i32 {
    match factotum::fileparser::parse(factfile) {
        Ok(job) => {
            match factotum::runner::execute_factfile(&job) {
                ExecutionResult::AllTasksComplete(_) => 0,
                ExecutionResult::EarlyFinishOk(_) => 0, 
                ExecutionResult::AbnormalTermination(res) => {
                    let incomplete_tasks = res.iter()
                                              .filter(|r| !r.run)
                                              .map(|r| r.name.clone())
                                              .collect::<Vec<String>>()
                                              .join(", ");
                    println!("\nFactotum job executed abnormally - the following tasks were not completed: {}!", incomplete_tasks);
                    2
                }
            }
        }, 
        Err(msg) => {
            println!("{}", msg);
            1
        }      
    }
}

fn get_log_config() -> Result<log4rs::config::Config, log4rs::config::Errors> {
    let file_appender = log4rs::appender::FileAppender::builder(".factotum/factotum.log").build();
    let root = log4rs::config::Root::builder(log::LogLevelFilter::Info)
        .appender("file".to_string());  
    
    log4rs::config::Config::builder(root.build())
        .appender(log4rs::config::Appender::builder("file".to_string(), Box::new(file_appender.unwrap())).build()).build()    
}

fn init_logger() {
    fs::create_dir(".factotum").ok();   
    let log_config = get_log_config();
    log4rs::init_config(log_config.unwrap()).unwrap();
}

fn main() {
    std::process::exit(factotum())
}

fn factotum() -> i32 {    
    init_logger();

    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optflag("h","help", "print out this help menu");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string())
    };

    match (matches.opt_present("h"), !matches.free.is_empty()) { // todo --no-colour ? decide on colour outputs. And verbosity 
        (false, true) => { return parse_file(&matches.free[0].clone()); },
        (_, _)  => { print_usage(&program, opts); return 1 }
    };
}

#[test]
fn have_valid_config() {
    fs::create_dir(".factotum").ok(); 
    if let Err(errs) = get_log_config() {
        panic!("config not building correctly! {:?}", errs);
    }       
}