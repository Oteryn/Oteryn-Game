`server-reply.bin` is an actual command 388 selector 134 reply captured on
2026-11-15 by the real NPC purchase/dispatch test
`s10_blessings_postgres_nonpremium_and_phoenix_stages`. It contains three purchased
blessings with their receipt dates and the authoritative normal-death forecast.
Set `OTERYN_BLESSINGS_DIALOG_CAPTURE` to a file to regenerate it. The same scenario
then commits a real death and verifies its dated consumption event through dispatch.
Client reducer and Ankh Bevy tests replay the captured bytes, without requiring an
external capture variable or skipping in CI.
