#!/bin/bash -e

if [ "${TRAVIS_OS_NAME}" == "osx" ]; then
    pip2 install --user release-manager==0.1.0
else
    pip install --user release-manager==0.1.0
fi
