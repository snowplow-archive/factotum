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
extern crate docopt;
extern crate daggy;
extern crate rustc_serialize;
extern crate valico;
extern crate colored;
extern crate chrono;

use docopt::Docopt;
use std::fs;
use factotum::executor::ExecutionResult;
use factotum::executor::TaskExecutionResult;
use factotum::factfile::Factfile;
use colored::*;
use std::time::Duration;

mod factotum;

const PROC_SUCCESS: i32 = 0;
const PROC_PARSE_ERROR: i32 = 1;
const PROC_EXEC_ERROR: i32 = 2;
const PROC_OTHER_ERROR: i32 = 3;

const USAGE: &'static str = "
Factotum.

Usage:
  factotum run <factfile> [--start=<start_task>] [--env=<env>]
  factotum validate <factfile>
  factotum (-h | --help)

Options:
  -h --help             Show this screen.
  --start=<start_task>  Begin at specified task.
  --env=<env>           Supply JSON to define mustache variables in Factfile.
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_start: Option<String>,
    flag_env: Option<String>,
    arg_factfile: String,
    cmd_run: bool,
    cmd_validate: bool
}

// macro to simplify printing to stderr
// https://github.com/rust-lang/rfcs/issues/1078
macro_rules! print_err {
    ($($arg:tt)*) => (
        {
            use std::io::prelude::*;
            if let Err(e) = write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*)) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", format!($($arg)*), e);
            }
        }
    )
}

fn get_duration_as_string(d:&Duration) -> String {
    // duration doesn't support the normal display format
    // for now lets put together something that produces some sensible output
    // e.g.
    // if it's under a minute, show the number of seconds and nanos
    // if it's under an hour, show the number of minutes, and seconds
    // if it's over an hour, show the number of hours, minutes and seconds
    const NANOS_ONE_SEC: f64 = 1000000000_f64;
    const SECONDS_ONE_HOUR: u64 = 3600;
    
    if d.as_secs() < 60 {
        let mut seconds: f64 = d.as_secs() as f64;    
        seconds += d.subsec_nanos() as f64 / NANOS_ONE_SEC; 
        format!("{:.1}s", seconds)
    } else if d.as_secs() >= 60 && d.as_secs() < SECONDS_ONE_HOUR {
        // ignore nanos here..
        let secs = d.as_secs() % 60;
        let minutes = (d.as_secs() / 60) % 60;  
        format!("{}m, {}s", minutes, secs)
    } else {
        let secs = d.as_secs() % 60;
        let minutes = (d.as_secs() / 60) % 60;  
        let hours = d.as_secs() / SECONDS_ONE_HOUR;
        format!("{}h, {}m, {}s", hours, minutes, secs)
    }
}

fn get_task_result_line_str(task_result:&TaskExecutionResult) -> (String, Option<String>) {
        
    let (opening_line, stdout, stderr, summary_line) = if let Some(ref run_result) = task_result.run_details {
        // we know tasks with run details were attempted
          
         let opener = format!("Task '{}' was started at {}\n", task_result.name.cyan(), run_result.run_started);
         
         let output = match run_result.stdout {
             Some(ref o) => Some(format!("Task '{}' stdout:\n{}\n", task_result.name.cyan(), o.trim_right().bold())), 
             None => None
         };
         
         let errors = match run_result.stderr {
             Some(ref e) => Some(format!("Task '{}' stderr:\n{}\n", task_result.name.cyan(), e.trim_right().red())),
             None => None
         };
         
         let summary = if let Some(ref run_error) = run_result.task_execution_error {
            let mut failure_str = "Task '".red().to_string();
            failure_str.push_str(&format!("{}", task_result.name.cyan()));
            failure_str.push_str(&format!("': failed after {}. Reason: {}", get_duration_as_string(&run_result.duration), run_error).red().to_string());
            failure_str
         } else {
            let mut success_str = "Task '".green().to_string();
            success_str.push_str(&format!("{}", task_result.name.cyan()));
            success_str.push_str(&format!("': succeeded after {}", get_duration_as_string(&run_result.duration)).green().to_string());
            success_str
         };
         
         (opener, output, errors, summary)
         
    } else {
        // tasks without run details may have been unable to start (some internal error)
        // or skipped because a prior task errored or NOOPed
        
       let reason_for_not_running = if task_result.attempted {
           "Factotum could not start the task".red().to_string()          
       } else {
           "skipped".to_string()
       };
        
       let opener = format!("Task '{}': {}!\n", task_result.name.cyan(), reason_for_not_running);
       (opener, None, None, String::from(""))
    };
    
    let mut result = opening_line;
    if let Some(o) = stdout {
        result.push_str(&o);
    }

    if summary_line.len() > 0 {
        result.push_str(&format!("{}\n", summary_line));
    }
    
    return (result, stderr);
}

