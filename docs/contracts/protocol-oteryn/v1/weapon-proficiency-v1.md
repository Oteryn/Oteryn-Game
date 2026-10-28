# Weapon Proficiency v1

Capability 188 owns commands 480–483. Query has an empty payload. Mutation payloads contain a
16-byte occurrence, a length-prefixed canonical item key, a little-endian track revision and the
operation fields. Selection carries level/perk; refine carries slot; reshape carries slot/choice
(`3` declines an offer). Replies contain status, little-endian committed revision and bounded
operation data. All malformed or unselected-capability requests are refused without mutation.
