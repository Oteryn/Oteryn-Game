# CONTENT-QUEST-01 r12 — Annihilator source exit destination witness

Status: **OTS_HYPOTHESIS_ONLY / PHYSICAL_SOURCE_RETURN_BINDING / PROPOSED_NONCANONICAL**
Date: 2026-09-21
Tracking: #707 / PR #709
Evidence head: `f0d6b5f4da85d5eeea8969e22f7481961e0d0e56`
Workflow run: `35588746601`
Workflow job: `106298081624`

## Purpose

Close the one explicit spatial edge left by r11: the selected Crystal teleport table configured Annihilator room exit destination `(33213,31671,13)`, but r11 had not yet physically verified that target in the pinned `world.otbm`.

## Qualified source binding

r7/r11 already compose these source facts:

- static exit object: item `1949` at `(33236,31655,13)`;
- startup entry: `TeleportUnique[38017]` in `data-global/startup/tables/teleport.lua@2d798951e1d46b7198dacad57d792971d0045899`;
- configured destination: `(33213,31671,13)`;
- configured effect: `CONST_ME_TELEPORT`;
- generic handler: `data-global/scripts/movements/others/teleport.lua@c430e446311c87fbf3750f923a33970f9204e2ab`;
- handler UID range: `38001..40000`;
- handler accepts a Player and calls `player:teleportTo(setting.destination)`.

## Exact physical verification

The r12 one-shot evidence run reused:

- Crystal `zimbadev/crystalserver@ac447fef0935e6df52dc6b6376ae4c1534ecd73f`;
- `world.otbm` blob `e95e8f7c7a95d1b634b49a5dea5a5dc76021406b`;
- existing parser `blakinio/Otheryn@e417c5e7c22986bf4acef0495eb47f7b72c97cce`.

The strict parse completed **SUCCESS** with `problems=[]` and returned for the configured source exit destination:

```text
position          = (33213,31671,13)
ground_server_id  = 410
flags             = 0
house_id          = null
top_level_items   = []
action_ids        = none observed
unique_ids        = none observed
```

Thus the source exit path now has a fully qualified physical/configuration/handler chain for the selected Crystal revision:

```text
static exit item 1949 @ source coordinate
+ startup UID 38017 / configured destination
+ registered generic Player teleport handler
+ physically existing target tile
= qualified Crystal source normal-exit binding
```

## What this does NOT prove

This does not establish:

- Global Tibia target exit geometry;
- that this destination is a Reference respawn/reconnect/recovery anchor;
- path/reachability semantics between target and staging tiles;
- safety under channel/process loss;
- InstanceRuntime origin/return metadata;
- that arbitrary encounter failures should teleport players here.

The source binding is a **normal authored exit interaction** only.

## Oteryn return/recovery separation

Oteryn must keep these identities/policies separate:

```text
AuthoredEncounterExitAnchor
ValidatedOriginReturnMetadata
ReconnectPlacement
InstanceRecoveryHold
DeathRespawnPolicy
AdminRepairDestination
```

A source teleport destination may inform a migrated authored exit anchor after Content/evidence acceptance. It cannot automatically become the fallback destination for disconnect, crash, failed handoff, unavailable origin or admin repair.

FND-04/InstanceRuntime recovery still resolves the authoritative current placement/origin. If that destination is unavailable or ambiguous, recovery holds/fails closed rather than guessing the legacy quest exit.

## New required test

- **T83 — authored exit anchor is not recovery fallback:** a qualified normal encounter exit may lower to an explicit authored exit action, but disconnect/restart/handoff/death/admin-repair paths must use their owning typed placement/recovery policy and cannot silently route to the legacy exit coordinate. Missing or mismatched physical source destination fails source linking rather than inventing another coordinate.

Required CONTENT-QUEST-01 corpus is now **83 cases**.

## Evidence classification

```yaml
annihilator_source_exit:
  source_classification: OTS_HYPOTHESIS_ONLY
  static_exit_object: PROVEN_FOR_PINNED_MAP_BYTES
  startup_teleport_binding: SOURCE_PROVEN
  handler_binding: SOURCE_PROVEN
  configured_destination: SOURCE_PROVEN
  configured_destination_physical_tile: PROVEN_FOR_PINNED_MAP_BYTES
  native_authored_exit_candidate: MIGRATION_INPUT_ONLY
  native_recovery_fallback: REJECTED_INFERENCE
  global_target_exit_geometry: EVIDENCE_REQUIRED
```

This r12 witness composes with r1-r11 and adds no runtime/production authority.
