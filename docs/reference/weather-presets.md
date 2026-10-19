# Current populated Oteryn reference parameters

All values below are working Oteryn data, classified DERIVED with explicit assumptions in the target records. They do not assert that an unproven Global value has become official evidence. Native activation remains blocked.

| Subject / field | Populated Oteryn working value |
| --- | --- |
| proposal.weather.rain / intensity | `{"normalized_intensity":0.35,"range":[0,1]}` |
| proposal.weather.rain / duration_seconds | `900` |
| proposal.weather.rain / area | `{"selector":"OUTDOOR_WALKABLE_TILES_IN_INVOKING_WORLD_AND_CHANNEL","exclude":["indoor","underground"],"geometry":"existing native tile-area tags"}` |
| proposal.weather.fog / density | `{"normalized_density":0.3,"range":[0,1],"interpretation":"linear visual-only renderer input"}` |
| proposal.weather.snow / intensity | `{"normalized_intensity":0.25,"range":[0,1]}` |

Parameter register: docs/reference/content-prep/weather-content-1/oteryn-parameters.json. Original source limitations: docs/reference/content-prep/weather-content-1/tibia-source-archive.json.

## Earlier Global source investigation

<!-- Current Oteryn values are populated in the sample JSON below. Earlier source-only UNKNOWN wording is superseded by the working-parameter decision recorded in docs/reference/content-prep/weather-content-1/oteryn-parameters.json. No renderer/runtime acceptance is implied. -->

# Weather presets: proposal only

Status: architecture ruling required. This document does not create `content/weather/`, a production schema, default effects, schedules or gameplay visibility penalties. Owner requested rain/fog/snow preparation for WEATHER-1 #2202. Labels below are proposal identifiers, not observed Global mechanics.

Architecture must decide world/channel ownership, area geometry, renderer units, normalized ranges, time basis, persistence and deterministic selection. Until then all intensities, durations, frequencies and region bindings are UNKNOWN. Weather must not imply damage, movement or visibility penalties without a separate accepted rule.

Sample evidence JSON (not a runtime contract):

