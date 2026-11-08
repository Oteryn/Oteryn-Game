CI on PR #3058 (cgmech) is red: tools/architecture-check enforces client file size (warning 400 lines, hard fail 600; feature mod.rs/window.rs warning 200, hard 300). On cgmech head 159a9491d these files exceed the hard limit:
- apps/client/src/features/local-reference-preferences/native.rs: 723 lines
- apps/client/src/features/0199-cyclopedia-character/native.rs: 636 lines
Task for you NOW (before anything else): merge origin/claude/happy-davinci-ijz3wy-cgmech into your work branch, then split the file(s) you own into cohesive submodules (each <= 400 lines, no behaviour change), run `cargo run -p oteryn-architecture-check -- workspace .` and the client tests, fmt, push to your work branch. From now on keep every client file under 400 lines.
