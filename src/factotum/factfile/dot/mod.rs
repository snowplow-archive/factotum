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

use factotum::factfile::Factfile;

pub fn generate_graphviz_dot(factfile: &Factfile, start: Option<String>) -> String {
    let tasks = if let Some(start_task) = start {
        factfile.get_tasks_in_order_from(&start_task)
    } else {
        factfile.get_tasks_in_order()
    };

    let mut topologically_sorted_tasks = vec![];

    for t in tasks.iter() {
        for task in t.iter() {
            topologically_sorted_tasks.push(task);
        }
    }

    let title = format!("digraph \"{}\" {{", factfile.name);

    let task_names = topologically_sorted_tasks.iter()
        .map(|t| format!("    \"{}\"\n", t.name))
        .collect::<String>();

    let task_connections = topologically_sorted_tasks.iter()
        .map(|t| {
            t.depends_on
                .iter()
                .map(|dep| format!("    \"{}\" -> \"{}\"\n", dep, t.name))
                .collect::<String>()
        })
        .collect::<String>();


    format!("{}\n{}{}{}", title, task_names, task_connections, "}")
}
