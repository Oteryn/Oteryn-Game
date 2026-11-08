# Kilmaresh: Yonan's Regalia assembly

Reuse questl2b be6626b49508c33b062c097480ed173472a8bdba's qualified
Regalia exchange and native player journey. The fresh branch uses main's
existing state-free paid item exchange owner. No migration is needed.

Crystal 9f5a72c64b87b222a0c8f7c130dadf8e2f125c6d,
data-global/npc/yonan.lua:198-209,
SHA256 ab32c38fec7fc99be97b91ed19d79e64880c9ff955d58d8eee3504282f6e3848:
parts 31572, 31573, 31574 and 31575, one each, yield one 31576. The branch
has no fee or quest-progress predicate. The native transaction keeps all
parts on refusal, commits intake and output together, and permits another
paid occurrence. The dialogue question is the exact source line 199.

The accepted canonical weight for 31576 is 3400 at
content/items/definitions/items-18500-18999.json:1, SHA256 5a64227887be32b4a1ea4a94d3cb8a1a4ad138a1891d65d0072bc3ce361bfb1d.
Canary items.xml:59035-59043 has weight 850; its pinned full source is
imports/canary/items-xml/items.xml, SHA256
1cf2992cdd7cc5b97bcf930b8c89676ec1627170008e995fd2576110e26022f2.
Keep the accepted canonical fact and require this exact reviewed pair; any
drift raises L2_REGALIA_WEIGHT_PRECEDENCE_DRIFT. Other physical facts retain
the existing source/canonical conflict checks.

PostgreSQL RED on main: the source binding was absent, after a real 18.6
binding probe lasting 4.603184268 seconds. The retained journey also checks
missing/partial inputs, the complete exchange, literal command replay, and
a second complete exchange with no artificial quest track.
