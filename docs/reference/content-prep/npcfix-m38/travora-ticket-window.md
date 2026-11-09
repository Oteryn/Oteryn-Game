# M38 Travora ticket window

The retained manual, `docs/reference/tibia-manual/products.md:162`, specifies a
1000-gold ticket bought through a harbor captain, valid24h. Its SHA256 is
`cf08d91adf258fab093f7e66767374ecd0bdc5a401ce4bdde7e6f918b6b2c2df`.

The optional native travel ticket requirement names a Quest-owned purchase-time
track and an elapsed lifetime in seconds. The existing Quest `SetNow` operation
can record the purchase in its fenced payment transaction. Travel observes that
stored value and database time together; it does not accept a client clock or
renew the purchase time. A purchase is valid from its recorded second until,
but excluding, purchase+86400 for the Travora policy. Midnight and server-save
calendar changes do not reset an elapsed ticket. Missing/zero, future, expired
and invalid timestamps refuse entry. The bounded horizon matches the existing
Quest elapsed-time condition's366-day bound.

This is a reusable requirement, not a ticket sale or a fare discount. Existing
routes omit it. Ticket-sale/arrival integration and last-port return remain
unfinished. In particular, this requirement must not block Petros's outward
return routes and strand a visitor whose ticket expired on Travora.

Before implementation, qengine2 and qmech were fetched and their current
`content/npc_catalogue/travel.rs` models checked; neither exposed a ticket or
elapsed-time gate. Neither branch was merged.
