# Game server bootstrap governance

This directory owns the native Oteryn Game Server process composition root.

During `OTV2-IMPL-BOOTSTRAP` it is intentionally **foundation-only**:

- no gameplay socket/listener;
- no wire framing or `protocol-oteryn` schema/IDs;
- no Game Session/admission credential handling;
- no persistence/database adapter;
- no gameplay movement/combat/content semantics;
- no production deployment behavior.

The executable must fail closed when invoked for real gameplay before a later coordinator allocation merges the accepted Foundation protocol/runtime/admission seams.

Any later change that introduces protocol, session, admission, persistence, public identifiers, fencing, multichannel authority or security semantics is high risk under root governance and requires the owning accepted contract plus genuinely independent exact-head review.

Do not introduce Canary/legacy-Tibia compatibility code or an alternate login authority here.
