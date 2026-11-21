# Blessings dialog v1

Capability 503 requires 199. Command 388 request `[134]` is self-only and rejected without both selections. Existing selectors and replies are unchanged.

Reply: `134:u8, forecast_length:u8, forecast:bytes` (the existing selector-131 forecast), `experience_protection_percent:u8, skill_protection_percent:u8, held:keys, history_count:u8`, then history entries `at_unix_ms:u64, acquired:u8, blessings:keys`. Integers are little endian. Keys encode `count:u8` then `length:u8, utf8:bytes` per key, strictly sorted and unique, at most eight ASCII storage keys of at most 128 bytes. At most twenty history entries; acquired is 0 or 1. No trailing bytes.

Current holdings, normal-death forecast and newest twenty NPC blessing acquisitions/death consumptions are read under one Character fence and transaction. History is ordered by committed Character revision descending, includes Twist of Fate and multi-blessing purchases, and omits deaths consuming nothing. Dates are receipt timestamps in UTC; an empty history means no recorded acquisitions/consumptions. It does not reconstruct events before receipts existed.

Experience protection uses the accepted death rule: 8% per regular blessing plus 30% for a current promotion benefit, before experience capping. The accepted writer preserves all magic/skill progress (100% protection, D58/NIGHT-W1 decision 19); the UI must not claim that skills are currently lost. Item/container probabilities, skull, amulet and Adventurer exceptions come from the same forecast as death settlement. Creature-specific Bless reductions are excluded from this baseline. Twist is a PvP safeguard, not a regular blessing.
