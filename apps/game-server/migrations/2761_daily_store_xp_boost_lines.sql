-- WU26-03: Winter Update 2026 adds the Daily Reward XP boost (10/20/25/50) to the
-- Store XP boost (50) instead of extending its timer, so a kill records the sum.
ALTER TABLE game_party_kill_xp_members DROP CONSTRAINT game_party_kill_xp_members_xp_boost_percent_check;
ALTER TABLE game_party_kill_xp_members ADD CONSTRAINT game_party_kill_xp_members_xp_boost_percent_check
    CHECK(xp_boost_percent IN(0,10,20,25,50,60,70,75,100));
