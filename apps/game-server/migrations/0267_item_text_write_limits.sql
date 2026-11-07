-- WRITE-0 §3/§5: every writable definition has at most 4,000 Unicode scalars
-- (16,000 UTF-8 bytes). The application still enforces each definition's lower
-- bound and MAIL's unchanged 1,999/79 limits. Existing text and ACLs are retained.
ALTER TABLE game_item_texts DROP CONSTRAINT IF EXISTS game_item_texts_text_check;
ALTER TABLE game_item_texts ADD CONSTRAINT game_item_texts_text_check
 CHECK (char_length(text) BETWEEN 1 AND 4000 AND octet_length(text) <= 16000);
