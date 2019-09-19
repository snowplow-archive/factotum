#!/bin/bash
set -e

bashrc=/home/vagrant/.bashrc

echo "======================="
echo "INSTALLING DEPENDENCIES"
echo "-----------------------"
apt-get update
apt-get install -y language-pack-en git unzip libyaml-dev python3-pip python-pip python-yaml python-paramiko python-jinja2 libssl-dev

echo "======================="
echo "INSTALLING RUST & CARGO"
echo "-----------------------"
# curl https://sh.rustup.rs -sSf | sh -s -- -y
echo 'curl https://sh.rustup.rs -sSf | sh -s -- -y;' | su vagrant

guidance=${vagrant_dir}/up.guidance

if [ -f ${guidance} ]; then
    echo "==========="
    echo "PLEASE READ"
    echo "-----------"
    cat $guidance
fi
