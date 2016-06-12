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
 
use factotum::factfile::*;
use std::process::Command;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;
use chrono::DateTime;
use chrono::UTC;

enum TaskResult {
    Ok(i32, Duration),
    TerminateJobPlease(i32, Duration),
    Error(Option<i32>, String)
}

pub struct RunResult {
   pub run_started: DateTime<UTC>,
   pub duration: Duration,
   pub requests_job_termination: bool,
   pub task_execution_error: Option<String>, 
   pub stdout: Option<String>,  
   pub stderr: Option<String>,
   pub return_code: i32
}

pub struct TaskExecutionResult {
   pub name: String, 
   pub attempted: bool,
   pub run_details: Option<RunResult>
}

pub enum ExecutionResult {
    AllTasksComplete(Vec<TaskExecutionResult>),
    EarlyFinishOk(Vec<TaskExecutionResult>),
    AbnormalTermination(Vec<TaskExecutionResult>)     
}

#[inline]
fn drain_values(mut map:HashMap<String, TaskExecutionResult>, tasks_in_order:&Vec<Vec<&Task>>) -> Vec<TaskExecutionResult> {
    let mut task_seq:Vec<TaskExecutionResult> = vec![];
    for task_level in tasks_in_order.iter() {
        for task in task_level.iter() {
            match map.remove(&task.name) {
              Some(task_result) => task_seq.push(task_result),
              _ => warn!("A task ({}) does not have an execution result? Skipping", task.name)     
            }            
        }
    }
    task_seq
}

pub fn execute_factfile(factfile:&Factfile, start_from:Option<String>) -> ExecutionResult {
    
    let tasks = if let Some(start_task) = start_from {
        info!("Reduced run! starting from {}", &start_task);
        factfile.get_tasks_in_order_from(&start_task)  
    } else {
        factfile.get_tasks_in_order()
    };
    
    for (idx, task_level) in tasks.iter().enumerate() {
        info!("Run level: {}", idx);
        for task in task_level.iter() {
            info!("Task name: {}", task.name);
        }
    }
    
    let mut task_results:HashMap<String, TaskExecutionResult> = HashMap::new(); 
    for task_level in tasks.iter() {    // TODO replace me with helper iterator
        for task in task_level.iter() {
           let new_task_result = TaskExecutionResult { name: task.name.clone(), attempted: false, run_details:None };
           task_results.insert(new_task_result.name.clone(), new_task_result );
        }
    }    
              
    for task_level in tasks.iter() {
        // everything in a task "level" gets run together
        let (tx, rx) = mpsc::channel::<(usize, TaskResult, Option<String>, Option<String>, DateTime<UTC>)>();
    
        for (idx,task) in task_level.iter().enumerate() {
            info!("Running task '{}'!", task.name);
            {
                let tx = tx.clone();
                let args = format_args(&task.command, &task.arguments);
                let executor = task.executor.to_string();
                let continue_job_codes = task.on_result.continue_job.clone();
                let terminate_job_codes = task.on_result.terminate_job.clone();
                let task_name = task.name.to_string();
                
                thread::spawn(move || {
                    let start_time = UTC::now();
                    let (task_result, stdout, stderr) = execute_task(task_name, executor, args, terminate_job_codes, continue_job_codes);
                    tx.send((idx, task_result, stdout, stderr, start_time)).unwrap();
                });  
            }          
        }        
        
        let mut terminate_job_please = false; 
        let mut task_failed = false;
        
        for _ in 0..task_level.len() {                    
            match rx.recv().unwrap() {
                (idx, TaskResult::Ok(code, duration), stdout, stderr, start_time) => {                    
                     info!("'{}' returned {} in {:?}", task_level[idx].name, code, duration); 
                     let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                     task_result.attempted = true;
                     task_result.run_details = Some(RunResult { run_started: start_time,
                                                                duration: duration,
                                                                requests_job_termination: false,
                                                                task_execution_error: None, 
                                                                stdout: stdout,  
                                                                stderr: stderr,
                                                                return_code: code });                               
                }, 
                (idx, TaskResult::Error(code, msg), stdout, stderr, start_time)   => { 
                    warn!("task '{}' failed to execute!\n{}", task_level[idx].name, msg); 
                    let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                    task_result.attempted = true;
                    
                    if let Some(return_code) = code {
                        task_result.run_details = Some(RunResult {
                                                            run_started: start_time, 
                                                            duration: Duration::from_secs(0),
                                                            requests_job_termination: false,
                                                            task_execution_error: Some(msg), 
                                                            stdout: stdout,  
                                                            stderr: stderr,
                                                            return_code: return_code });      
                    }
                    task_failed = true;
                },
                (idx, TaskResult::TerminateJobPlease(code, duration), stdout, stderr, start_time) => {
                     warn!("job will stop as task '{}' called for termination (no-op) with code {}", task_level[idx].name, code);
                     
                     let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                     task_result.attempted = true;
                     task_result.run_details = Some(RunResult {
                                                           run_started: start_time, 
                                                           duration: duration,
                                                           requests_job_termination: true,
                                                           task_execution_error: None, 
                                                           stdout: stdout,  
                                                           stderr: stderr,
                                                           return_code: code });     
                     
                     terminate_job_please = true; 
                }
            }
        }
        
        match (terminate_job_please, task_failed) {
            (_, true) => { return ExecutionResult::AbnormalTermination(drain_values(task_results, &tasks)); },
            (true, false) => { return ExecutionResult::EarlyFinishOk(drain_values(task_results, &tasks)); }, 
            _ => {}
        }
    }

    ExecutionResult::AllTasksComplete(drain_values(task_results, &tasks))       
}

