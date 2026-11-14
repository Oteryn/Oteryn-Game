# Platform coin wire fixtures

Copied byte-for-byte from `Oteryn/Oteryn-Platform@da97f5d4bc44172822fc7bc265a6b0406d986dd4`
`docs/contracts/fixtures/coin-wire-v1/`. The adjacent manifest records canonical bytes and SHA-256.
Source contract: `docs/contracts/OTERYN_V2_COIN_TRANSFER_AND_MARKET_CONTRACT.md` at that commit;
accepted by owner decision B, 2026-11-09, Platform #1791. No fixture is a live receipt.

COIN1 syncs the seven vectors that Platform #1852 changed (`balance-ok.json`,
`transfers-ok.json`, `offers-ok.json`, `settlements-ok.json`, `cancellations-ok.json`,
`operation-ok.json`, `replay.json`) byte-for-byte to
`Oteryn/Oteryn-Platform@a85b5da4de23f3e8633cfb25a2db1ad7f6133acd`
(`non_transferable*` always "0", the whole balance in `transferable*`), following the
"One coin class" contract at `bcaa1605` and implementation `a058456e`. Every manifest
hash equals the Platform manifest at that commit.
