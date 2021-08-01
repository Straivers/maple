:: Generates documents for only the modules in the file
@echo off
setlocal
    :: set RUSTDOCFLAGS=--Zunstable-options

    cargo clean --doc
    cargo doc --workspace --no-deps --open
    :: cargo +nightly doc --no-deps --open
endlocal
@echo on
