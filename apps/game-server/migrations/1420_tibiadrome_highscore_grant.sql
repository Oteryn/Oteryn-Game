-- SRX-3 category13 reads its existing Tibiadrome source under the isolated
-- highscore job role. This forward grant does not modify earlier migrations.
GRANT SELECT ON game_character_tibiadrome TO oteryn_game_highscore_job;
