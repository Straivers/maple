@echo off
cargo clippy -p renderer -p utils -p windowing -p maple -- --no-deps -W clippy::pedantic
@echo on
