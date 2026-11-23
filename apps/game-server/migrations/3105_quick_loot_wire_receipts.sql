-- Preserve the exact command before session handles are resolved to durable Items.
ALTER TABLE game_quick_loot_receipts
    ADD COLUMN wire_command_type INTEGER,
    ADD COLUMN wire_request BYTEA,
    ADD CONSTRAINT game_quick_loot_wire_binding CHECK (
        (wire_command_type IS NULL AND wire_request IS NULL)
        OR (wire_command_type IS NOT NULL AND wire_command_type IN (114,6253)
            AND wire_request IS NOT NULL AND octet_length(wire_request) BETWEEN 1 AND 64));
