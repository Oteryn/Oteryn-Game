# Arena v1

Capability 170 exposes commands 240 and 436 to an admitted session with current channel runtime authority. It remains bundle-world only.

## Request

The payload is empty. Challenge facts are server-owned; any payload byte is refused.

## Reply

A sequence of field 1 messages, one per Svargrond challenge, at most 1024 bytes. Each message carries varint field 1 rank (1 Greenhorn, 2 Scrapper, 3 Warlord), field 2 entry fee in gold, field 3 room count and field 4 seconds per room.

## Match request

Command 436 uses singular uint32 varint fields, at most 32 bytes:

| Field | Meaning |
|---|---|
| 1 | Action: 1 status, 2 enter the next incomplete rank, 3 advance through the exit after the server recorded the opponent's death, 4 leave, 5 claim a reward |
| 2 | Reward rank: 1 Greenhorn, 2 Scrapper, 3 Warlord; claim only |
| 3 | Reward category: 1 present, 2 supplies, 3 weapon, 4 trophy; claim only |
| 4 | Reward choice: 1, or 1–3 for weapons; claim only |

Non-claim actions must omit reward fields or send their zero defaults. Unknown actions/fields, duplicate fields, non-varint fields, invalid reward selectors and oversized messages are refused. The client cannot report kills, set a rank, supply a timer, a price, an item, or a spawn position. Character identity and session generation come from the admitted session.

## Match reply

At most 128 bytes, singular uint32 varint fields (zero is the omitted default):

| Field | Meaning |
|---|---|
| 1 | Disposition: 1 applied, 2 wrong state, 3 insufficient gold, 4 already claimed, 5 inventory full, 6 timed out, 7 busy, 8 invalid reward, 9 all challenges completed, 10 content unavailable |
| 2 | Highest completed rank, 0–3 |
| 3 | Active rank, 0–3; 0 means none |
| 4 | Current pit, 0–10; 0 means none |
| 5 | Phase: 0 idle, 1 fighting, 2 advance ready, 3 reward room |
| 6 | Remaining server-owned pit time, 0–600 seconds |
| 7–9 | Claimed reward masks for Greenhorn, Scrapper and Warlord; bits 0–3 represent present, supplies, weapon and trophy |

The successful or refused transition and its exact reply are recorded with the fenced command receipt. Replaying the same command returns the recorded outcome and never charges the fee, spawns another match, or grants another reward. A client displays the replayed state as the recorded result and can request fresh status to refresh its countdown.

## Native client

The arena panel opens on Ctrl+Shift+A and provides catalogue/status refresh, entry, pit advancement, leaving and the four independent reward claims. Combat remains on the normal attack/spell path; the panel has no kill-report control. Capability 170 is selected only when offered. The panel shows gameplay refusals and does not apply predicted rewards or progression.
