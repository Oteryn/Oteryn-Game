# M38 source choices

Maris (Mistrock) uses the existing promotion candidate at32640,31437,7, rather than a fabricated dialogue-only location. Evidence: tools/content-schema/npc-authoring/samples/promotion-candidates-v1.json:1, identity oteryn:npc.maris_mistrock. The retained Crystal source SHA256 is4bcd7342494cb23d49db54e1dfd0026034a8df39e445aefb16f3a185d91d97b1; Canary286f03f014dc7c0a699d849e10b662b09e6af7ba4886a4f1fb78379d9ad2080c. The World compiler must admit this exact point under its ordinary collision rules.

The native hidden-path and hidden-path-repeat rules already exist. This work reuses them. Main's tools/content-schema/quest-authoring/questpc2_live_eligibility.py:13-25 applies a premium prerequisite to the reviewed Beregar graph; this slice preserves it. The raw donor callback only checks whether WayToBeregar differs from1 (Crystal data-global/npc/maris_mistrock.lua:67-84). No new premium policy or retail free-account dialogue equivalence is asserted here. The negative PG case checks that an ineligible account writes neither progress nor a grant receipt.

Donor URL: https://raw.githubusercontent.com/zimbadev/crystalserver/9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d/data-global/npc/maris_mistrock.lua (the same4bcd7342 source bytes).

Promotion-candidate file SHA256: 1f9b6ebbd79d92fbcb4d6e522c6dc30c95ad83ca6f4b0f5030ee81fb2f47635a.

M38 Rascalio basic contact: keep the retained Oteryn name/job leaves, but use
only the neutral greeting prefix for the event visitor. The full retained
greeting's Draccoon mission statement is not evidence of current quest state.
This is an explicit Oteryn adaptation, not canonical dialogue fidelity; travel,
mission grants and other dialogue operations remain closed. See
rascalio-event-contact.md for the exact retained Dialogue identity and scope.
