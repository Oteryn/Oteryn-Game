# Owner decision batch D159-D164

- Decision: `OWNER-DECISION-BATCH-D159-D164-V1`
- Status: owner decisions recorded; docs only. Acceptance requires validation and protected integration.
- Origin: the owner answered the control-plane question batch directly on 2026-09-29 (second batch of the day; the first is `OTERYN_GAME_OWNER_DECISION_BATCH_D154_D158_2026-09-29.md`)
- Numbering: continues from D158
- Runtime, schema, registry, Platform and production authority: **NONE** (each decision names its own limits; nothing here is a merge, release or production grant)
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Decisions

| # | Topic | Owner choice (2026-09-29) | Source | Q |
|---|---|---|---|---|
| D159 | Item family table (266 ids) | The control plane approves the 176 HIGH rows. The 76 MEDIUM and 14 LOW rows are re-verified by the ITEM-ID-1 writer. Disputed ids return to the owner. Evidence: `docs/agents/evidence/OTV2-20260929-item-family-proposals-266.md`. Refines the whole-table approval of D156 for this table | Q13d, #162 2026-09-29 | Q13d |
| D160 | Platform write access for N4-P | The control plane may write to `Oteryn/Oteryn-Platform` for the native client entry task (N4-P). Scope limits are those in #162 comment 5899892092; nothing else in Platform is authorized | #162 5899892092 | Q14 |
| D161 | N4-P design and entry rules | The control plane commissions the N4-P design (Q15a). Nodes report runtime status automatically (Q16b). Character listing is a Game-owned `ListCharactersForAccount` (Q17a). Internal-build client entry may open without N6/N7 and only for internal builds (Q18a) | Q15a, Q16b, Q17a, Q18a, #162 2026-09-29 | Q15a, Q16b, Q17a, Q18a |
| D162 | Subagent concurrency | Up to 4 concurrent writing subagents, at most 1 of them running Rust builds. Separate from D157 (task sessions). The coordinator prompt amendment is a separate coordinator batch | Q19a, #162 2026-09-29 | Q19a |
| D163 | ITEM-ID-1 CI wiring | CI wiring of the ITEM-ID-1 checks in `.github/workflows/item-authoring-schema.yml` and `.github/workflows/g4-item-crystal-bindings.yml` is authorized. It still needs its own PR, checks and Merge Queue under the bound META policy | Q20a, #162 2026-09-29 | Q20a |
| D164 | N4-P decision package | Accepted as recommended by the independent re-review (#162 5900392843), with the owner answer in #162 5900403086. Items: U3 pinned Oteryn gameplay CA (dev root for Q18a builds); D1 Laravel issuer in a dedicated pool/user; D2 character list read before the ticket; P1/U12 push with watermark (S=30 s, watermark every 10 s, `transaction_timeout` enforced), pull if the measured S fails; U15 separate short-lived per-purpose certificates from a private service CA; U16 `assignment_epoch` stored with assignments plus a retained witness outside the restore unit, restore detected automatically; U18 Registry does not pin gameplay revisions; U1 separate Passport public client for the Rust client (RFC 8252 loopback); U4 grant TTL 20 s; U5 H=5 s, F=15 s; U7 per-scope native login policy in the Registry, default deny; U8 testing/preproduction only; U9 key file in testing/preprod, production KMS/HSM only if pure Ed25519 else file on a dedicated host, rotation about 90 days | #162 5900403086, 5900392843 | Q21a |

Not given a D number: **Q11** and **Q12** are already recorded (stage A #1281; D154 wording, Q12a) and are
reference only.

## 2. Notes

- D159: no id is approved beyond the 176 HIGH rows until the ITEM-ID-1 writer re-verifies the rest.
- D160 and D164 U8: Platform writes and any native login policy stay limited to testing and preproduction;
  production needs separate explicit owner authority.
- D161 Q18a and D164: the internal-build entry does not lift the N6/N7 requirements for other builds.
- D163 authorizes the two workflow changes only.
- D164: architect confirmation (Q15a: owner and architect accept) is requested from the supervising
  architect; lanes that do not depend on an open architect point start now, the rest on confirmation.
  Platform #1420 and Game #1291 remain candidates until integrated.

## 3. Handback

```yaml
result: RESOLVED_WITH_OWNER_DECISIONS
owner_decisions: [D159, D160, D161, D162, D163, D164]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_OWNER_DECISION_BATCH_D159_D164_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
```
