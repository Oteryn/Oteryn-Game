-- Food uses u16 seconds. A first meal can exceed the 1,200-second fullness threshold.
ALTER TABLE game_character_last_positions
    DROP CONSTRAINT IF EXISTS game_character_last_positions_rest,
    ADD CONSTRAINT game_character_last_positions_rest CHECK (
        rest IS NULL OR (cardinality(rest)=4 AND array_ndims(rest)=1 AND array_lower(rest,1)=1
            AND array_length(rest,1)=4 AND array_position(rest,NULL) IS NULL
            AND rest[1] BETWEEN 0 AND 16383 AND rest[2] BETWEEN 0 AND 65535000
            AND rest[3] BETWEEN 0 AND 12000 AND rest[4] BETWEEN 0 AND 6000));
