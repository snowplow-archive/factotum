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

use daggy::*;
use factotum::sequencer;

pub struct Factfile {
    pub name:String,
    dag: Dag<Task, ()>,
    root: NodeIndex
}

#[derive(Clone)]
pub struct Task {
    pub name: String,
    pub depends_on: Vec<String>,
    pub executor: String,
    pub command: String,
    pub arguments: Vec<String>,
    pub on_result: OnResult
}

#[derive(Clone)]
pub struct OnResult {
    pub terminate_job: Vec<i32>,
    pub continue_job: Vec<i32>
}

impl Factfile {

    pub fn new<S: Into<String>>(name:S) -> Factfile {
        let mut new_dag = Dag::<Task, ()>::new();
        let root_task = Task { name:"FactotumJob".to_string(),
                               depends_on: vec![],
                               executor: "".to_string(),
                               command: "".to_string(),
                               arguments: vec![],
                               on_result: OnResult { terminate_job: vec![], continue_job: vec![] } };
        let parent = new_dag.add_node(root_task);
        Factfile { name: name.into(), dag: new_dag, root:parent }
    }
    
    fn get_tasks_in_order_from_node_index<'a>(&'a self, start_node_index:NodeIndex) -> Vec<Vec<&'a Task>> {
        let mut tree:Vec<Vec<&Task>> = vec![];
        sequencer::get_tasks_in_order(&self.dag, &self.dag.children(start_node_index).iter(&self.dag).map(|(_, node_idx)| node_idx).collect(), &mut tree);
        tree
    }
    
    pub fn get_tasks_in_order_from<'a>(&'a self, start_from:&str) -> Vec<Vec<&'a Task>> {
        if let Some((task_index, task)) = self.find_task_by_name(start_from) {
            let mut tasks = self.get_tasks_in_order_from_node_index(task_index); // we also need to add in the start task!
            tasks.insert(0, vec![task]);
            tasks 
        } else {
            panic!("cannot start from {} - task does not exist", start_from);
        }
    }

    pub fn get_tasks_in_order<'a>(&'a self) -> Vec<Vec<&'a Task>> {
        self.get_tasks_in_order_from_node_index(self.root)
    }    

    fn find_task_by_name(&self, name:&str) -> Option<(NodeIndex, &Task)> {
        sequencer::find_task_recursive(&self.dag, name, self.root)
    }
    
    pub fn can_job_run_from_task(&self, name:&str) -> Result<bool, &'static str> {
        let task_index = self.find_task_by_name(name);
        if let Some((node_index, _)) = task_index {
            Ok(sequencer::is_proper_sub_tree(&self.dag, node_index))
        } else {
            Err("the task specified could not be found")
        }
    }
    
    // this is used in tests
    #[allow(dead_code)]
    pub fn add_task_obj(&mut self, task:&Task) {
        self.add_task(&task.name,
                      &task.depends_on.iter().map(AsRef::as_ref).collect(),
                      &task.executor, 
                      &task.command,
                      &task.arguments.iter().map(AsRef::as_ref).collect(),
                      &task.on_result.terminate_job,
                      &task.on_result.continue_job) // TODO should this function really be the main one? or even the only one, its nicer to pass a struct as it has named params
    }

    pub fn add_task(&mut self,
                    name:&str,
                    depends_on:&Vec<&str>,
                    executor:&str,
                    command:&str,
                    args:&Vec<&str>,
                    terminate_job_on: &Vec<i32>,
                    continue_job_on: &Vec<i32>) { // TODO ensure all fields are validated, Result is returned rather than panic (and get tests in shape for this validation)

         if let Some((_,task)) = self.find_task_by_name(name) {
            panic!("Key '{}' already exists!", task.name)
        }

        if depends_on.len() > 0 {
            if depends_on.iter().any(|s| s==&name) {
                panic!("A task cannot depend on itself");
            }

            let mut parents:Vec<NodeIndex> = vec![];
            let mut deps:Vec<String> = vec![];

            for dependency in depends_on {
                if let Some((idx,_)) = self.find_task_by_name(dependency) {
                    parents.push(idx);
                    deps.push(dependency.to_string());
                } else {
                    panic!("A task must have it's dependencies already defined - couldn't find definition of {}", dependency);
                }
            }

            let node = self.dag.add_node(Task { name: String::from(name),
                                                depends_on: deps,
                                                executor: String::from(executor),
                                                command:String::from(command),
                                                arguments: args.iter().map(|s| String::from(*s)).collect(),
                                                on_result: OnResult { terminate_job: terminate_job_on.iter().map(|i| *i).collect(), 
                                                                      continue_job: continue_job_on.iter().map(|i| *i).collect() } });

            for parent in parents {
                if let Err(_) = self.dag.add_edge(parent, node, ()) {
                    panic!("Couldn't add edge between {} and {}!", self.dag[parent].name, self.dag[node].name);
                }
            }
        } else {
            let new_task = Task { name: String::from(name),
                                  depends_on: vec![],
                                  executor: String::from(executor),
                                  command: String::from(command),
                                  arguments:  args.iter().map(|s| String::from(*s)).collect(),
                                  on_result: OnResult { terminate_job: terminate_job_on.iter().map(|i| *i).collect(),
                                                        continue_job: continue_job_on.iter().map(|i| *i).collect() } };
            self.dag.add_child(self.root, (), new_task);
        }
    }

}
