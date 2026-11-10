# Sewer USE hypotheses
1. Created sewer435 is sent as MapAdded handle; no production USE handler recognizes that source, so real USE returns NothingToUse / StaleState instead of descending.
2. Immutable sewer435 similarly falls through to the generic door handler. Resolve the current base ordinal before choosing the same dropdown operation.
3. Landing without ground must refuse before committing, retaining the actor and plate. Reuse the sealed movement teleport proof, PZ checks, special movement StepOut, and ingress replay owner.
Source: Crystal9f5a72c6 data/scripts/actions/items/ladder_up.lua:1-22. Sewer435 is the explicit downward branch; source does not consume or transform it on USE.
