# Character information: Premium observation extension

T165 adds operation byte 5 to existing command 113 under capability 97. Request
is exactly `[5]`, with no account, Character or target argument. Result layout,
authority rules and compatibility are defined in
[OTERYN_PREMIUM_PANEL_V1.md](../../OTERYN_PREMIUM_PANEL_V1.md).

Operations 0–4 remain unchanged. This adds no capability or registry identifier.
The read is restricted to the admitted session's current fenced Character/account
and is not a Premium grant or a replacement for gameplay eligibility checks.
