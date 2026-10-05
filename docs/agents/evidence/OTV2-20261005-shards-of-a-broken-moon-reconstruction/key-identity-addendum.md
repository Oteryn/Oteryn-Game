# Asura Citadel key identity addendum

## Cross-source result

The earlier Oteryn family proposal treated donor id `54262` as a MEDIUM possible Shards key and explicitly did not prove the association.

A fresh independent web cross-check strengthens, but does not fully close, that identity:

- the Fandom Item IDs index names `54262` as **Key (SU26)**;
- TibiaWiki has **Key (Asura Citadel)** as a new 15.30.a30dad item, used at the exact Shards door `33899,32666,8`, re-granted by Javala when lost, and unnecessary after quest completion;
- the Summer Update 2026 item list contains the generic new `Key (SU26)` alongside the Shards-specific new quest items.

This supports:

```text
donor item 54262
  -> DERIVED candidate for Key (Asura Citadel)
```

It is **not promoted to EXACT** by this packet because the public Asura-key page exposes no item id/key number and the donor has no quest script that binds id 54262 to that door.

## Crystal Summer door-table readback

Exact `00ce02a5:data-global/startup/tables/door_key.lua` contains the historical key-door table but has no Asura Citadel / `33899,32666,8` / `54262` entry.

Therefore Crystal Summer does not provide the executable key-door semantics for this quest.

## Runtime consequence

The final implementation still needs owner-qualified facts for:

- the canonical Item identity;
- immutable key number / key binding, if this nonstandard key uses the ordinary key-number path;
- the exact door placement binding;
- lost-key regrant eligibility;
- post-completion bypass.

Do not infer a key number from item id `54262`.

The post-completion rule remains a Door/Key owner-composition blocker, because the accepted key-door path and conjunction-only quest-gate path do not currently express `matching_key OR quest_completed` as one accepted gate.
