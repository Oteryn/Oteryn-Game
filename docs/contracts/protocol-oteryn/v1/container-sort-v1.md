# Container sort v1

Task: NEWS-HIST-CONTAINER-SORT-1. Capability 146 requires container trees (14).
Command 168 reorders the existing placement ordinals through the item loop owner.

## Request

Exactly four bytes: open view id (0–15), key (1 name, 2 total weight, 3 remaining
expiry, 4 stack quantity), descending (0/1), recursive (0/1). Unknown values fail
closed. Only the target must be open; recursive sorting includes closed descendants.
Current carried custody, session generation, lease, scope and Content are rechecked
in the durable transaction. Ground trees are refused, matching the current carried
container move scope. Physical trade reservations refuse sorting under the item lane.

Bags precede other items in both directions. Direction applies only to the key;
ties use definition production key then instance id ascending. Names compare using
Unicode lowercase. Weight includes stack quantity and all descendants. Missing
expiry follows expiring items ascending (and precedes them descending). Expiry uses
the current timed state's remaining milliseconds, evaluated at one transaction time.
No item identity, quantity, state revision, parent or custody changes.

## Reply

One byte: 1 sorted (including unchanged/empty), 2 refused. An unknown durable outcome
ends the connection; resending the same CommandRef reconciles the retained receipt.
Foundation returns its standard `DuplicateReplay` status for a remembered command.
Successful sorting refreshes every open container view using domain 14. Receipts
bind Character, CommandRef, target, options, current fence, Content and the exact
ordinal permutation; same request replay returns success without writing again.
