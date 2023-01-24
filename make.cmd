@echo off

cd buildsystem
cargo run --quiet -- %*
