# PROF-1 PostgreSQL semantic qualification

Exact candidate: 9bde3bfcc946e18600913e295a983998321f4fdfd9dd244bf5b28a28e04142b5.

Independent authority fixture: account guard + Character root revision 1 and independently declared progression revision 1, level 1, XP 0, context revisions v1, Harmony/time 0. Receipt builders do not resolve these from records. SQL boundary sees only current Character root/progression; live session/lease/PZ/content admission and active N are NOT_APPLICABLE to this migration and remain future writer obligations.

| Invariant class | Consumer boundary | Applicable mutation operators |
|---|---|---|
| identity/binding | header CHECK/FK/unique | UUID version; missing/wrong Character; original/committed successor; malformed source context; empty/oversize command; wrong digest length; cause |
| current authority | header deferred guard | missing lines; selection multiple lines; receipt beyond/stale root; missing/stale progression; cross-kind duplicate revision |
| identity/binding | line CHECK/FK | malformed Item/definition; same-key binding; header Character/revision/cause substitution; duplicate track |
| temporal/direction | line CHECK | training no progress/revision/selection substitution; selection no-op/two slots/progress/revision/length substitution; migration progress/no revision change |
| shape | line/current row CHECK | NULL array; empty; NULL elements accepted; 0/2 elements accepted; 3/-1 rejected; multi-dimensional; non-one lower bound; >7 |
| provenance | line deferred guard | first nonzero progress/filled choices; wrong predecessor progress/selection/definition/revision; historical append; latest row missing/stale/mismatched |
| preservation | immediate history/row guard | receipt and line UPDATE/DELETE/TRUNCATE; current DELETE/rekey/decreasing progress/TRUNCATE |
| global progression | shared deferred guard | initial relation; missing one kind; duplicate cross-kind; mixed-kind discontinuity; XP/death/stance/bestiary/charm/monk/build/proficiency valid successors and per-kind invalid tip |

Negative cases begin from independently valid seed/transition and change one named semantic invariant. A fresh transaction rolls each case back. Expected SQLSTATE and specific rejection message identify the rejecting boundary. Positive controls run under the identical fixture first. The candidate is applied only to a unique isolated local database; no runtime activation, live data or remote changes.