fn execute_task(task_name:String, executor:String, args:String, terminate_job_codes:Vec<i32>, continue_job_codes:Vec<i32>) -> (TaskResult, Option<String>, Option<String>) {
   if executor!="shell" {
        return (TaskResult::Error(None, "Only shell executions are supported currently!".to_string()), None, None) 
    } else {
        let run_start = Instant::now(); 
        info!("Executing sh -c {:?}", args); 
        match Command::new("sh").arg("-c").arg(args).output() { 
            Ok(r) => {
                let run_duration = run_start.elapsed();
                let return_code = r.status.code().unwrap_or(1); // 1 will be returned if the process was killed by a signal  
                
                let task_stdout: String = String::from_utf8_lossy(&r.stdout).trim_right().into();
                let task_stderr: String = String::from_utf8_lossy(&r.stderr).trim_right().into();
                
                info!("task '{}' stdout:\n'{}'", task_name, task_stdout);
                info!("task '{}' stderr:\n'{}'", task_name, task_stderr);
                                
                let task_stdout_opt = if task_stdout.is_empty() { None } else { Some(task_stdout) };
                let task_stderr_opt = if task_stderr.is_empty() { None } else { Some(task_stderr) };
                
                if terminate_job_codes.contains(&return_code) {
                    (TaskResult::TerminateJobPlease(return_code, run_duration), task_stdout_opt, task_stderr_opt)
                } else if continue_job_codes.contains(&return_code) {
                    (TaskResult::Ok(return_code, run_duration), task_stdout_opt, task_stderr_opt)
                } else {
                    let expected_codes = continue_job_codes.iter()
                                                 .map(|code| code.to_string())
                                                 .collect::<Vec<String>>()
                                                 .join(",");
                    (TaskResult::Error(Some(return_code), format!("the task exited with a value not specified in continue_job - {} (task expects one of the following return codes to continue [{}])", return_code, expected_codes)), 
                        task_stdout_opt,
                        task_stderr_opt)
                }
            
            },
            Err(message) => (TaskResult::Error(None, format!("Error executing process - {}", message)), None, None)
        }
    }
}

fn format_args(command:&str, args:&Vec<String>) -> String {
    let arg_str = args.iter()
                      .map(|s| format!("\"{}\"", s))
                      .collect::<Vec<String>>()
                      .join(" ");
    format!("{} {}", command, arg_str)
}

