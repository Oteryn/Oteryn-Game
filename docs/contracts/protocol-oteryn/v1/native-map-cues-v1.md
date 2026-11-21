# Qualified native map cues v1

Capability 385 requires capabilities 23 and 30. It is offered only after the current native Bundle and content pins qualify; availability includes a published World whose current coordinates use the legacy positive-floor basis. Offering support does not select a coordinate basis.

## Epoch basis

An empty domain 19 snapshot of type 2 declares the native cue basis for the current scope epoch. Type 1 retains the legacy basis. Type 2 requires capability 385; a scope transition supplies its own qualified snapshot. Presentation delta type 2 and local chat delta type 6 carry native positions with Global z from 0 through 15, with ground at 7. Their fields, ordering, identity, size and unknown-field checks otherwise match the existing codecs. Chat also requires capability 7, and monster speech retains its existing ambient capability gate.

## Native presentation

Native presentation uses the existing ordered batch fields with Global z validation.

## Native local chat

Native local chat uses only the Local variant of the existing line fields. Nonlocal variants are refused by delta type 6.

## Legacy compatibility

Presentation and local chat use Global z under both delta types. The scope marker
continues to gate the native delta types. The client uses the decoded floor directly.

The conditional capability offer must be identical at BOOT, fresh authentication, resume and handoff. The source frame used for cue visibility and encoding comes from the same current actor and scope proof. No casting command or player casting feature is added.
