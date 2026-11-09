# Native key-door receiver slice

The existing `DoorBook::use_key_observed` supplies an opaque opening only after a
successful matching-key native transition. Its wrong-key, stale, closing and
concurrent-attempt cases remain the World owner's tests. Ordinary quest commands
remain forbidden for the reserved `oteryn:quest-door-use/` prefix.

The registered Root receiver binds a server-captured placement, native opening
revision, key instance/number, observed Character revision and quest definition.
It checks exact carried custody and refuses an opening captured before a later
mission start/reset. Existing native-death receivers retain their original
binding and admission logic behind the shared internal observation slot. Runtime
callers use the new revision-slot method; the writer registration includes it.

The PostgreSQL case uses a registered-receiver fixture and actually mints the
numbered copper keys through the native reward owner. It qualifies Root admission
and replay, not the physical Draconia route. Production dispatch capture/delivery
and the exact Draconia placement binding are the next slice; no production door
binding is enabled by this receiver change alone.
