// Copyright (c) 2016-2021 Snowplow Analytics Ltd. All rights reserved.
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
use std::collections::HashMap;
use factotum::executor::execution_strategy::RunResult;
use chrono::UTC;
use chrono::DateTime;

#[derive(Clone, PartialEq, Debug)]
pub enum State {
    Waiting,
    Running,
    Success,
    SuccessNoop,
    Failed(String),
    Skipped(String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct Task<T> {
    pub name: String,
    pub state: State,
    pub task_spec: T,
    pub run_started: Option<DateTime<UTC>>,
    pub run_result: Option<RunResult>,
}

impl<T> Task<T> {
    pub fn new<S: Into<String>>(name: S, task_spec: T) -> Self {
        Task {
            name: name.into(),
            state: State::Waiting,
            task_spec: task_spec,
            run_started: None,
            run_result: None,
        }
    }
}

pub type TaskGroup<T> = Vec<Task<T>>;

#[derive(Clone, Debug)]
pub struct TaskList<T> {
    pub tasks: Vec<TaskGroup<T>>,
    edges: HashMap<String, Vec<String>>,
}

impl<T> TaskList<T> {
    pub fn new() -> Self {
        TaskList {
            tasks: vec![],
            edges: HashMap::new(),
        }
    }

    pub fn add_group(&mut self, tasks: TaskGroup<T>) -> Result<(), String> {
        {
            let new_edges: Vec<&str> = tasks.iter()
                .map(|t| t.name.as_ref())
                .collect();

            for edge in new_edges {
                if self.edges.contains_key(edge) {
                    return Err(format!("Task '{}' has been added already - task names must be \
                                        unique",
                                       edge));
                } else {
                    self.edges.insert(edge.to_string(), vec![]);
                }
            }
        }

        self.tasks.push(tasks);
        return Ok(());
    }

    pub fn set_child(&mut self, parent: &str, child: &str) -> Result<(), String> {
        if self.get_task_by_name(&child).is_some() {
            if let Some(children) = self.edges.get_mut(parent) {
                children.push(child.to_string());
                Ok(())
            } else {
                Err(format!("Parent task '{}' doesn't exist!", parent))
            }
        } else {
            Err(format!("Child task '{}' doesn't exist!", &child))
        }
    }

    pub fn is_task_name_present(&mut self, name: &str) -> bool {
        self.get_task_by_name(name).is_some()
    }

    pub fn get_task_by_name(&mut self, name: &str) -> Option<&mut Task<T>> {
        for task_group in self.tasks.iter_mut() {
            for task in task_group.iter_mut() {
                if task.name == name {
                    return Some(task);
                }
            }
        }
        None
    }

    pub fn get_descendants(&self, task_name: &str) -> Vec<String> {
        let mut descendants = self.get_descendants_recursively(task_name);
        descendants.sort();
        descendants.dedup();
        descendants
    }

    fn get_descendants_recursively(&self, task_name: &str) -> Vec<String> {
        let default = &vec![];
        let deps: Vec<String> =
            self.edges.get(task_name).unwrap_or(default).iter().map(|x| x.clone()).collect();

        let mut seen = vec![];

        for dep in deps {
            seen.push(dep.clone());
            seen.extend(self.get_descendants_recursively(&dep));
        }

        return seen;
    }
}
