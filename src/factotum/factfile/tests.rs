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
use factotum::tests::compare_tasks;
use factotum::tests::make_task;

#[test]
#[should_panic(expected = "Key 'hello' already exists!")]
fn duplicate_names_panics() {
    let mut f = Factfile::new("test");
    let dup_task = make_task("hello", &vec![]);
    f.add_task_obj(&dup_task);
    f.add_task_obj(&dup_task);
}

#[test]
#[should_panic(expected = "A task cannot depend on itself")]
fn job_depend_itself() {
    let self_depending_task = make_task("hello", &vec!["hello"]);
    Factfile::new("test").add_task_obj(&self_depending_task);
}

#[test]
#[should_panic(expected = "A task must have it's dependencies already defined")]
fn task_depend_existing_tasks_only() {
    let non_existing_task = make_task("mytask", &vec!["undefined as yet"]);
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&non_existing_task);
}

#[test]
fn a_complicated_tree_works() {
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    //           FactotumJob --------------
    //          /                          \
    //        apple----------             turnip
    //       /     \         \
    //   orange    egg-----   \
    //      \              \   \
    //       \               potato
    //        \                 \
    //         --------------- chicken

    let expected = vec![ vec!["turnip", "apple"],
                         vec!["egg", "orange"],
                         vec!["potato"],
                         vec!["chicken"] ];

    let actual = ff.get_tasks_in_order();

    compare_tasks(expected, actual);
}

#[test]
fn heirachy_only_last() {
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&make_task("a",      &vec![]));
    ff.add_task_obj(&make_task("b",      &vec!["a"]));
    ff.add_task_obj(&make_task("c",      &vec!["a"]));
    ff.add_task_obj(&make_task("banana", &vec!["b", "a"]));

    //              FactotumJob
    //                  |
    //     ----------- a
    //     \         /   \
    //      \       b     c
    //       \     /
    //       banana

    // it's not cyclic, but it will come out incorrectly if we just run each of the children, because banana is a child of both b and a
    // what should happen is the following;

    let expected = vec![ vec!["a"],
                         vec!["c", "b"],
                         vec!["banana"] ];

    // (only the topmost dep needs running, subsequent deps are already satisfied)

    let actual = ff.get_tasks_in_order();

    compare_tasks(expected, actual);
}

#[test]
fn no_cycles_ok() {
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&make_task("hello",       &vec![]));
    ff.add_task_obj(&make_task("hello_world", &vec!["hello"]));
    ff.add_task_obj(&make_task("say_hello",   &vec!["hello"]));
    ff.add_task_obj(&make_task("say_goodbye", &vec!["say_hello", "hello_world"]));

    //              FactotumJob
    //                  |
    //               hello
    //           /          \
    //        hello_world  say_hello
    //           \          /
    //           say_goodbye

    let expected = vec![ vec!["hello"],
                         vec!["say_hello", "hello_world"],
                         vec!["say_goodbye"] ];

    let actual = ff.get_tasks_in_order();

    compare_tasks(expected, actual);
}
