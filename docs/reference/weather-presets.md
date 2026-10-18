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
          "value": null,
          "classification": "UNKNOWN",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [],
          "note": "Architecture must define normalization and renderer units."
        },
        "duration_seconds": {
          "value": null,
          "classification": "UNKNOWN",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [],
          "note": "No proven Global schedule or approved Oteryn duration."
        },
        "area": {
          "value": null,
          "classification": "UNKNOWN",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [],
          "note": "No approved world/channel scope or affected-area binding."
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
          "value": null,
          "classification": "UNKNOWN",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [],
          "note": "Renderer units and physical interpretation await architecture."
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
          "value": null,
          "classification": "UNKNOWN",
          "target_cut": "2026-10-18",
          "continuity": "NOT_ESTABLISHED",
          "sources": [],
          "note": "No accepted renderer contract."
        }
      }
    }
  ]
}
```

No `world_weather*.rs` or generated/world-pin file is touched. Reference boundary: [source registry](content-prep/weather-content-1/sources.json).
