
## The zen of Factotum

1. A Turing-complete job is not a job, it's a program
2. A job must be composable from other jobs
3. A job exists independently of any job schedule

## Spec for first version (move into wiki)

### 1. Running a DAG

```bash
$ factotum dag run FILE
```

where `FILE` is a local file containing a self-describing JSON per #2.

The execution will be similar to `make -k` - i.e. keep running as far as possible through the DAG if a task fails.

On success, print out a summary to stdout.

In case of failure, we return a 1 error code and print out a summary of what succeeded and what failed to stderr.

In the case of a noop, we (needs discussion).

Out of scope for v1:

* Validating the self-describing JSON (will wait on Iglu Rust Client)
* Fancy rules on what to do in case of failure
* SD-JSON representing job status to stdout or stderr on success or failure

TODO: please turn these three out-of-scopes into tickets.

### 2. Generating a DAG from a Makefile

We need this so we can start migrating makefiles over to Factotum.

```bash
$ factotum dag make2factotum FILE
```

where `FILE` is a minimalistic Makefile, containing only the following elements:

* Variables
* Rules

Example of a minimalistic Makefile that will be convertable:

```make
pipeline=acme

done: run-web-sql
	/notify-v0.2.0.sh $(pipeline) "Completed successfully"

start:
	/notify-v0.2.0.sh $(pipeline) "Started"
check-lock: start
	/check-lock.sh $(pipeline)
emr-etl-runner: check-lock
	/r73/emr-etl-runner-r73-rc2.sh $(pipeline) && /notify-v0.2.0.sh $(client) "Ran EmrEtlRunner"
storage-loader: emr-etl-runner
	/storage-loader-r73-rc2.sh $(pipeline) && /notify-v0.2.0.sh $(pipeline) "Ran StorageLoader"
run-dedupe-sql: storage-loader
	/sql-runner-0.2.0-lock.sh $(pipeline) dedupe
run-web-sql: run-dedupe-sql
	/sql-runner-0.2.0-lock.sh $(pipeline) web
```

### Out of scope for v1

* Any notifications to PagerDuty, Slack etc

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
