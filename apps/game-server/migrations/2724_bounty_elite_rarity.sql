-- T201: persist bounded per-creature rarity history in the existing fenced board/receipt.
-- At most 4096 canonical race keys (128 bytes each), u16 counters and existing task state.
-- No wallet, new writer or new table; older JSON has empty rarity history and Elite disabled.
ALTER TABLE game_character_task_boards
    DROP CONSTRAINT game_character_task_boards_body_check,
    ADD CONSTRAINT game_character_task_boards_body_check
      CHECK (jsonb_typeof(body)='object' AND octet_length(body::text)<=1048576);
ALTER TABLE game_task_board_receipts
    DROP CONSTRAINT game_task_board_receipts_reply_check,
    ADD CONSTRAINT game_task_board_receipts_reply_check
      CHECK (jsonb_typeof(reply)='object' AND octet_length(reply::text)<=1048576);

-- Bind completion dust replay to the selected bounty, even if task credit drains later.
ALTER TABLE game_party_kill_xp_members ADD COLUMN bounty_dust_key BYTEA
    CHECK (bounty_dust_key IS NULL OR octet_length(bounty_dust_key)=8);
