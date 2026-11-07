# Other quest-associated Source components

90 assigned donor files are converted into lossless Source behavior definitions, not admitted Native Quests. Files are not one-to-one quests. No network, donor Lua execution, wiki research, canonical quest mutation, or runtime activation is performed.

The builder verifies the assigned provenance against the donor manifest, raw SHA256, raw Git blob SHA1 and byte length; it verifies compressed AST bytes against the existing qualified index before reading the tree. Its Source behavior tree recursively preserves every AST node, field, branch, loop, helper, parameter, return, expression, configuration table and comment. Reversing `op` to `node_type` reconstructs the complete qualified donor AST exactly. Unsupported target semantics remain present in that Source tree.

Derived projections index functions, guards/control flow, assignments, call sites, constructor/configuration/registration syntax, effect candidates and lexical caller context. Arguments remain available at exact pointers in the behavior tree. Callee categories are syntactic candidates: they do not prove builtin identity, target-framework compatibility or runtime activation. Same-file helper edges retain exact symbolic candidates with scope/runtime binding explicitly unproven. No external helper ownership is guessed.

UTF-8 character spans are converted to verified raw byte slices. Parser nodes without token bounds retain an AST pointer and whole-source SHA with an explicit missing-span status; positions are not fabricated. Source tree indices preserve ordering.

Build offline:

```sh
python builder.py --assignment other-assignment.json --corpus-manifest corpus-manifest.json --ast-root lua-ast --out components.json.gz
python -m unittest discover -s . -p test_builder.py
```

Both physical AST captures and the six portable AST tar shards/index are supported. Manifest cache paths may be absolute or relative to the manifest (portable CAS). Root verifies the donor corpus and AST index/shard manifests independently before this replay. Gzip output has deterministic zero mtime.

Checked result: 90 records, 18,368 AST nodes, 171 functions, 724 control/return nodes, 255 return statements and 1,788 call sites. `native_admission`, `runtime_activation`, and `oteryn_semantic_equivalence` remain false. Completing target Quest semantics and external helper/event bindings is separate work; preserving all donor behavior is not a claim those bindings are finished.

Copy-ready product files: builder.py, schema.json, test_builder.py, components.json.gz, README.md. The acquisition-specific qualification runner/log and schema generator are lane evidence only.
