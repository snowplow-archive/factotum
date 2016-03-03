
## The zen of Factotum

1. A Turing-complete job is not a job, it's a program
2. A job must be composable from other jobs
3. A job exists independently of any job schedule

## Spec for first version (move into wiki)

### 1. Running a DAG

```bash
$ factotum job run FILE --var env=prod --var email=bob@acme.com
```

where `FILE` is a local file containing a self-describing JSON per #2 and `--var` specifies variables that we want to make available within the job.

The execution will be similar to `make -k` - i.e. keep running as far as possible through the DAG if a task fails.

On success, print out a summary to stdout.

In case of failure, we return a 1 error code and print out a summary of what succeeded and what failed to stderr.

In the case of a noop - needs discussion, #12.

## Copyright and license

Snowplow is copyright 2016 Snowplow Analytics Ltd.

Licensed under the **[Apache License, Version 2.0] [license]** (the "License");
you may not use this software except in compliance with the License.

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

[license]: http://www.apache.org/licenses/LICENSE-2.0
