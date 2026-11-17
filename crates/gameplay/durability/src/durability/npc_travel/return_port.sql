-- One indexed lookup per explicit inbound route (policy limit: 64).
-- Completion is separate from creation; an unplaced or cancelled passage is not an origin.
-- Logical receipt order survives wall-clock repair and replay of an older arrival.
WITH recent AS MATERIALIZED (
    SELECT found.route, found.created_xact_id
    FROM unnest($2::text[]) AS requested(route)
    CROSS JOIN LATERAL (
        SELECT receipt.route, receipt.created_xact_id
        FROM game_newhaven_travel_receipts AS receipt
        WHERE receipt.character_id = encode($1::bytea, 'hex')::uuid
          AND receipt.route = requested.route
          AND receipt.arrived_at IS NOT NULL
        ORDER BY receipt.created_xact_id DESC
        LIMIT 1
    ) AS found
), newest AS (
    SELECT created_xact_id FROM recent ORDER BY created_xact_id DESC LIMIT 1
)
SELECT CASE WHEN count(DISTINCT route) = 1 THEN min(route) ELSE NULL END
FROM recent
WHERE created_xact_id = (SELECT created_xact_id FROM newest);
