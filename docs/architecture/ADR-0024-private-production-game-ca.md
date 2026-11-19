# ADR-0024: Private production gameplay CA

- Status: Proposed (needs architect acceptance; root key custody decided by the owner 2026-11-19)
- Date: 2026-11-19
- Decision owner: Oteryn project owner
- Decides when accepted: the gameplay trust anchor only (Platform contract §17 U3). It does not close D3 (route-record acceptance), U7 (production login policy), U15 (internal producer PKI) or the D171/D172 release gates of ADR-0021, which still need their own decisions and release evidence
- Related: ADR-0003, ADR-0014, ADR-0021 (D171/D172 release blockers), FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, `OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md` (PKI row, U3/U15), owner decision batch D165-D173
- Companion contract: `docs/contracts/OTERYN_GAME_GAMEPLAY_TLS_PKI_V2.md`

The minimum-build gate dependency is Platform #1982. The links below pin its
merged contract and design.

## Context

The native client reaches a game node over TLS 1.3 (rustls 0.23, aws-lc-rs provider, ALPN
`oteryn-game/1`). The node loads its listener certificate chain and key from
`[listener] certificate_chain_file` / `private_key_file` (`apps/game-server/src/config.rs`,
`serve.rs`). The client validates the node against one root loaded from the per-user
`OTERYN_DEV_ROOT` path (`apps/client/src/native_entry/config.rs`), and takes the host, port
and `tls_server_name` from the Platform login response (`crates/platform-client/src/native_login.rs`).

D172 allows only a testing/preproduction trust: the configured `oteryn-dev-client` root.
The installer ships no trust root (`CLIENT-INSTALLER-0` §2.4) and defers a production root
to its owning contract. The topology review leaves production PKI and the gameplay CA open
(U15, U3). Without a production trust anchor the client cannot be released to players who
do not hand-install a development root, which is the release blocker ADR-0021 records.

Owner decision 2026-11-19: the public gameplay endpoint is `game.oteryn.com:443`. The
Platform edge (caddy-l4) routes it by SNI as raw TCP passthrough: it does not terminate TLS
and sends no PROXY protocol header.

## Decision test

1. **Must decide now?** YES.
2. **Blocked downstream work:** the first player-facing client release, the installer's
   shipped trust bundle, and the production listener configuration of the game node.
3. **Harder later:** a root embedded in shipped clients can only be replaced by shipping a
   new client; choosing the anchor late forces a forced-update cutover for every player.
4. **Evidence to supersede:** a requirement for browser or third-party clients that cannot
   embed a private root; a compromise of the root; a decision to terminate TLS at the edge.
5. **Not decided:** per-IP limits behind the edge (needs PROXY v2, out of scope), client
   certificates for players, Platform's own web PKI, and the HSM/KMS product for the
   intermediate (a file key on the host is the minimum; a later change may move it).

## Options

- **A. Public WebPKI certificate (ACME) on the node.** The client would still need a change
  (it trusts one configured certificate and never consults system roots), the trust
  decision moves to every public CA, the node host must answer ACME challenges for
  `game.oteryn.com` that today terminate on the Platform edge, and the client would accept any
  WebPKI-issued certificate for the name.
- **B. Private Oteryn gameplay CA embedded in the client (chosen).** The client trusts only
  roots Oteryn ships; the edge stays a passthrough; issuance stays inside Game operations.
- **C. Pin the node leaf key in the client.** Simplest, but every key rotation is a client
  release and a lost key bricks every installed client.

## Decision

1. **Two-tier private CA.**
   - *Root:* ECDSA P-256, self-signed, long-lived (10 years), generated and held **offline**
     by the owner (owner decision 2026-11-19). It signs only intermediates.
   - *Intermediate:* ECDSA P-256, `pathLenConstraint=0`, name constraint
     `permitted DNS: game.oteryn.com`, at most 2 years, held on the Oracle game host and used only
     by Game operations tooling. It signs only gameplay server leaves.
   - *Leaf:* ECDSA P-256, SAN `DNS:game.oteryn.com` only, EKU `serverAuth` only, validity at
     most 30 days. The host renews it when less than the greater of one third of its
     lifetime or 14 days remains. The node serves leaf + intermediate.
