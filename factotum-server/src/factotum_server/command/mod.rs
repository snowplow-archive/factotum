// Copyright (c) 2016-2017 Snowplow Analytics Ltd. All rights reserved.
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

use std::collections::HashMap;
use std::process::Command;

macro_rules! commands {
    ($( $key: expr => $val: expr ),*) => {{
         let mut map = ::std::collections::HashMap::new();
         $( map.insert($key, $val); )*
         ::factotum_server::command::CommandStore::new(map)
    }}
}

#[cfg(test)]
mod tests;

pub trait Execution {
    fn get_command(&self, command: &str) -> Result<String, String>;
    fn execute(&self, cmd_path: String, cmd_args: Vec<String>) -> Result<String, String>;
}

#[derive(Clone, Debug)]
pub struct CommandStore {
    pub command_map: HashMap<String, String>
}

impl CommandStore {
    pub fn new(commands: HashMap<String, String>) -> CommandStore {
        CommandStore {
            command_map: commands
        }
    }
}

impl Execution for CommandStore {
    fn get_command(&self, command: &str) -> Result<String, String> {
        match self.command_map.get(command) {
            Some(command) => Ok(command.to_owned()),
            None => Err(format!("Command <{}> not found in map.", command))
        }
    }

    fn execute(&self, cmd_path: String, cmd_args: Vec<String>) -> Result<String, String> {
        let command_str = format!("{} {}", cmd_path, cmd_args.join(" "));
        debug!("Executing: [{}]", command_str);
        let failed_command_msg = format!("Failed to execute command: [{}]", command_str);
        match Command::new(cmd_path)
                    .args(&cmd_args)
                    .output()
                    {
                        Ok(output) => {
                            if output.status.success() {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                Ok(stdout.into_owned())
                            } else {
                                let stderr = String::from_utf8_lossy(&output.stderr);
                                Err(format!("{} - {}", failed_command_msg, stderr.into_owned()))
                            }
                        }
                        Err(e) => Err(format!("{} - {}", failed_command_msg, e))
                    }
    }
}
