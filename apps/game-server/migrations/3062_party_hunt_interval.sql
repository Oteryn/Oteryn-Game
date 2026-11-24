-- Advisory hunt settings share the existing fenced World party lock and lifecycle.
ALTER TABLE game_parties
 ADD COLUMN hunt_interval BIGINT NOT NULL DEFAULT 0 CHECK (hunt_interval >= 0),
 ADD COLUMN hunt_started_at TIMESTAMPTZ,
 ADD COLUMN hunt_uses_leader_prices BOOLEAN NOT NULL DEFAULT FALSE,
 ADD COLUMN hunt_price_owner UUID,
 ADD COLUMN hunt_leader_prices JSONB NOT NULL DEFAULT '[]',
 ADD CHECK ((hunt_interval = 0) = (hunt_started_at IS NULL)),
 ADD CHECK (jsonb_typeof(hunt_leader_prices) = 'array' AND jsonb_array_length(hunt_leader_prices) <= 2048);
