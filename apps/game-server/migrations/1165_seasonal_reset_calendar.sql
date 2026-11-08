-- W05d: annual gameplay events bind to the immutable server-save epoch date.
-- Crystal a_piece_of_cake_config.lua:121-124,547-553 supplies Cake's window;
-- schedule.json retains the accepted Halloween/Cake server-save interpretation.
-- Santa's producer is santa_claus.lua:103-122; its availability dates are the
-- separately cited Christmas revision431702 in the committed annual calendar.
-- Existing epochs retain their historical announcement without retroactive activation.
ALTER TABLE game_world_event_epochs
 ADD COLUMN seasonal_calendar_revision SMALLINT NOT NULL DEFAULT 0
  CHECK (seasonal_calendar_revision IN (0,1)),
 ADD COLUMN seasonal_event_keys TEXT[] NOT NULL DEFAULT '{}'
  CHECK (cardinality(seasonal_event_keys) BETWEEN 0 AND 4
   AND octet_length(array_to_string(seasonal_event_keys,',')) <= 256);

ALTER TABLE game_world_event_epochs
 ADD CONSTRAINT game_world_event_seasonal_legacy_shape
 CHECK (seasonal_calendar_revision <> 0 OR cardinality(seasonal_event_keys)=0);
