@echo off
:: The win32 crate is ignored because it just passes through windows-rs, which
:: produces lots of extraneous warnings
cargo clippy -p renderer  -p sys -p utils -p maple -- --no-deps -W clippy::pedantic -A clippy::unused_self
@echo on
