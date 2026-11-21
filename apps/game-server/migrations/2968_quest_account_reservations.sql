-- Timed quest payments have one pending owner per account and addon.
CREATE TABLE game_quest_account_reservations (
    account_id UUID NOT NULL REFERENCES game_character_account_guards(account_id),
    reservation_key TEXT NOT NULL
        CHECK (reservation_key ~ '^oteryn:quest-reservation/[a-z0-9-]+$' AND octet_length(reservation_key) <= 128),
    character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE CASCADE,
    pending_track TEXT NOT NULL
        CHECK (pending_track ~ '^oteryn:quest-progress/[A-Za-z0-9._:/-]+$' AND octet_length(pending_track) <= 128),
    pending_value BIGINT NOT NULL,
    PRIMARY KEY (account_id, reservation_key)
);
CREATE INDEX game_quest_account_reservations_character
    ON game_quest_account_reservations(character_id);

-- Honor a payment that already committed. If an account had two pending
-- payments before the upgrade, the earliest recorded timer takes precedence.
INSERT INTO game_quest_account_reservations
    (account_id, reservation_key, character_id, pending_track, pending_value)
SELECT DISTINCT ON (c.account_id)
    c.account_id, 'oteryn:quest-reservation/citizen-backpack', c.character_id, p.track_key, p.value
FROM (VALUES
    ('oteryn:quest-progress/quest/u7_8/citizen_outfits/addon_backpack',
     'oteryn:quest-progress/quest/u7_8/citizen_outfits/addon_backpack_timer'),
    ('oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/addon_backpack_rook',
     'oteryn:quest-progress/quest/u7_8/citizen_outfits_rook/addon_backpack_rook_timer')
) AS route(progress, timer)
JOIN game_character_quest_tracks p ON p.track_key = route.progress AND p.value = 2
JOIN game_character_quest_tracks t ON t.character_id = p.character_id
    AND t.track_key = route.timer AND t.value > 0
JOIN game_character_roots c ON c.character_id = p.character_id AND c.lifecycle <> 4
ORDER BY c.account_id, t.value, c.character_id, p.track_key;

REVOKE ALL ON game_quest_account_reservations FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE, DELETE ON game_quest_account_reservations TO oteryn_game_runtime;
GRANT SELECT ON game_quest_account_reservations TO oteryn_game_control;
