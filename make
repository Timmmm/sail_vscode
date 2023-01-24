#!/bin/sh

dir="$(dirname -- "$0")"
cd "$dir/buildsystem" && cargo run --quiet -- "$@"
