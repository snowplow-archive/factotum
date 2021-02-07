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

use factotum::factfile::Task;
use daggy::*;

#[cfg(test)]
mod tests;

pub fn get_tasks_in_order<'a>(dag: &'a Dag<Task, ()>,
                              start: &Vec<NodeIndex>,
                              tree: &mut Vec<Vec<&'a Task>>) {
    let mut row: Vec<&Task> = vec![];

    for idx in start {
        for row in tree.iter_mut() {
            let posn = row.iter().position(|s| s.name == dag[*idx].name);
            if let Some(remove_idx) = posn {
                row.remove(remove_idx);
            }
        }
        let no_dups = !row.iter().any(|s| s.name == dag[*idx].name);
        if no_dups {
            row.push(&dag[*idx]);
        }
    }
    tree.push(row);

    let mut children: Vec<NodeIndex> = vec![];
    for parent in start.iter() {
        for (_, node_index) in dag.children(*parent).iter(&dag) {
            children.push(node_index);
        }
    }

    if children.len() != 0 {
        get_tasks_in_order(dag, &children, tree);
    }
}

pub fn find_task_recursive<'a>(dag: &'a Dag<Task, ()>,
                               name: &str,
                               start: NodeIndex)
                               -> Option<(NodeIndex, &'a Task)> {
    if dag.children(start).iter(&dag).count() != 0 {
        if let Some((_, node)) = dag.children(start).find(&dag, |g, _, n| g[n].name == name) {
            return Some((node, &dag[node]));
        } else {
            for (_, child_node) in dag.children(start).iter(&dag) {
                if let Some(v) = find_task_recursive(dag, name, child_node) {
                    return Some(v);
                }
            }
            None
        }
    } else {
        None
    }
}

pub fn is_proper_sub_tree(dag: &Dag<Task, ()>, start: NodeIndex) -> bool {

    let mut allowed_deps = vec![];
    allowed_deps.push(&dag[start]);
    let mut process_children = vec![start];

    while process_children.len() != 0 {

        let mut next_to_process = vec![];

        for child_index in process_children.iter() {
            for (_, child_node) in dag.children(*child_index).iter(&dag) {
                allowed_deps.push(&dag[child_node]);
                next_to_process.push(child_node);
            }
        }

        for child_index in process_children.iter() {
            if *child_index != start {
                for (_, parent_node) in dag.parents(*child_index).iter(&dag) {
                    if !allowed_deps.iter().any(|node| node.name == dag[parent_node].name) {
                        return false;
                    }
                }
            }
        }

        process_children = next_to_process;
    }

    true
}
