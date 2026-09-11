# Manifest — PR #571 chat/audit archive

- Date: **2026-09-11**
- Status: **NON-AUTHORITATIVE AUDIT EVIDENCE**
- Repository: `Oteryn/Oteryn-Game`
- Source PR head: `b2ef322791520130a2683f3004d26abd5c027ac8`
- Archive branch base: `c6103b325748fd31268c7b332defcddf76c59a66`
- `IMPLEMENTATION_AUTHORITY: NONE`
- `PRODUCTION_AUTHORITY: NONE`

## A. Readable full audit

The exact generated file `oteryn_pr571_audyt_koncowy.md` was split only to satisfy connector payload limits. Concatenate the four repository files in order **without inserting separators**:

1. `OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-01.md`
2. `OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-02.md`
3. `OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-03.md`
4. `OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-04.md`

Expected reconstructed size: **67682 bytes**.  
Expected SHA-256: `114832ecd79500f2766419be86fb6dac1f5382c9a7db7cb58d3cb6776bf2b46a`.

Part evidence:

| Part | Bytes | Git blob SHA | SHA-256 |
|---|---:|---|---|
| 01 | 15288 | `91c5e246ae0a3a239f729da7b7896ed2a30629d8` | `d514b6212788b06275969374f095be3f024e2fab9999c2e8a00e52647a4bf299` |
| 02 | 14222 | `9cd481fd344814c6680380a36adf4b8b917e7550` | `f026d36200c50d9656a01210ceaad7b9a3e1852c4aec62a5941c5d10007741c7` |
| 03 | 20267 | `044ebf1f58df3b008021eb59fd1c3a80441ea1a0` | `f28dfc1b6f6fc9dfc8343ac2c4fa235e9f8cac9c511a5f358887ed85e41530a4` |
| 04 | 17905 | `6b7165bc0c551fd0813abca1df0fb80fbce35fe2` | `c83867fa3c9116039895ed2416d90d667361e66d319390edf5e883332865aa9d` |

Example reconstruction from this directory:

```bash
cat OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-01.md \
    OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-02.md \
    OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-03.md \
    OTERYN_PR571_PRODUCT_AUDIT_2026-09-11.part-04.md \
  > oteryn_pr571_audyt_koncowy.md
sha256sum oteryn_pr571_audyt_koncowy.md
```

## B. Machine-readable 192-entry decision register

Original generated JSON: `oteryn_pr571_rejestr_decyzji.json`.

Original JSON size: **69662 bytes**.  
Original JSON SHA-256: `a02a1713f26e588d9fa8f419f1f29a64ce4ae781d2d7d3da362be032eb66a802`.

For exact preservation the JSON was gzip-compressed, then the gzip stream was split into five binary parts. Concatenate the five repository files in order, then gunzip:

1. `OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-01`
2. `OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-02`
3. `OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-03`
4. `OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-04`
5. `OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-05`

Reconstructed gzip size: **16698 bytes**.  
Reconstructed gzip SHA-256: `181f670c838a158a17924b55f8b4b4552fe8ce8a2abf47b5871fb75de3e32264`.

Part evidence:

| Part | Bytes | Git blob SHA | SHA-256 |
|---|---:|---|---|
| 01 | 4000 | `bbe9c16f9b1a7c43245096f9ba330235c552c479` | `57dcb95d9467888207fb35a56126ae87afd43ae12fb7e1b067434db1f79c7d5c` |
| 02 | 4000 | `5d40d708992f599981d67db78e954b9ca1e84c42` | `54df4c6b3e9756eab6b03f534bbefcdd7f0b55819b624024719c98f9fcfb46d5` |
| 03 | 4000 | `49142cdce4a3b2e4e71b8c301134d4a5bc996c25` | `b5f7386754389ae792d79423aac2f31db098da6080ab653bd2f8d2cf9f8e7e95` |
| 04 | 4000 | `a175fea07aa06869168a5d2ff17d83b6545146b5` | `2cac2c52a90e17a82815865feba037fe655420fc0554f928f45b453a1ad26498` |
| 05 | 698 | `12fae2fd493beae00acf7f4eb7829da81a16bdb0` | `cd0a1531125dd6679afb7371d9d761901864ff17fb18f222a497c20d3dde2c89` |

Example reconstruction:

```bash
cat OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-01 \
    OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-02 \
    OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-03 \
    OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-04 \
    OTERYN_PR571_DECISION_REGISTER_2026-09-11.json.gz.part-05 \
  > oteryn_pr571_rejestr_decyzji.json.gz
sha256sum oteryn_pr571_rejestr_decyzji.json.gz
gzip -dc oteryn_pr571_rejestr_decyzji.json.gz > oteryn_pr571_rejestr_decyzji.json
sha256sum oteryn_pr571_rejestr_decyzji.json
```

## C. Chat context

`OTERYN_PR571_CHAT_AUDIT_ARCHIVE_2026-09-11.md` records the user-visible owner requests and a content-preserving index of the conclusions saved from this chat. The full substantive audit itself is preserved in section A and the complete per-entry registry in section B.

No system/developer messages, connector payloads, credentials, hidden reasoning, or other non-user-visible execution internals are intended to be archived as project state.
