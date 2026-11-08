# T026 account Friends — proposal, not an accepted runtime contract

The current VIP contract gives contacts and groups Account + World scope, but
consent belongs to a Character. Its accepted invitations must not silently grant
account-wide visibility or disclose links between alternate characters.

Proposed behavior:

- Keep every existing VIP invitation and character visibility rule unchanged.
- Add a separate account Friends invitation with explicit account-level acceptance.
- Let an authenticated owner select one of their owned characters as their Game
  social main character; never infer this from a contact, login order or name.
- Show that selected main character only to accepted account friends. Revocation
  removes that disclosure on the next query.
- Preserve World and Channel distinctions, session-generation fencing and durable
  command receipts. Do not expose AccountIds or convert character consent.

The pending owner choice is whether to adopt this separate account-level consent
or retain character-only consent. Until resolved, T026 remains PARTIAL; the client
has the confirmed VIP Friends/invitation view but no authoritative main-character
producer. This proposal does not authorize or implement a new permission grant.
