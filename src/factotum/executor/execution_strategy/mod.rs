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
use std::process::Command;
use std::time::{Instant, Duration};

#[derive(Clone, PartialEq, Debug)]
pub struct RunResult {
    pub duration: Duration,
    pub task_execution_error: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub return_code: i32,
}

pub fn simulation_text(name: &str, command: &Command) -> String {

    use std::cmp;
    let command_text = format!("{:?}", command);

    let col_task_title = "TASK";
    let col_command_title = "COMMAND";
    let col_padding = 2;
    let task_col_width = cmp::max(name.len() + col_padding, col_task_title.len() + col_padding);
    let command_col_width = cmp::max(command_text.len() + col_padding,
                                     col_command_title.len() + col_padding);

    let lines = vec![ 
                      format!("/{fill:->taskwidth$}|{fill:->cmdwidth$}\\", fill="-", taskwidth=task_col_width, cmdwidth=command_col_width),
                      format!("| {:taskwidth$} | {:cmdwidth$} |", "TASK", "COMMAND", taskwidth=task_col_width-col_padding, cmdwidth=command_col_width-col_padding),
                      format!("|{fill:-<taskwidth$}|{fill:-<cmdwidth$}|", fill="-", taskwidth=task_col_width, cmdwidth=command_col_width),
                      format!("| {:taskwidth$} | {:-<cmdwidth$} |", name, command_text, taskwidth=task_col_width-col_padding, cmdwidth=command_col_width-col_padding),
                      format!("\\{fill:-<taskwidth$}|{fill:-<cmdwidth$}/\n", fill="-", taskwidth=task_col_width, cmdwidth=command_col_width),
                    ];

    lines.join("\n")
}

pub fn execute_simulation(name: &str, command: &mut Command) -> RunResult {
    info!("Simulating execution for {} with command {:?}",
          name,
          command);
    RunResult {
        duration: Duration::from_secs(0),
        task_execution_error: None,
        stdout: Some(simulation_text(name, &command)),
        stderr: None,
        return_code: 0,
    }
}

pub fn execute_os(name: &str, command: &mut Command) -> RunResult {
    let run_start = Instant::now();
    info!("Executing sh {:?}", command);
    match command.output() {
        Ok(r) => {
            let run_duration = run_start.elapsed();
            let return_code = r.status.code().unwrap_or(1); // 1 will be returned if the process was killed by a signal

            let task_stdout: String = String::from_utf8_lossy(&r.stdout).trim_right().into();
            let task_stderr: String = String::from_utf8_lossy(&r.stderr).trim_right().into();

            info!("task '{}' stdout:\n'{}'", name, task_stdout);
            info!("task '{}' stderr:\n'{}'", name, task_stderr);

            let task_stdout_opt = if task_stdout.is_empty() {
                None
            } else {
                Some(task_stdout)
            };
            let task_stderr_opt = if task_stderr.is_empty() {
                None
            } else {
                Some(task_stderr)
            };

            RunResult {
                duration: run_duration,
                task_execution_error: None,
                stdout: task_stdout_opt,
                stderr: task_stderr_opt,
                return_code: return_code,
            }
        }
        Err(message) => {
            RunResult {
                duration: Duration::from_secs(0),
                task_execution_error: Some(format!("Error executing process - {}", message)),
                stdout: None,
                stderr: None,
                return_code: -1,
            }
        }
    }
}
