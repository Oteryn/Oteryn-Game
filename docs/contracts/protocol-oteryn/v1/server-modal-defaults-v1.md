# Server modal defaults V1

Capability 505 requires 450. Commands 736 and 737 retain their identities.
A selected connection may receive dialog tag 4: the tag 1 dialog grammar
with a default confirmation button byte immediately before the cancel flag.
Both button ids must name existing buttons. Tag 1 has no confirmation default.
The cancel button is the Escape action. Enter and a primary double click on
a choice submit the explicit default, retaining the selected choice and input.
Unselected connections receive tag 1 with unchanged bytes. Capability 490
appends its input block after either dialog grammar. Answers are unchanged.
