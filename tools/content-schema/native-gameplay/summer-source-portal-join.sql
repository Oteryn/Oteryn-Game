-- Sequential Summer portal members share one real Activity Instance/group.
-- The original participant_count remains the immutable initial batch size.
CREATE TABLE game_summer_portal_rooms (
 admission UUID PRIMARY KEY REFERENCES game_activity_handoff_groups(handoff_id),
 world_id UUID NOT NULL,
 instance_id UUID NOT NULL,
 boss_key TEXT NOT NULL CHECK(boss_key IN('oteryn:boss.rakesh_moonfang','oteryn:boss.the_moonsnow_magnolia')),
 group_kind SMALLINT NOT NULL CHECK(group_kind=3),
 group_id UUID NOT NULL,
 membership_sha256 BYTEA NOT NULL CHECK(octet_length(membership_sha256)=32),
 source_channel_id UUID NOT NULL,
 source_portal_key TEXT NOT NULL,
 source_map_digest BYTEA NOT NULL CHECK(octet_length(source_map_digest)=32),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(world_id,instance_id),
 FOREIGN KEY(world_id,instance_id) REFERENCES game_activity_handoff_groups(world_id,instance_id)
);
CREATE INDEX game_summer_portal_group ON game_summer_portal_rooms
 (world_id,boss_key,source_channel_id,group_kind,group_id,membership_sha256);
CREATE TABLE game_summer_portal_joins (
 join_id UUID PRIMARY KEY,
 admission UUID NOT NULL REFERENCES game_summer_portal_rooms,
 character_id UUID NOT NULL REFERENCES game_character_roots,
 source_game_session_id UUID NOT NULL REFERENCES game_durability_reconnect_sessions,
 source_connection_generation NUMERIC(20,0) NOT NULL CHECK(source_connection_generation>0),
 source_lease_generation NUMERIC(20,0) NOT NULL CHECK(source_lease_generation>0),
 step_occurrence NUMERIC(20,0) NOT NULL CHECK(step_occurrence>0),
 request_sha256 BYTEA NOT NULL CHECK(octet_length(request_sha256)=32),
 joined_at BIGINT NOT NULL CHECK(joined_at>=0),
 created_xact_id XID8 NOT NULL DEFAULT pg_current_xact_id(),
 UNIQUE(admission,character_id),
 UNIQUE(character_id,source_game_session_id,source_connection_generation,step_occurrence),
 FOREIGN KEY(admission,character_id) REFERENCES game_activity_handoff_participants(handoff_id,character_id)
 DEFERRABLE INITIALLY DEFERRED
);
CREATE INDEX game_summer_portal_join_source ON game_summer_portal_joins(source_game_session_id);
CREATE INDEX game_summer_portal_join_character ON game_summer_portal_joins(character_id);
REVOKE ALL ON game_summer_portal_rooms,game_summer_portal_joins FROM PUBLIC,oteryn_game_runtime,oteryn_game_control;
-- Source-context arming, same-XID reciprocal guards, existing participant guard
-- extension, and shared retirement-count amendment are composed by the closed
-- Activity owner; these tables alone grant no runtime authority.
