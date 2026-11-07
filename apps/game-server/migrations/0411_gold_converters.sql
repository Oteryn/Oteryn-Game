-- GOLDCONV4: bind a charged converter to the existing carried-coin command receipt.
ALTER TABLE game_coin_exchange_receipts
  ADD COLUMN IF NOT EXISTS converter_source UUID,
  ADD COLUMN IF NOT EXISTS converter_kind SMALLINT;

ALTER TABLE game_coin_exchange_receipts DROP CONSTRAINT IF EXISTS game_coin_exchange_converter_shape;
ALTER TABLE game_coin_exchange_receipts ADD CONSTRAINT game_coin_exchange_converter_shape CHECK (
  (converter_source IS NULL AND converter_kind IS NULL)
  OR (item_source IS NOT NULL AND converter_source IS DISTINCT FROM item_source
      AND converter_kind IN (1, 2))
);

REVOKE ALL ON TABLE game_coin_exchange_receipts FROM PUBLIC;
GRANT SELECT, INSERT, UPDATE ON TABLE game_coin_exchange_receipts TO oteryn_game_runtime;
GRANT SELECT ON TABLE game_coin_exchange_receipts TO oteryn_game_control;
