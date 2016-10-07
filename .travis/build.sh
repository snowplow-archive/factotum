#!/bin/bash -e

if [ "${TRAVIS_OS_NAME}" == "osx" ]; then
	echo "Configuring openssl libs for OSX..."
	#brew install openssl
	export OPENSSL_INCLUDE_DIR=`brew --prefix openssl`/include
	export OPENSSL_LIB_DIR=`brew --prefix openssl`/lib
	echo "...done!"
fi

if [ "$1" == "--release" ]; then
	cargo build --verbose --release
else
	cargo build --verbose
	cargo test --verbose
fi