fn get_task_results_str(task_results:&Vec<TaskExecutionResult>) -> (String, String) {
    let mut stderr = String::new();
    let mut stdout = String::new(); 
    
    let mut total_run_time = Duration::new(0,0);
    let mut executed = 0;
    
    for task in task_results.iter() {
         let (task_stdout, task_stderr) = get_task_result_line_str(task);
         stdout.push_str(&task_stdout);
         
         if let Some(task_stderr_str) = task_stderr {
             stderr.push_str(&task_stderr_str);
         }
         
         if let Some(ref run_result) = task.run_details {
             total_run_time = total_run_time + run_result.duration;
             executed += 1;
         }
    }
    
    let summary = format!("{}/{} tasks run in {}\n", executed, task_results.len(), get_duration_as_string(&total_run_time));
    stdout.push_str(&summary.green().to_string());
    
    (stdout,stderr)
}

fn validate_start_task(job: &Factfile, start_task:&str) -> Result<(), &'static str> {
        /*
               A
              / \
             B   C
            / \ /
           D   E 
           
           We cannot start at B because E depends on C, which depends on A (simiar for C)    
           
               A 
              / \
             B   C  
            / \   \ 
           D   E   F 
           
           It's fine to start at B here though, causing B, D, and E to be run 
        */        
        
        match job.can_job_run_from_task(start_task) {
            Ok(is_good) => if is_good { Ok(()) } else { Err("the job cannot be started here without triggering prior tasks") },
            Err(msg) => Err(msg)
        }   
}

fn validate(factfile:&str, env:Option<String>) -> Result<String, String> {
    match factotum::parser::parse(factfile, env) {
        Ok(_) => Ok(format!("'{}' is a valid Factfile!", factfile).green().to_string()),
        Err(msg) => Err(msg.red().to_string())
    }
}

