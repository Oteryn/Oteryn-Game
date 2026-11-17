# Goblin Merchant: first perfume sample

Sources checked: [Goblin Merchant Quest](https://www.tibiawiki.com.br/wiki/Goblin_Merchant_Quest#The_Perfume_Gatherer) and [Perfume Gatherer](https://www.tibiawiki.com.br/wiki/Perfume_Gatherer).
The first sample uses a freshly killed Slug: empty gatherer 17670 becomes
slug-scent gatherer 17672. This route advances the accepted programme's odours
counter from zero to one. Later gas, rotworm and cheese steps are separate routes.

The pinned native item data gives fresh Slug corpse 12526 a ten-second lifetime
before appearance 12533. The engine checks the original native corpse MINT's
transaction timestamp, immutable species provenance and current ground custody;
appearance alone cannot qualify a sample. Sampling leaves the corpse and its
contents, duration and placement unchanged. The production limit is 10000 ms.

The PostgreSQL fixture realizes a native Slug, uses its actual native health and
race, commits its death and corpse MINT, waits the full ten seconds to exercise
refusal, and then samples another fresh native Slug. Personal quest admission
uses the production Rafzan introduction rather than direct storage writes.
