# Oteryn Game — Anti-Automation Player Reporting Open Analysis

- Date: 2026-08-27
- Issue: #220
- Admission protected main: `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`
- Status: `OPEN / NOT_ACCEPTED / REQUIRES_ANTI_ABUSE_AND_GOVERNANCE_ANALYSIS`
- Mode: architecture analysis only
- Runtime implementation authority: `NONE`

## Purpose

Preserve the owner-directed anti-automation reporting direction reached during the architecture continuation without turning player accusations into automatic enforcement or silently creating a canonical anti-cheat contract.

This document is a bounded analysis note. It complements the existing Rested / `EligibleRawXP` anti-automation analysis and the repository's UUIDv7/value-provenance direction. Exact thresholds, retention periods, scoring weights, UI flows, sanctions and schemas remain open unless explicitly stated otherwise.

## Owner-directed direction to preserve

Player reports should be an important human-signal layer in Oteryn's anti-bot system because players can notice suspicious behaviour in-world that purely automated systems may not immediately prioritize.

At the same time, reports must be designed against malicious use. A guild, PvP opponent, spawn competitor or hostile group must not be able to convert coordinated accusations into punishment of a legitimate player.

The core principle is therefore:

```text
player report != proof of botting
player report = prioritization / evidence-gathering signal
```

A report may increase investigation priority or trigger a bounded period of richer server-side observation, but enforcement must depend on independent server-authoritative evidence and an explicit risk/review policy.

## Candidate report and case identity

Oteryn's UUIDv7 direction is a natural fit for auditable report/case identity. Candidate semantic identities for later contract work are:

```text
BotReportId = UUIDv7
BotCaseId   = UUIDv7
```

A candidate `BotReport` would reference at least:

- reporter `AccountId` / reporting Character context;
- target `CharacterId`;
- `WorldId` and current `ChannelId` where relevant;
- authoritative server timestamp;
- report category such as suspected automated hunting, movement, market activity or other automation;
- optional bounded player comment;
- link to a server-generated evidence snapshot or observation window.

A `BotCase` may aggregate multiple reports, telemetry findings and economic/provenance evidence for the same target or related farm network. Report identity must remain distinct from guilt or sanction identity.

## Server-generated evidence snapshot

The system should not rely on the reporter's prose as technical evidence. At report time the server may create a bounded, authoritative context snapshot sufficient to reconstruct why the report was made.

Candidate snapshot inputs include:

- target `CharacterId`, `WorldId`, `ChannelId` and position/area context;
- current combat/hunting state;
- session duration and recent activity classification;
- recent `EligibleRawXP` aggregates;
- recent movement/route and target-selection summaries;
- relevant transaction/economy references where justified;
- server timestamp and observation provenance.

The exact retention and detail level remain open and must be balanced against privacy, storage cost and investigation value. The design should prefer server-derived facts over client clocks or reporter-supplied evidence.

## Report aggregation and duplicate handling

Multiple reports against one target should aggregate into a case rather than creating independent punishment pressure.

```text
BotCase
├─ BotReport A
├─ BotReport B
├─ BotReport C
└─ server-side telemetry/evidence
```

Report count must not contribute linearly to guilt. Ten or one hundred reports are not ten or one hundred times stronger than one report, especially when reporters are socially or operationally correlated.

Duplicate reports from the same account against the same target should be deduplicated, cooled down or otherwise rate-limited so repeated clicking cannot manufacture evidence weight.

No exact cooldown or per-day quota is selected by this analysis.

## Reporter independence and anti-collusion

The system should estimate whether reports are meaningfully independent before using report volume as a prioritization signal.

Candidate correlation signals include:

- same reporting `AccountId`;
- same guild or strongly connected social cluster;
- tightly synchronized report timing;
- repeated coordinated reporting of the same opposing guild or spawn competitors;
- historical overlap in report targets;
- suspiciously identical report patterns across many reporter accounts.

A coordinated guild mass-report may therefore be treated as one correlated cluster rather than dozens of independent witnesses.

Shared IP, household, VPN or other coarse network correlation must never be sufficient evidence by itself because legitimate players may share infrastructure. Such data, if used at all, is only a supporting risk signal under explicit privacy/governance policy.

## Hidden reporter reliability / precision

Oteryn may maintain a non-player-visible reliability signal for reporters based on historical report outcomes.

A reporter whose previous reports repeatedly corresponded to independently confirmed automation may contribute more prioritization value than an account that reports large numbers of legitimate players.

Important constraints:

- reporter reliability is hidden and must not become a public score to optimize or trade;
- an incorrect good-faith report is not abuse;
- low historical precision does not remove the ability to report;
- reporter history influences investigation priority only, not automatic guilt;
- exact scoring rules remain private and open for future anti-abuse design.

## Malicious reporting protection

Oteryn should distinguish ordinary mistakes from deliberate report weaponization.

Candidate malicious-reporting patterns include:

- extreme report spam;
- repeated targeting of PvP enemies or economic competitors without corroborating telemetry;
- coordinated mass-reporting campaigns;
- repeated reports of the same target despite cooldown/deduplication;
- reporter networks whose reports systematically map to social conflict rather than independently confirmed automation.

