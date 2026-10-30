# Cyclopedia tracker v1

## Request

Byte 0 is the action (`0` remove, `1` add). The remaining 1..128 UTF-8 bytes are the canonical creature key. Command 383 owns Bestiary entries; command 384 owns Bosstiary entries and refuses an add until that boss has at least one credited kill.

## Snapshot

An accepted result refreshes the selected tracker: a big-endian `u16` count followed by each sorted key as a `u8` byte length and UTF-8 bytes. At most 256 keys are returned.
