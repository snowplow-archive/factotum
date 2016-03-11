#!/bin/bash
set -e

# Constants
bintray_package=factotum
bintray_artifact_prefix=factotum_
bintray_user=snowplowbot
bintray_repository=snowplow/snowplow-generic
guest_repo_path=/vagrant
dist_path=./target/release
build_dir=/vagrant
build_cmd="cargo build --release"

# Similar to Perl die
function die() {
    echo "$@" 1>&2 ; exit 1;
}

# Check if our Vagrant box is running. Expects `vagrant status` to look like:
#
# > Current machine states:
# >
# > default                   poweroff (virtualbox)
# >
# > The VM is powered off. To restart the VM, simply run `vagrant up`
#
# Parameters:
# 1. out_running (out parameter)
function is_running {
    [ "$#" -eq 1 ] || die "1 argument required, $# provided"
    local __out_running=$1

    set +e
    vagrant status | sed -n 3p | grep -q "^default\s*running (virtualbox)$"
    local retval=${?}
    set -e
    if [ ${retval} -eq "0" ] ; then
        eval ${__out_running}=1
    else
        eval ${__out_running}=0
    fi
}

# Get version, checking we are on the latest
#
# Parameters:
# 1. out_version (out parameter)
# 2. out_error (out parameter)
function get_version {
    [ "$#" -eq 2 ] || die "2 arguments required, $# provided"
    local __out_version=$1
    local __out_error=$2

    # get the version number in a line like 'version = "0.1.0"' from Cargo.toml (e.g 0.1.0)
    file_version=`perl -ne 'print $1 if m/version\s*=\s*\"(\d+\.\d+\.\d+)\"/' Cargo.toml` 
    # take the latest tag, stripping anything not looking like a version (e.g. 0.1.0-rc99 => 0.1.0)
    tag_version=`git describe --abbrev=0 --tags | perl -ne 'print $1 if /(\d+\.\d+\.\d+)/'` 
    
    tag_name=`git describe --abbrev=0 --tags` 
    
    if [[ ${file_version} != ${tag_version} ]] ; then
        eval ${__out_error}="'File version ${file_version} != tag version ${tag_version}'"
    else
        eval ${__out_version}=${tag_name}
    fi
}

# Go to parent-parent dir of this script
function cd_root() {
    source="${BASH_SOURCE[0]}"
    while [ -h "${source}" ] ; do source="$(readlink "${source}")"; done
    dir="$( cd -P "$( dirname "${source}" )/.." && pwd )"
    cd ${dir}
}

function create_bintray_package() {
    [ "$#" -eq 2 ] || die "2 arguments required, $# provided"
    local __package_version=$1
    local __out_error=$2

    echo "========================================"
    echo "CREATING BINTRAY VERSION ${__package_version}*"
    echo "* if it doesn't already exist"
    echo "----------------------------------------"

    http_status=`echo '{"name":"'${__package_version}'","desc":"Release of '${bintray_package}'"}' | curl -d @- \
        "https://api.bintray.com/packages/${bintray_repository}/${bintray_package}/versions" \
        --write-out "%{http_code}\n" --silent --output /dev/null \
        --header "Content-Type:application/json" \
        -u${bintray_user}:${bintray_api_key}`

    http_status_class=${http_status:0:1}
    ok_classes=("2" "3")

    if [ ${http_status} == "409" ] ; then
        echo "... version ${__package_version} already exists, skipping."
    elif [[ ! ${ok_classes[*]} =~ ${http_status_class} ]] ; then
        eval ${__out_error}="'BinTray API response ${http_status} is not 409 (package already exists) nor in 2xx or 3xx range'"
    fi
}


# Zips all of our binaries into individua packages
#
# Parameters:
# 1. artifact_version
# 2. out_artifact_name (out parameter)
# 3. out_artifact_[atj] (out parameter)
function build_artifact() {
    [ "$#" -eq 4 ] || die "4 arguments required, $# provided"
    local __artifact_version=$1
    local __type=$2
    local __arch=$3
    local __out_binary=$4

    all_underscores_version=`echo ${__artifact_version} | perl -ne 's/-/_/g; print $_'`
    artifact_root="${bintray_artifact_prefix}${all_underscores_version}_${__type}_${__arch}"
    artifact_name="${artifact_root}.zip"
    echo "================================================"
    echo "BUILDING ARTIFACT ${artifact_name}"
    echo "------------------------------------------------"

    vagrant ssh -c "cd ${build_dir} && ${build_cmd}"
    cd ${dist_path}
    zip ${artifact_name} ${__out_binary}
    cd ../../
}

# Uploads our artifact to BinTray
#
# Parameters:
# 1. version (e.g. 0.1.0_rc2)
# 2. type (e.g. linux)
# 3. arch (e.g. amd64)
# 3. out_error (out parameter)
function upload_artifact_to_bintray() {
    [ "$#" -eq 4 ] || die "4 arguments required, $# provided"
    local __artifact_version=$1
    local __type=$2
    local __arch=$3
    local __out_error=$4

    all_underscores_version=`echo ${__artifact_version} | perl -ne 's/-/_/g; print $_'`
    artifact_root="${bintray_artifact_prefix}${all_underscores_version}_${__type}_${__arch}"
    artifact_name="${artifact_root}.zip"
	echo "==============================="
	echo "UPLOADING ARTIFACT TO BINTRAY*"
	echo "* ~2 minutes"
	echo "-------------------------------"

	http_status=`curl -T ${dist_path}/${artifact_name} \
		"https://api.bintray.com/content/${bintray_repository}/${bintray_package}/${version}/${artifact_name}?publish=1&override=1" \
		-H "Transfer-Encoding: chunked" \
		--write-out "%{http_code}\n" --silent --output /dev/null \
		-u${bintray_user}:${bintray_api_key}`

	http_status_class=${http_status:0:1}
	ok_classes=("2" "3")

	if [[ ! ${ok_classes[*]} =~ ${http_status_class} ]] ; then
		eval ${__out_error}="'BinTray API response ${http_status} is not in 2xx or 3xx range'"
	fi
}

cd_root

# Precondition for running
running=0 && is_running "running"
[ ${running} -eq 1 ] || die "Vagrant guest must be running to push"

# Precondition
version="" && error="" && get_version "version" "error"
[ "${error}" ] && die "Versions don't match: ${error}. Are you trying to publish an old version, or maybe on the wrong branch?"

# get the api key
read -es -p "Please enter API key for Bintray user ${bintray_user}: " bintray_api_key
echo

# Build artifacts
build_artifact "${version}" "linux" "x86_64" "factotum"

create_bintray_package "${version}" "error"
[ "${error}" ] && die "Error creating package: ${error}"

upload_artifact_to_bintray "${version}" "linux" "x86_64" "error"
[ "${error}" ] && die "Error uploading package: ${error}"