The preferred response is progressive anti-spam/weight reduction and investigation of coordinated abuse rather than harsh punishment for occasional false reports.

No automatic sanction for a wrong report is accepted by this analysis.

## Report-triggered enhanced observation

A sufficiently credible report or correlated set of independent reports may trigger a bounded `enhanced observation window` for the target.

This can allow Oteryn to retain or derive temporarily richer server-side telemetry than is economically justified for every player at full detail all the time, for example:

- finer action/cadence summaries;
- route-transition and encounter-order features;
- target-selection behaviour;
- death-to-return behaviour;
- market/value-transfer activity relevant to the case;
- cross-Channel continuation of materially identical farming.

Enhanced observation must still obey explicit data-retention and privacy policy. A report may trigger observation, not hidden gameplay penalties.

## Integration with `EligibleRawXP`

The existing candidate `EligibleRawXP` measure remains useful because it represents server-authoritative raw XP attributed to a Character before Rested, prey, Store, event and other downstream bonuses.

Player reports can therefore be evaluated alongside neutral activity windows such as:

- eligible raw-XP velocity;
- continuous repeatable-farming density;
- route/path repetition;
- combat/target cadence;
- session structure;
- cross-Channel farming continuation.

High playtime or high `EligibleRawXP` alone remains insufficient evidence of automation.

## Integration with UUIDv7 item/value provenance

Reports become substantially more useful when behaviour can be joined to value provenance.

Oteryn's existing UUIDv7 identity direction can support tracing material item/value events through stable identities such as item instances, source events, transaction IDs and ledger entries. A suspicious hunting case can therefore be correlated with where generated value moves afterward.

Conceptually:

```text
reported Character
      |
      +-> behaviour / EligibleRawXP
      |
      +-> loot/source events
                |
                v
        UUIDv7 provenance graph
                |
        trade / market / mule
                |
                v
           farm-network case
```

A downstream legal buyer is not guilty merely because an item once came from a bot. The purpose of provenance is to identify repeated laundering/farm-network structure and preserve forensic history, not to create guilt-by-contact.

## Reports against simple, OCR and AI bots

Player reporting should remain useful regardless of how automation controls the client:

- simple macro/cavebot;
- OCR/computer-vision bot using normal keyboard/mouse input;
- adaptive AI/VLM agent that varies paths and timings.

The report does not need to identify the bot technology. It tells the server that a particular Character or local cluster deserves analysis. The server then evaluates behaviour and value movement independently of whether the client process itself was modified.

This preserves the server-first anti-automation direction and avoids making invasive client anti-cheat the sole line of defence.

## Farm-network reporting

If players encounter a visible group of suspected bots, reports may be clustered into a broader farm investigation rather than treated only as isolated Character cases.

Backend analysis may then search for shared behavioural fingerprints, common value destinations, mule consolidation and repeated transaction patterns across targets.

The objective is not merely to remove disposable farm Characters, but to understand and disrupt the economic network when evidence supports it.

## Player-facing feedback

The preferred player experience is intentionally minimal:

```text
Thank you. Your report has been recorded.
```

A later generic message may state that action was taken on a report, but Oteryn should not disclose internal risk scores, detection features, thresholds or exact enforcement triggers. Detailed feedback would create an oracle for bot developers and malicious reporters.

## Enforcement separation

The anti-automation pipeline should preserve explicit separation:

```text
player report
    |
    v
prioritization / evidence collection
    |
    +-> EligibleRawXP / behaviour telemetry
    +-> UUIDv7 item/value provenance
    +-> reporter independence/reliability
    |
    v
risk analysis
    |
    v
review / challenge policy
    |
    v
enforcement
```

A report count, reporter reputation, long session or behavioural anomaly must not individually equal guilt.

No automatic `N reports = ban`, `N hours = bot`, `high raw XP = bot` or `same IP = bot` rule is allowed by this analysis direction.

## Open decisions before canonicalization

Before a canonical anti-automation/reporting contract is proposed, resolve at least:

1. exact report categories and minimal UI flow;
2. report deduplication and soft/hard rate-limiting semantics;
3. how reporter-independence clusters are calculated without overreaching into legitimate social relationships;
4. reporter reliability model, decay and minimum evidence requirements;
5. evidence-snapshot fields and retention periods;
6. criteria for triggering enhanced observation and its maximum scope/duration;
7. privacy/governance boundaries for account, device, network and social correlation;
8. behaviour features used for simple versus adaptive/OCR/AI automation;
9. provenance joins between behaviour, item instances, currency ledger and transaction graph;
10. farm-network case construction and innocent downstream-recipient treatment;
11. challenge/review/escalation workflow and false-positive safeguards;
12. enforcement policy and appeal/audit evidence;
13. malicious-reporting remediation without chilling good-faith reports;
14. observability needed to evaluate false positives, report precision and bot-network disruption without exposing detection thresholds.

## Non-goals / authority

This document does not authorize runtime/client/server/protocol/schema/DDL/telemetry-retention/production/enforcement implementation. It does not select an invasive client anti-cheat product, exact scoring algorithm, machine-learning model, report quota, ban threshold or sanction schedule.

No current accepted ADR/contract is superseded. The material remains an open analysis input under Issue #220 until an explicit reviewed contract or amendment is prepared.