2. **Client trust.** The player build embeds a trust bundle of one or two roots
   (current, and next during a rotation overlap) and validates the node with the
   upstream rustls `WebPkiServerVerifier` against that bundle, wrapped by a Game check of
   the leaf EKU and an intermediate SPKI denylist. Development trust (`OTERYN_DEV_ROOT`)
   is selected by a compile-time feature that player-channel artefacts never carry; player
   builds have no root input in their launch path.
3. **Admission gate.** Player builds report the ordered, per-update-channel
   `release_sequence` in `offer.release`. Platform applies the effective rule
   for each game channel before preparing or signing a grant. Its §5 update
   refusal is `NATIVE_LOGIN_CLIENT_UPDATE_REQUIRED` with `update`. The existing
   TUF updater works independently of gameplay TLS. [Platform gate contract] §§2–6.
4. **Edge.** `game.oteryn.com:443` is relayed on SNI plus ALPN `oteryn-game/1` to the node's
   internal address: no TLS termination at the edge, no PROXY header. The node therefore
   sees the edge's address as the peer; any per-IP limit added later would count the edge,
   not players, until PROXY v2 is decided.
5. **Node listener.** The node listener publishes no host port; it is reachable on the
   internal docker network, which is part of the trusted host boundary. The public port is
   443 at the edge.
6. **Upstream only.** rustls and rcgen (already workspace dependencies) cover issuance,
   serving and validation. No new service, daemon or dependency.
7. **Chain selection.** During planned overlap, the node serves A and B on
   one listener. The player client advertises its embedded roots in the TLS 1.3
   `certificate_authorities` extension ([RFC 8446 §4.2.4]). The node selects B
   when its root matches a hint. A is the default for absent or unmatched hints
   until step 6 retires it. [Platform gate design] §4, option 1.
8. **Rotation, revocation and rollback.** The companion contract maps both
   rollouts to [Platform gate contract] §7. Planned rotation proves overlap
   before enforcing `s_AB` and ends old sessions before retiring A. For a root
   or intermediate compromise, `maintenance` stops new grants at once. Game
   drains A and proves B is live before Platform enforces the replacement
   release's sequence with `chain_evidence`. Revoking an intermediate also
   requires a player build with its SPKI on the denylist. Leaf compromise calls
   for reissuance without a floor change. The client makes no OCSP or CRL fetch.

## Consequences

- Release clients work without any hand-installed root; development stays unchanged.
- PR2 depends on deployment of Platform's minimum-client-build gate and the edge relay
  of Platform #1980.
- Leaf renewal is an operations job on the game host, not a Platform job.
- Implementation (PR2) follows acceptance and must meet the companion contract's §9.
  Joint preproduction tests 11–13 verify refusal before grant issuance,
  both chains on the overlap listener, and B-only service after retirement.
  [Platform gate contract] §8.

## Owner decision: root key custody

- 2026-11-19: the owner holds the root key offline, with one encrypted backup copy. The online
  intermediate lives on the Oracle game host and signs short-lived leaf certificates for
  `game.oteryn.com`. The owner re-signs the intermediate at most every two years.

[Platform gate contract]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/contracts/OTERYN_V2_MINIMUM_CLIENT_BUILD_GATE_CONTRACT_PROPOSAL.md
[Platform gate design]: https://raw.githubusercontent.com/Oteryn/Oteryn-Platform/d8b2d3774c23eb773d9f7c9ae51db18464c98032/docs/architecture/MINIMUM_CLIENT_BUILD_GATE_DESIGN.md
[RFC 8446 §4.2.4]: https://www.rfc-editor.org/rfc/rfc8446.html#section-4.2.4
