# Byte-exact execution logs and readable display

The repository whitespace gate checks the complete candidate against main. Rust test output includes final blank lines and PostgreSQL aligned output includes trailing padding. The .txt paths listed in [storage index](raw-log-storage-20261005.json) now contain whitespace-normalized display copies.

[Original raw logs](raw-log-originals-20261005.zip) preserve each original path and its exact bytes in a standard ZIP. The index records original and rendered SHA256 values and the archive SHA. Historical qualification/status JSON and their execution scopes remain unchanged; their original log SHA values are verified against the archived member. Current repair/latest receipt indexes explicitly distinguish displayed hashes and byte-exact raw hashes. No result, runtime source, data, test, limit or validation rule changed. The current source/binary execution remains2081af9b (2413library plus19individual native tests).
