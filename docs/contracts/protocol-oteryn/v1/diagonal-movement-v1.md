# DIAGONAL_MOVEMENT_V1 (371)

Extends core command 1. Paced receipts additionally require PACED_MOVEMENT_V1 (13).
Existing cardinal
codes remain North=1, East=2, South=3, West=4. New STEP codes are NorthEast=5,
SouthEast=6, SouthWest=7, NorthWest=8. An unselected diagonal is rejected before
pacing, movement or quest callbacks. No new command, state domain or facing is
introduced; TURN and actor-facing fields retain four directions.

A successful diagonal commits one adjacent destination in both axes through the
same movement authority, collision lookup and snapshot/revision compare-commit.
Height-based try-up/down is cardinal-only; destination floor-change flags retain
the existing source rules. A blocked or invalid move changes neither axis.

Diagonal duration is three times the ordinary source duration after beat
rounding. The ordinary maximum remains 111150 ms; selected sessions accept
receipts up to 333450 ms, including a TOO_EARLY remainder from an earlier
diagonal. The legacy paced codec retains its old bound. Both capabilities are
required to admit the larger receipt. Native sprites remain four-way; horizontal
movement wins the facing after a diagonal.

Client Numpad 1/3/7/9 sends southwest/southeast/northwest/northeast respectively,
only after negotiation. Existing modal input routing and paced movement queues
apply. No new persistent setting or separate UI window is added.
