#!/bin/bash

# ship release using release manager

if [ "${TRAVIS_OS_NAME}" == "osx" ]; then
    SUFFIX="_darwin_x86_64"
    export PATH=$PATH:/Users/travis/Library/Python/2.7/bin
else 
    SUFFIX="_linux_x86_64"
fi

env RM_SUFFIX=${SUFFIX} release-manager --config .travis/deploy/tasks/release_config.yaml --make-version  --make-artifact --upload-artifact
