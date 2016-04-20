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
use time::PreciseTime;
use time::Duration;
use std::thread;
use std::sync::mpsc;
use std::collections::HashMap;
use std::hash::Hash;
use colored::*;

enum TaskResult {
    Ok(i32, Duration),
    TerminateJobPlease(i32, Duration),
    InvalidTask(i32, String)
}

#[allow(dead_code)] // remove me
pub struct RunResult {
   duration: Duration,
   requests_job_termination: bool,
   task_execution_error: Option<&'static str>, 
   stdout: Option<String>,  
   stderr: Option<String>,
   return_code: i32
}

pub struct TaskExecutionResult {
   pub name: String, 
   pub run: bool,
   result: Option<RunResult>
}

pub enum ExecutionResult {
    AllTasksComplete(Vec<TaskExecutionResult>),
    EarlyFinishOk(Vec<TaskExecutionResult>),
    AbnormalTermination(Vec<TaskExecutionResult>)     
}

#[inline]
fn drain_values<K: Eq + Hash,V>(mut map:HashMap<K,V>) -> Vec<V> {
    map
     .drain()
     .map(|(_, v)| v) 
     .collect::<Vec<V>>()
}

pub fn execute_factfile(factfile:&Factfile) -> ExecutionResult {
    let tasks = factfile.get_tasks_in_order();
    
    let mut task_results:HashMap<String, TaskExecutionResult> = HashMap::new(); 
    for task_level in tasks.iter() {    // TODO replace me with helper iterator
        for task in task_level.iter() {
           let new_task_result = TaskExecutionResult { name: task.name.clone(), run: false, result:None };
           task_results.insert(new_task_result.name.clone(), new_task_result );
        }
    }    
              
    for task_level in tasks.iter() {
        // everything in a task "level" gets run together
        // this isn't quite right in a dag sense, but I think practically it'll be ok (if not we'll come back to it)
        let (tx, rx) = mpsc::channel::<(usize, TaskResult)>();
    
        for (idx,task) in task_level.iter().enumerate() {
            info!("Running task '{}'!", task.name);
            {
                let tx=tx.clone();
                let args = format_args(&task.command, &task.arguments);
                let executor = task.executor.to_string();
                let continue_job_codes = task.on_result.continue_job.clone();
                let terminate_job_codes = task.on_result.terminate_job.clone();
                let task_name = task.name.to_string();
                
                thread::spawn(move || {
                    println!("Executing task '{}'!", &task_name.cyan());
                    let task_result = execute_task(task_name, executor, args, terminate_job_codes, continue_job_codes);
                    tx.send((idx,task_result)).unwrap();
                });  
            }          
        }        
        
        let mut terminate_job_please = false; 
        let mut task_failed = false;
        
        for _ in 0..task_level.len() {                    
            match rx.recv().unwrap() {
                (idx, TaskResult::Ok(code, duration)) => {                     
                     info!("'{}' returned {} in {}", task_level[idx].name, code, duration); 
                     println!("Task '{}' after {} returned {}", &task_level[idx].name.cyan(), duration, code);
                     let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                     task_result.run = true;
                     task_result.result = Some(RunResult { duration: duration,
                                                           requests_job_termination: false,
                                                           task_execution_error: None, 
                                                           stdout: None,  
                                                           stderr: None,
                                                           return_code: code });                               
                }, 
                (idx, TaskResult::InvalidTask(code, msg))   => { 
                    warn!("task '{}' failed to execute!\n{}", task_level[idx].name, msg); 
                    let msg = format!("task '{}' failed to execute!\n{}", task_level[idx].name, msg);
                    println!("{}", &msg.red());
                    let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                    task_result.run = false;
                    task_result.result = Some(RunResult { duration: Duration::seconds(0),
                                                          requests_job_termination: false,
                                                          task_execution_error: None, 
                                                          stdout: None,  
                                                          stderr: None,
                                                          return_code: code });      
                            
                    task_failed = true;
                },
                (idx, TaskResult::TerminateJobPlease(code, duration)) => {
                     warn!("job will stop as task '{}' called for termination (no-op) with code {}", task_level[idx].name, code);
                     println!("Job will now stop as task '{}' ended with {}", &task_level[idx].name.cyan(), code);
                     
                     let task_result:&mut TaskExecutionResult = task_results.get_mut(&task_level[idx].name).unwrap();
                     task_result.run = true;
                     task_result.result = Some(RunResult { duration: duration,
                                                           requests_job_termination: true,
                                                           task_execution_error: None, 
                                                           stdout: None,  
                                                           stderr: None,
                                                           return_code: code });     
                     
                     terminate_job_please = true; 
                }
            }
        }
        
        match (terminate_job_please, task_failed) {
            (_, true) => { return ExecutionResult::AbnormalTermination(drain_values(task_results)); },
            (true, false) => { return ExecutionResult::EarlyFinishOk(drain_values(task_results)); }, 
            _ => {}
        }
    }

    ExecutionResult::AllTasksComplete(drain_values(task_results))          
}



fn execute_task(task_name:String, executor:String, args:String, terminate_job_codes:Vec<i32>, continue_job_codes:Vec<i32>) -> TaskResult {
    if executor!="shell" {
        TaskResult::InvalidTask(101, "Only shell executions are supported currently!".to_string()) 
    } else {
        let start_time = PreciseTime::now();        
        match Command::new("sh").arg("-c").arg(args).output() { 
            Ok(r) => {
                
                let stop_time = PreciseTime::now();
                let run_duration = start_time.to(stop_time);
                let return_code = r.status.code().unwrap_or(1); // 1 will be returned if the process was killed by a signal  
                
                let task_stdout = String::from_utf8_lossy(&r.stdout);
                let task_stderr = String::from_utf8_lossy(&r.stderr);
                info!("task '{}' stdout:\n{}", task_name, task_stdout);
                info!("task '{}' stderr:\n{}", task_name, task_stderr);
                
                if task_stdout.len() != 0 {
                    println!("Task '{}' wrote the following to STDOUT:\n{}", &task_name.cyan(), &task_stdout.trim_right().green());
                }
                
                if task_stderr.len() != 0 {
                    println!("Task '{}' wrote the following to STDERR:\n{}", &task_name.cyan(), &task_stderr.trim_right().red());
                }
                
                if terminate_job_codes.contains(&return_code) {
                    TaskResult::TerminateJobPlease(return_code, run_duration)
                } else if continue_job_codes.contains(&return_code) {
                    TaskResult::Ok(return_code, run_duration)
                } else {
                    let expected_codes = continue_job_codes.iter()
                                                 .map(|code| code.to_string())
                                                 .collect::<Vec<String>>()
                                                 .join(",");
                    TaskResult::InvalidTask(return_code, format!("The task exited with a value not specified in continue_job: {} (task expects one of the following return codes to continue [{}])", return_code, expected_codes))
                }
            
            },
            Err(message) => TaskResult::InvalidTask(101, format!("Error executing process: {}", message))
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

