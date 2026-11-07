# Retained binding integrity P1 repair advisory review

Disposition: accepted and repaired in the private candidate; source repair verified. Executable GREEN qualification remains root-owned and pending at this review. No additional P0/P1/P2 findings.

Reviewed only native-candidate.patch and the actual formatted codec delta against pre-fix codec SHA256 8809f7c74a707013a9b65709a7d28a581fcfae38176c97cf81041b40ff94ddb1. The supplied real-PG RED log confirms all six Training/PerkSelection hash/version/digest corruption cases ran and restored their baselines; read/open accepted each corruption before repair.

Repair correctness: every receipt now contributes to the intents map. Selection expected-track revision is captured from the same-track immutable predecessor before latest.insert, including sparse global revision histories; absent predecessor is rejected. Training/Migration use None. Header association validation and request construction enforce common cause and cardinality. The existing constructor normalizes line ordering, and full v1 command binding comparison covers version, digest and all semantic values for all causes. Existing migration declaration verification is unchanged. The corruption fixture now receives a valid constructed v1 binding before positive-control assertions.

Sibling behavior: replay/reconcile retain their caller-intent binding comparison, then invoke the strengthened independent history verifier. Retained reads and global authority open reach that same verifier. Historical predecessor revision is an intent field, not current authority; independent recovery/gameplay fences remain untouched. No current policy lookup or gameplay authority is introduced into historical verification.

No Cargo, repository edits or repeated whole-module review were performed.

Actual codec SHA256: ebc155ea12b57f02110e31a6172e240d93080a4e7c4981645a860b14155a8130
Patch SHA256: ed9dd7b8cddda83d068645cff19880c72c6bc897a6002dc6511c7db39ff19ca3
