#!/bin/bash

cwd=$(realpath $(dirname $0))

pushd $cwd &> /dev/null

cargo run --manifest-path ./build/Cargo.toml -- $@
e=$?

popd &> /dev/null

exit $e
