# Animus Mastery v1

Capability 168 exposes command 232 to an admitted session. It reads the admitted Character's own Animus Mastery completions. Completions are server-owned and immutable.

## Request

Exactly 4 bytes: tag `0`, a little-endian `u16` offset, and a `u8` limit from 1 to 50. Any other payload is refused.

## Reply

A little-endian `u16` total completion count, a `u8` count of keys in this page, then that many species keys. Each key is a `u8` length followed by that many bytes of lowercase ASCII (`a-z`, `0-9`, `:`, `_`, `.`, `-`). Keys are in ascending order. The reply is at most 8192 bytes.
