# Team Finder leader v1

## Upsert
The request carries bounded minimum/maximum level, vocation mask, total/free slots,
client timestamp, and exactly one boss, hunt (kind plus area), or quest target.

## Remove
The request payload is empty.

## Reply
Success has an empty payload; refusal uses the command status.