```json
{
  "schema": "OTERYN_CONTENT_PREP_EVIDENCE/v1",
  "task": "WEATHER-CONTENT-1",
  "target_cut": "2026-10-18",
  "runtime_admission": "BLOCKED_PENDING_CP_QUALIFICATION",
  "runtime_switch_authorized": false,
  "reference_only": true,
  "records": [
    {
      "key": "proposal.weather.rain",
      "facts": {
        "type": {
          "value": "rain",
          "classification": "DERIVED",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [
            {
              "source": "owner:2026-10-18-weather-task",
              "selector": "Task 13; local proposed label; values not prescribed by owner"
            }
          ],
          "note": "Owner task requests weather presets; proposal-only local label, not a Global weather observation."
        },
        "intensity": {
          "value": {
            "normalized_intensity": 0.35,
            "range": [
              0,
              1
            ]
          },
          "classification": "DERIVED",
          "target_cut": "2026-10-19",
          "continuity": "OWNER_AUTHORIZED_OTERYN_WORKING_DEFAULT",
          "evidence_scope": "OTERYN_REFERENCE_PARAMETER",
          "sources": [
            {
              "source": "owner:oteryn-data-completion:2026-10-19",
              "selector": "docs/reference/content-prep/weather-content-1/oteryn-parameters.json / field docs/reference/weather-presets.md/embedded_json/0/records/0/facts/intensity"
            }
          ],
          "note": "Explicit Oteryn design choice where inspected sources do not establish a complete current Global value.",
          "assumptions": [
            "Visual-only preset; no damage, speed or gameplay visibility penalty. Normalized units are proposed renderer inputs; native architecture must accept mapping before implementation. No global weather equivalence claimed."
          ],
          "global_evidence_archive": "docs/reference/content-prep/weather-content-1/tibia-source-archive.json#docs/reference/weather-presets.md/embedded_json/0/records/0/facts/intensity",
          "runtime_admission": "BLOCKED_PENDING_NATIVE_QUALIFICATION"
        },
        "duration_seconds": {
          "value": 900,
          "classification": "DERIVED",
          "target_cut": "2026-10-19",
          "continuity": "OWNER_AUTHORIZED_OTERYN_WORKING_DEFAULT",
          "evidence_scope": "OTERYN_REFERENCE_PARAMETER",
          "sources": [
            {
              "source": "owner:oteryn-data-completion:2026-10-19",
              "selector": "docs/reference/content-prep/weather-content-1/oteryn-parameters.json / field docs/reference/weather-presets.md/embedded_json/0/records/0/facts/duration_seconds"
            }
          ],
          "note": "Explicit Oteryn design choice where inspected sources do not establish a complete current Global value.",
          "assumptions": [
            "Visual-only preset; no damage, speed or gameplay visibility penalty. Normalized units are proposed renderer inputs; native architecture must accept mapping before implementation. No global weather equivalence claimed."
          ],
          "global_evidence_archive": "docs/reference/content-prep/weather-content-1/tibia-source-archive.json#docs/reference/weather-presets.md/embedded_json/0/records/0/facts/duration_seconds",
          "runtime_admission": "BLOCKED_PENDING_NATIVE_QUALIFICATION"
        },
        "area": {
          "value": {
            "selector": "OUTDOOR_WALKABLE_TILES_IN_INVOKING_WORLD_AND_CHANNEL",
            "exclude": [
              "indoor",
              "underground"
            ],
            "geometry": "existing native tile-area tags"
          },
          "classification": "DERIVED",
          "target_cut": "2026-10-19",
          "continuity": "OWNER_AUTHORIZED_OTERYN_WORKING_DEFAULT",
          "evidence_scope": "OTERYN_REFERENCE_PARAMETER",
          "sources": [
            {
              "source": "owner:oteryn-data-completion:2026-10-19",
              "selector": "docs/reference/content-prep/weather-content-1/oteryn-parameters.json / field docs/reference/weather-presets.md/embedded_json/0/records/0/facts/area"
            }
          ],
          "note": "Explicit Oteryn design choice where inspected sources do not establish a complete current Global value.",
          "assumptions": [
            "Visual-only preset; no damage, speed or gameplay visibility penalty. Normalized units are proposed renderer inputs; native architecture must accept mapping before implementation. No global weather equivalence claimed."
          ],
          "global_evidence_archive": "docs/reference/content-prep/weather-content-1/tibia-source-archive.json#docs/reference/weather-presets.md/embedded_json/0/records/0/facts/area",
          "runtime_admission": "BLOCKED_PENDING_NATIVE_QUALIFICATION"
        }
      }
    },
    {
      "key": "proposal.weather.fog",
      "facts": {
        "type": {
          "value": "fog",
          "classification": "DERIVED",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [
            {
              "source": "owner:2026-10-18-weather-task",
              "selector": "Task 13; local proposed label; values not prescribed by owner"
            }
          ],
          "note": "Proposal-only label from requested weather families."
        },
        "density": {
          "value": {
            "normalized_density": 0.3,
            "range": [
              0,
              1
            ],
            "interpretation": "linear visual-only renderer input"
          },
          "classification": "DERIVED",
          "target_cut": "2026-10-19",
          "continuity": "OWNER_AUTHORIZED_OTERYN_WORKING_DEFAULT",
          "evidence_scope": "OTERYN_REFERENCE_PARAMETER",
          "sources": [
            {
              "source": "owner:oteryn-data-completion:2026-10-19",
              "selector": "docs/reference/content-prep/weather-content-1/oteryn-parameters.json / field docs/reference/weather-presets.md/embedded_json/0/records/1/facts/density"
            }
          ],
          "note": "Explicit Oteryn design choice where inspected sources do not establish a complete current Global value.",
          "assumptions": [
            "Visual-only preset; no damage, speed or gameplay visibility penalty. Normalized units are proposed renderer inputs; native architecture must accept mapping before implementation. No global weather equivalence claimed."
          ],
          "global_evidence_archive": "docs/reference/content-prep/weather-content-1/tibia-source-archive.json#docs/reference/weather-presets.md/embedded_json/0/records/1/facts/density",
          "runtime_admission": "BLOCKED_PENDING_NATIVE_QUALIFICATION"
        }
      }
    },
    {
      "key": "proposal.weather.snow",
      "facts": {
        "type": {
          "value": "snow",
          "classification": "DERIVED",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [
            {
              "source": "owner:2026-10-18-weather-task",
              "selector": "Task 13; local proposed label; values not prescribed by owner"
            }
          ],
          "note": "Proposal-only label from requested weather families."
        },
        "intensity": {
          "value": {
            "normalized_intensity": 0.25,
            "range": [
              0,
              1
            ]
          },
          "classification": "DERIVED",
          "target_cut": "2026-10-19",
          "continuity": "OWNER_AUTHORIZED_OTERYN_WORKING_DEFAULT",
          "evidence_scope": "OTERYN_REFERENCE_PARAMETER",
          "sources": [
            {
              "source": "owner:oteryn-data-completion:2026-10-19",
              "selector": "docs/reference/content-prep/weather-content-1/oteryn-parameters.json / field docs/reference/weather-presets.md/embedded_json/0/records/2/facts/intensity"
            }
          ],
          "note": "Explicit Oteryn design choice where inspected sources do not establish a complete current Global value.",
          "assumptions": [
            "Visual-only preset; no damage, speed or gameplay visibility penalty. Normalized units are proposed renderer inputs; native architecture must accept mapping before implementation. No global weather equivalence claimed."
          ],
          "global_evidence_archive": "docs/reference/content-prep/weather-content-1/tibia-source-archive.json#docs/reference/weather-presets.md/embedded_json/0/records/2/facts/intensity",
          "runtime_admission": "BLOCKED_PENDING_NATIVE_QUALIFICATION"
        }
      }
    }
  ],
  "evidence_scope": "SOURCE_OBSERVATIONS_WITH_EXPLICIT_OTERYN_WORKING_DEFAULTS"
}
```

No `world_weather*.rs` or generated/world-pin file is touched. Reference boundary: [source registry](content-prep/weather-content-1/sources.json).