fn parse_file_and_execute(factfile:&str, env:Option<String>, start_from:Option<String>) -> i32 {
    match factotum::parser::parse(factfile, env) {
        Ok(job) => {
            
            if let Some(ref start_task) = start_from {
                if let Err(msg) = validate_start_task(&job, &start_task) {
                    warn!("The job could not be started from '{}' because {}", start_task, msg);
                    println!("The job cannot be started from '{}' because {}", start_task.cyan(), msg);
                    return PROC_OTHER_ERROR;
                }
            }
            
            match factotum::executor::execute_factfile(&job, start_from) {
                ExecutionResult::AllTasksComplete(tasks) => { 
                    let (stdout_summary, stderr_summary) = get_task_results_str(&tasks);
                    print!("{}", stdout_summary);
                    if !stderr_summary.trim_right().is_empty() {
                        print_err!("{}", stderr_summary.trim_right()); 
                    }
                    PROC_SUCCESS 
                    },
                ExecutionResult::EarlyFinishOk(tasks) => { 
                    let (stdout_summary, stderr_summary) = get_task_results_str(&tasks);
                    print!("{}", stdout_summary); 
                    if !stderr_summary.trim_right().is_empty() {
                        print_err!("{}", stderr_summary.trim_right());
                    }
                    let incomplete_tasks = tasks.iter()
                                              .filter(|r| !r.attempted)
                                              .map(|r| format!("'{}'", r.name.cyan())) 
                                              .collect::<Vec<String>>()
                                              .join(", ");
                    let stop_requesters = tasks.iter()
                                            .filter(|r| r.run_details.is_some() && r.run_details.as_ref().unwrap().requests_job_termination)
                                            .map(|r| format!("'{}'", r.name.cyan()))
                                            .collect::<Vec<String>>()
                                            .join(", ");                                 
                    println!("Factotum job finished early as a task ({}) requested an early finish. The following tasks were not run: {}.", stop_requesters, incomplete_tasks);
                    PROC_SUCCESS 
                    }, 
                ExecutionResult::AbnormalTermination(tasks) => {
                    let (stdout_summary, stderr_summary) = get_task_results_str(&tasks);
                    print!("{}", stdout_summary); 
                    if !stderr_summary.trim_right().is_empty() {
                        print_err!("{}", stderr_summary.trim_right());
                    }
                    let incomplete_tasks = tasks.iter()
                                              .filter(|r| !r.attempted)
                                              .map(|r| format!("'{}'", r.name.cyan()))
                                              .collect::<Vec<String>>()
                                              .join(", ");
                    let failed_tasks = tasks.iter()
                                            .filter(|r| r.run_details.is_some() && r.run_details.as_ref().unwrap().task_execution_error.is_some())
                                            .map(|r| format!("'{}'", r.name.cyan()))
                                            .collect::<Vec<String>>()
                                            .join(", ");   
                    println!("Factotum job executed abnormally as a task ({}) failed - the following tasks were not run: {}!", failed_tasks, incomplete_tasks);
                    return PROC_EXEC_ERROR;
                }
            }
        }, 
        Err(msg) => {
            println!("{}", msg);
            return PROC_PARSE_ERROR;
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

    let args: Args = match Docopt::new(USAGE).and_then(|d| d.decode()) {
                        Ok(a) => a,
                        Err(e) => { 
                            print!("{}", e);
                            return PROC_OTHER_ERROR  
                        }
                    };

    if args.cmd_run {
        parse_file_and_execute(&args.arg_factfile, args.flag_env, args.flag_start)
    } else if args.cmd_validate {
        match validate(&args.arg_factfile, args.flag_env) {
            Ok(msg) => {
                println!("{}",msg);
                PROC_SUCCESS
            },
            Err(msg) => {
                println!("{}",msg);
                PROC_PARSE_ERROR
            }
        }
    } else {
        unreachable!("Unknown subcommand!")
    }
}

#[test]
fn validate_ok_factfile_good() {
    let test_file_path = "./tests/resources/example_ok.factotum";
    let is_valid = validate(test_file_path, None);
    let expected:String = format!("'{}' is a valid Factfile!", test_file_path).green().to_string();
    assert_eq!(is_valid, Ok(expected));
}

#[test]
fn validate_ok_factfile_bad() {
    let test_file_path = "./tests/resources/invalid_json.factotum";
    let is_valid = validate(test_file_path, None);
    match is_valid {
        Ok(_) => panic!("Validation returning valid for invalid file"),
        Err(msg) => {
            let expected = format!("'{}' is not a valid",test_file_path); 
            assert_eq!(expected, msg[5..expected.len()+5]) // ignore red colour code
        }
    }
}

#[test]
fn have_valid_config() {
    fs::create_dir(".factotum").ok(); 
    if let Err(errs) = get_log_config() {
        panic!("config not building correctly! {:?}", errs);
    }       
}

#[test]
fn get_duration_under_minute() {
    assert_eq!(get_duration_as_string(&Duration::new(2, 500000099)), "2.5s".to_string());
    assert_eq!(get_duration_as_string(&Duration::new(0, 0)), "0.0s".to_string());
}

#[test]
fn get_duration_under_hour() {
    assert_eq!(get_duration_as_string(&Duration::new(62, 500000099)), "1m, 2s".to_string()); // drop nanos for minute level precision
    assert_eq!(get_duration_as_string(&Duration::new(59*60+59, 0)), "59m, 59s".to_string()); 
}

#[test]
fn get_duration_with_hours() {
    assert_eq!(get_duration_as_string(&Duration::new(3600, 0)), "1h, 0m, 0s".to_string());  
    assert_eq!(get_duration_as_string(&Duration::new(3600*10+63, 0)), "10h, 1m, 3s".to_string()); 
}

#[test]
fn test_get_task_result_line_str() {     
    use chrono::UTC;
    use factotum::executor::RunResult;

    let dt = UTC::now();    
    let sample_task = TaskExecutionResult { 
        name: String::from("hello world"),
        attempted: true,
        run_details: Some(RunResult {
           run_started: dt,
           duration: Duration::from_secs(20),
           task_execution_error: None,
           requests_job_termination: false,
           stdout: Some(String::from("hello world")),
           stderr: None,
           return_code: 0
           })
        };
        
    let expected = format!("Task '{}' was started at {}\nTask '{}' stdout:\n{}\n{}{}{}\n", "hello world".cyan(), dt, "hello world".cyan(), "hello world".bold(), "Task '".green(), "hello world".cyan(), "': succeeded after 20.0s".green());
    let (result_stdout, result_stderr) = get_task_result_line_str(&sample_task);
    assert_eq!(result_stdout, expected);
    assert_eq!(result_stderr, None);
    
    let sample_task_stdout = TaskExecutionResult { 
        name: String::from("hello world"),
        attempted: true,
        run_details: Some(RunResult {
           run_started: dt,
           duration: Duration::from_secs(20),
           task_execution_error: None,
           requests_job_termination: false,
           stdout: Some(String::from("hello world")),
           stderr: Some(String::from("There's errors")),
           return_code: 0
           })
        };
        
    assert_eq!(format!("Task '{}' stderr:\n{}\n", sample_task.name.cyan(), "There's errors".red()), get_task_result_line_str(&sample_task_stdout).1.unwrap());
    assert_eq!(get_task_result_line_str(&sample_task_stdout).0, expected);
    
    let task_skipped = TaskExecutionResult { 
        name: String::from("skip"),
        attempted: false,
        run_details: None
    };
    
    assert_eq!(format!("Task '{}': skipped!\n", "skip".cyan()), get_task_result_line_str(&task_skipped).0);
    assert_eq!(None, get_task_result_line_str(&task_skipped).1);
    
    let task_init_fail = TaskExecutionResult { 
         name: String::from("init fail"),
         attempted: true,
         run_details: None
    };
    // todo: is there a better error here?
    // I think this specific case is very unlikely as it'd hint at a problem with the rust stdlib
    // it means we've tried to execute a process, but didn't get a return code etc
    assert_eq!(format!("Task '{}': {}!\n", "init fail".cyan(), "Factotum could not start the task".red()), get_task_result_line_str(&task_init_fail).0);
    assert_eq!(None, get_task_result_line_str(&task_init_fail).1);
            
    let task_failure = TaskExecutionResult { 
        name: String::from("fails"),
        attempted: true,
        run_details: Some(RunResult {
           run_started: dt,
           duration: Duration::from_secs(20),
           task_execution_error: Some(String::from("The task exited with something unexpected")),
           requests_job_termination: false,
           stdout: Some(String::from("hello world")),
           stderr: Some(String::from("There's errors")),
           return_code: 0
           })
        };
    
    let expected_failed = format!("Task '{}' was started at {}\nTask '{}' stdout:\n{}\n{}{}{}\n", "fails".cyan(), dt, "fails".cyan(), "hello world".bold(), "Task '".red(), "fails".cyan(), "': failed after 20.0s. Reason: The task exited with something unexpected".red());
    let (stdout_failed, stderr_failed) = get_task_result_line_str(&task_failure);
    assert_eq!(expected_failed, stdout_failed);
    assert_eq!(format!("Task '{}' stderr:\n{}\n", "fails".cyan(), "There's errors".red()), stderr_failed.unwrap());
    
    // todo noop ?
}

#[test]
fn test_get_task_results_str_summary() {
    use chrono::UTC;
    use factotum::executor::RunResult;

    let dt = UTC::now();
    
    let mut tasks = vec![];
    let (stdout, stderr) = get_task_results_str(&tasks);
    let expected:String = format!("{}", "0/0 tasks run in 0.0s\n".green());
    
    assert_eq!(stdout, expected);
    assert_eq!(stderr, "");     
    
    tasks.push(TaskExecutionResult { 
        name: String::from("hello world"),
        attempted: true,
        run_details: Some(RunResult {
           run_started: dt,
           duration: Duration::from_secs(20),
           task_execution_error: None,
           requests_job_termination: false,
           stdout: Some(String::from("hello world")),
           stderr: Some(String::from("Mistake")),
           return_code: 0
           })
        });
   
   let (one_task_stdout, one_task_stderr) = get_task_results_str(&tasks);
   let (first_task_stdout, first_task_stderr) = get_task_result_line_str(&tasks[0]);
   let expected_one_task = format!("{}{}", first_task_stdout, "1/1 tasks run in 20.0s\n".green());
   
   assert_eq!(one_task_stdout, expected_one_task);
   let first_task_stderr_str = first_task_stderr.unwrap();
   assert_eq!(one_task_stderr, first_task_stderr_str);
  
    tasks.push(TaskExecutionResult { 
        name: String::from("hello world 2"),
        attempted: true,
        run_details: Some(RunResult {
           run_started: dt,
           duration: Duration::from_secs(80),
           task_execution_error: None,
           requests_job_termination: false,
           stdout: Some(String::from("hello world")),
           stderr: Some(String::from("Mistake")),
           return_code: 0
           })
        });
        
   let (two_task_stdout, two_task_stderr) = get_task_results_str(&tasks);
   let (task_two_stdout, task_two_stderr) = get_task_result_line_str(&tasks[1]);
   let expected_two_task = format!("{}{}{}", first_task_stdout, task_two_stdout, "2/2 tasks run in 1m, 40s\n".green());
   assert_eq!(two_task_stdout, expected_two_task);
   assert_eq!(two_task_stderr, format!("{}{}", first_task_stderr_str, task_two_stderr.unwrap()));
   
}

#[test]
fn test_start_task_validation_not_present() {    
    let mut factfile = Factfile::new("test");    

    match validate_start_task(&factfile, "something") {
        Err(r) => assert_eq!(r, "the task specified could not be found"),
        _ => unreachable!("validation did not fail")
    }
    
    factfile.add_task("something", &vec![], "", "", &vec![], &vec![], &vec![]);
    if let Err(_) = validate_start_task(&factfile, "something") {
        unreachable!("validation failed when task present")
    }
}

#[test]
fn test_start_task_cycles() {
    
    use factotum::factfile::*;
    
    let mut factfile = Factfile::new("test");       
    
    let task_a = Task { name: "a".to_string(),
                        depends_on: vec![],
                        executor: "".to_string(),
                        command: "".to_string(),
                        arguments: vec![],
                        on_result: OnResult { terminate_job: vec![], continue_job: vec![] } };
                        
    let task_b = Task { name: "b".to_string(),
                        depends_on: vec!["a".to_string()],
                        executor: "".to_string(),
                        command: "".to_string(),
                        arguments: vec![],
                        on_result: OnResult { terminate_job: vec![], continue_job: vec![] } };
      
    let task_c = Task { name: "c".to_string(),
                        depends_on: vec!["a".to_string()],
                        executor: "".to_string(),
                        command: "".to_string(),
                        arguments: vec![],
                        on_result: OnResult { terminate_job: vec![], continue_job: vec![] } };        
                        
    let task_d = Task { name: "d".to_string(),
                        depends_on: vec!["c".to_string(),"b".to_string()],
                        executor: "".to_string(),
                        command: "".to_string(),
                        arguments: vec![],
                        on_result: OnResult { terminate_job: vec![], continue_job: vec![] } };        
                    
    factfile.add_task_obj(&task_a);
    factfile.add_task_obj(&task_b);
    factfile.add_task_obj(&task_c);
    factfile.add_task_obj(&task_d);
    
    match validate_start_task(&factfile, "c") {
        Err(r) => assert_eq!(r, "the job cannot be started here without triggering prior tasks"),
        _ => unreachable!("the task validated when it shouldn't have")
    }
}