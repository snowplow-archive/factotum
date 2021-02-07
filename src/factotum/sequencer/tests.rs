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

use factotum::factfile::*;
use daggy::*;
use factotum::tests::*;

#[test]
fn recursive_find_ok() {
    let mut dag = Dag::<Task, ()>::new();
    let parent = make_task("root", &vec![]);

    let idx = dag.add_node(parent);

    let task_child1 = make_task("child1", &vec![]);
    dag.add_child(idx, (), task_child1);

    let task_child2 = make_task("child2", &vec![]);
    let (_, child2) = dag.add_child(idx, (), task_child2);

    let grandchild_node = make_task("grandchild", &vec![]);
    let (_, grandchild_idx) = dag.add_child(child2, (), grandchild_node);

    if let Some((found_idx, found_node)) = super::find_task_recursive(&dag, "grandchild", idx) {
        assert_eq!(found_idx, grandchild_idx);
        assert_eq!(found_node.name, "grandchild");
    } else {
        panic!("couldn't find value");
    }
}

#[test]
fn get_tasks_in_order_basic() {
    let mut dag = Dag::<Task, ()>::new();

    let parent = make_task("root", &vec![]);

    let root_idx: NodeIndex = dag.add_node(parent);

    let child1 = make_task("child1", &vec![]);

    let child2 = make_task("child2", &vec![]);

    dag.add_child(root_idx, (), child1);
    let (_, child2_idx) = dag.add_child(root_idx, (), child2);

    let grandchild = make_task("grandchild", &vec![]);
    dag.add_child(child2_idx, (), grandchild);

    let expected = vec![vec!["root"], vec!["child2", "child1"], vec!["grandchild"]];

    let mut actual: Vec<Vec<&Task>> = vec![];
    super::get_tasks_in_order(&dag, &vec![root_idx], &mut actual);

    compare_tasks(expected, actual);
}

#[test]
fn check_valid_subtree() {
    // root <-- start here
    // /    \
    // child1 child2
    // \
    // grandchild
    //

    let mut dag = Dag::<Task, ()>::new();

    let parent = make_task("root", &vec![]);
    let root_idx: NodeIndex = dag.add_node(parent);

    let child1 = make_task("child1", &vec![]);
    let child2 = make_task("child2", &vec![]);

    dag.add_child(root_idx, (), child1);
    let (_, child2_idx) = dag.add_child(root_idx, (), child2);

    let grandchild = make_task("grandchild", &vec![]);
    dag.add_child(child2_idx, (), grandchild);

    assert_eq!(true, super::is_proper_sub_tree(&dag, root_idx));
}

#[test]
fn check_invalid_subtree() {
    // root <-- start here ok
    // /    \
    // child1 child2 <-- start here fails
    // \      \
    // grandchild
    //
    //

    let mut dag = Dag::<Task, ()>::new();

    let parent = make_task("root", &vec![]);
    let root_idx: NodeIndex = dag.add_node(parent);

    let child1 = make_task("child1", &vec![]);
    let child2 = make_task("child2", &vec![]);

    let (_, child1_idx) = dag.add_child(root_idx, (), child1);
    let (_, child2_idx) = dag.add_child(root_idx, (), child2);

    let grandchild = make_task("grandchild", &vec![]);

    let (_, grandchild_idx) = dag.add_child(child2_idx, (), grandchild);

    dag.add_edge(child1_idx, grandchild_idx, ()).ok().unwrap();

    assert_eq!(false, super::is_proper_sub_tree(&dag, child2_idx));

    assert_eq!(true, super::is_proper_sub_tree(&dag, root_idx));
}
