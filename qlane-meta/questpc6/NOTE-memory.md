# PC memory limit (owner's PC froze at 46/48 GB)
Run at most ONE cargo/rustc command at a time in this lane. Never start cargo
test, clippy and check in parallel (each dispatch rustc uses 5-7 GB). Wait for
one to finish before the next. Use CARGO_BUILD_JOBS=4.
