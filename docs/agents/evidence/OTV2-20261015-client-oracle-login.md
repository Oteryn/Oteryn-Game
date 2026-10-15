## Oracle migration / real QA login — 2026-10-15

PROVEN: owner supplied oteryn.com / Oracle and explicitly authorized a normal
QA account/character/login test. Runtime reports preproduction. Public fixture
now uses https://oteryn.com, https://gateway.oteryn.com and registered public Rust
OAuth client 01a1394f-1e08-701c-92d0-25629057faca (read-only registry check).
Previous molehill.cloud endpoints, invalid OAuth ID and expired creation request
are historical, not current blockers. Existing QA identity survived migration;
normal portal sign-in succeeded without duplicating the account.

One ordinary web-form submission created Client Probe Redcde. Request
01a1415b-509c-726f-a68f-77d15920ad63 changed from pending to the actual creation
receipt after a bounded 30-second check. Receipt JSON SHA256
c76f8b373c0d47707b046febaace2db319084f17aca2f9aa3d71559e47a427d6.
A separate read-only operator projection reports ready / AVAILABLE for character
01a1415b-54c0-789c-85a0-a5ac76416391. This is not authenticated native HTTP proof.
The receipt screenshot attempt failed; no such screenshot is claimed.

Native source1954627d1db8c0c00aa0c041bbe1f39366709b28 / executable SHA256
874620effe034b2ed3b278aff401a72e2f547eb1ee9ca6d7f5ce0c55db30ce75 is running in
a separate owned Windows fixture:
C:\Users\barte\Downloads\oteryn-oracle-login-20261015.
Its actual PKCE/S256 request, exact process-owned loopback listener and normal
Oteryn Rust Client game:ticket consent were checked. After consent the native UI
still displays its generic connection failure, before character selection.
UNKNOWN: token exchange versus owner-character request versus another native
failure; aggregate token metadata does not establish which stage succeeded.
A browser callback receipt alone does not establish sign-in or Game admission.
The owner's screenshot callback is a different request and is not QA evidence;
no authorization-code value was retained.

Game TCP7172 / OS-validated hostname and certificate chain passed TLS1.3 with
leaf SHA256 f7c1ff3adbaebad377ea180a68546c3135ab840ca81a65fa3b4cd528c3887c93
and public ISRG Root X1 SHA256
96bcec06264976f37460779acf28c5a7cfe8a3c0aae11a8ffcee05c0bddf08c6.
ALPN, ticket exchange and Game admission are NOT proven. No trust-store,
deployment, server configuration, OAuth registration or private-key changes.

Private oracle-login-acceptance.json SHA256
92ded7c830695e3e52e09dd28697dd4f1ff3b51b825b9490453c08ad25a2fb45 records
five decoded 1125x775 native captures, source, public config and explicit failed
native/admission gates. Latest visually inspected failure capture SHA256
104559f5ffd0f0e1d7277f21e6cb3ea5158724615d23e6b5fbedf29fe139134a.
The file named oracle-native-authorized.png also shows failure, not success.
PID36636 remains open; earlier PID36204 exit cause is unknown. Password stays in
the pre-existing user-bound private DPAPI fixture; no secrets entered the repo.

Recovery drivers and unchanged protected call gate are archived externally at
/workspace/oteryn-oracle-login-20261015-tools.tar.gz, SHA256
04449d7d2fa094a69b0f4e408b9b62705542698b88ce733a8f762e2337a4ffba.
Oracle Game source77af046ce14c81df6ed6cab03ae693a68acff32c / Platform source
1194d5548760d9d773a1cebb182bc5986a6a42bf. No client source changes or new
source qualification in this checkpoint. Draft PR1942 remains AUTHORING at195.
Next: obtain sanitized stage/error-class evidence for OAuth token response and
GET /api/v1/game-auth/native-characters in the owning client/Platform lane,
then repair the demonstrated cause and repeat actual character selection and
world admission. Do not create another account/character or blindly retry consent.

## Public community pages — owner report / 2026-10-15

PROVEN: GET https://oteryn.com/pl/online and /pl/highscores both return503 and
the unavailable-dependency view. Home /pl and /v1/client/directory return200.
Normal QA portal sign-in and character creation above remain functional; no
claim that all integration is absent or that native sign-in shares this cause.

Read-only diagnostic inside deployed oteryn-oracle-platform-1 (controller171730,
exit0): public_game_data.source=null; legacy connection configured=true;
PublicGameDataSource::isNative()=false. Both actual onlineCharacters() and
highscores() reads throw Illuminate\\Database\\QueryException, SQLSTATE HY000,
driver code2002. No raw exception messages, SQL, credentials or player rows
were emitted. No config/schema/service/deployment change was made.

Deployed source1194d554: routes/web.php routes these pages through
app/Http/Controllers/PublicGameData/PublicGameDataController.php, which catches
QueryException into503. app/PublicGameData/CanaryGameDataRepository.php reads
the canary MySQL connection / players, cluster_sessions and channels, not a
native Game projection. app/PublicGameData/PublicGameDataSource.php only blocks
that connection when native mode is configured; it supplies no native reader.
Setting PUBLIC_GAME_DATA_SOURCE=native would only show unavailable content
(the controller uses HTTP200 for that case), not implement live game data.

Platform owning lane needs accepted native read models for online presence and
ranking, their native producer/ingestion wiring and real HTTP acceptance; do not
reconnect an obsolete database, fabricate zero rows or mask failure with200.
This platform source/deployment repair is not a client-branch code change.
Native OAuth failure still needs separate sanitized stage diagnostics.
