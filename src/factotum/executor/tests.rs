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

use factotum::tests::make_task;
use factotum::factfile::*;
use factotum::executor::*;

#[test]
fn get_task_execution_list_good() {
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    //        apple----------             turnip
    //       /     \         \
    //   orange    egg-----   \
    //      \              \   \
    //       \               potato
    //        \                 \
    //         --------------- chicken

    let tl = get_task_execution_list(&ff, None);

    let expected = vec![ vec!["turnip", "apple"],
                         vec!["egg", "orange"],
                         vec!["potato"],
                         vec!["chicken"] ];

    let actual:Vec<Vec<String>> = tl.tasks.iter()
                                          .map(|task_group| task_group.iter().map(|task| task.name.clone()).collect())
                                          .collect();

    assert_eq!(actual, expected);

    // check the children are correctly mapped

    assert_eq!(tl.get_descendants("turnip"), Vec::<String>::new());
    assert_eq!(tl.get_descendants("apple"),  vec!["chicken", "egg", "orange", "potato"]);
    assert_eq!(tl.get_descendants("egg"),    vec!["chicken", "potato"]);
    assert_eq!(tl.get_descendants("orange"), vec!["chicken"]);
}

#[test]
fn get_task_execution_list_good_reduced() {
    let mut ff = Factfile::new("test");
    ff.add_task_obj(&make_task("apple",   &vec![]));
    ff.add_task_obj(&make_task("turnip",  &vec![]));
    ff.add_task_obj(&make_task("orange",  &vec!["apple"]));
    ff.add_task_obj(&make_task("egg",     &vec!["apple"]));
    ff.add_task_obj(&make_task("potato",  &vec!["apple", "egg"]));
    ff.add_task_obj(&make_task("chicken", &vec!["potato","orange"]));

    let tl = get_task_execution_list(&ff, Some("potato".to_string()));
    assert!(tl.tasks[0].len()==1);
    assert_eq!(tl.get_descendants("potato"), vec!["chicken"]);
}

#[test]
fn get_formatted_args() {
    let args_list = format_args("echo", &vec!["hello".to_string(), 
                                              "world".to_string(), 
                                              "abc abc".to_string()]);
    assert_eq!(args_list, "echo \"hello\" \"world\" \"abc abc\"");
}

// todo write test for rejecting non "shell" execution types
