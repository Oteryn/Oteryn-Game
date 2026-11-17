-- T124: indexes for read-only 30-day projections over the existing settled history.
CREATE INDEX IF NOT EXISTS t124_market_history_offer
 ON game_market_history(offer_id,transaction_id) INCLUDE(price,amount,character_id);
CREATE INDEX IF NOT EXISTS t124_market_settlement_time
 ON game_market_settlements(world_id,occurred_at_us,transaction_id) WHERE kind IN(2,5);
CREATE INDEX IF NOT EXISTS t124_market_offer_definition
 ON game_market_offers(world_id,definition_key,offer_id);
