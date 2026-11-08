# questpc2 n3 resume

Packet loop exhausted: 29 owned quests, three previously pushed whole routes and 26 WAITING-ENGINE. Six packet rows belong to other lanes. See QUEST-STATUS.json for exact names and ENGINE-REQUESTS.md for owner, API evidence and acceptance requirements.

This is an engine handoff, not a new gameplay qualification. Do not redo frozen NPC slices or qualify their parent quests. After an engine capability lands, fetch/merge current main, change that quest from WAITING-ENGINE to IN PROGRESS in the local .lane/rows.md, implement the route using the shared owner, and obtain real PostgreSQL end-to-end proof before marking it DONE.

Older branches remain frozen for the collector. Publish only this n3 branch. Completion matrix, pins and packets are regenerated through their repository tools; no manual checksum edits.

Engine refresh: ENGNPC 3423ac56e supplies the sixteen-track receipt bound. Dangerous Depths still waits on the combined item/points/clock source NPC receipt; do not reimplement capacity. All six other-lane rows now use the explicit OWNED ELSEWHERE terminal state. No quest has gained a new end-to-end qualification.

Merged main 83ee3c950 (native-login challenge). ENGNPC e22dff551 adds paid-travel confirmation ownership; that fix does not add the still-missing source NPC item/points/clock, equipment, name or magic-effect transaction capabilities. All 26 engine waits remain.

Merged subsequent main e9276ad7e (client settings import/export/backups/defaults); no quest-engine API changes or new quest qualifications.
