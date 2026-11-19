# ADR-0024: Private production gameplay CA

- Status: Proposed (needs architect acceptance; root key custody decided by the owner 2026-11-19)
- Date: 2026-11-19
- Decision owner: Oteryn project owner
- Decides when accepted: the gameplay trust anchor only (Platform contract §17 U3). It does not close D3 (route-record acceptance), U7 (production login policy), U15 (internal producer PKI) or the D171/D172 release gates of ADR-0021, which still need their own decisions and release evidence
- Related: ADR-0003, ADR-0014, ADR-0021 (D171/D172 release blockers), FND-02, `CLIENT-INSTALLER-0` §2.4, `OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md` §9, `OTERYN_GAME_ARCH_PLATFORM_PREPROD_TOPOLOGY_2026-10-06.md` (PKI row, U3/U15), owner decision batch D165-D173
- Companion contract: `docs/contracts/OTERYN_GAME_GAMEPLAY_TLS_PKI_V1.md`

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
     most 30 days, renewed at two-thirds of its life. The node serves leaf + intermediate.
2. **Client trust.** The player build embeds a trust bundle of one or two roots
   (current, and next during a rotation overlap) and validates the node with the
   upstream rustls `WebPkiServerVerifier` against that bundle, wrapped by a Game check of
   the leaf EKU and an intermediate SPKI denylist. Development trust (`OTERYN_DEV_ROOT`)
   is selected by a compile-time feature that player-channel artefacts never carry; player
   builds have no root input in their launch path.
3. **Admission gate.** Revocation and root rotation are enforced by Platform's minimum
   admitted client build before any grant is issued, and player builds update through the
   existing TUF-verified updater, independent of gameplay TLS.
4. **Edge.** `game.oteryn.com:443` is relayed on SNI plus ALPN `oteryn-game/1` to the node's
   internal address: no TLS termination at the edge, no PROXY header. The node therefore
   sees the edge's address as the peer; any per-IP limit added later would count the edge,
   not players, until PROXY v2 is decided.
5. **Node listener.** The node listener publishes no host port; it is reachable on the
   internal docker network, which is part of the trusted host boundary. The public port is
   443 at the edge.
6. **Upstream only.** rustls and rcgen (already workspace dependencies) cover issuance,
   serving and validation. No new service, daemon or dependency.
7. **Rotation, revocation and rollback** follow the companion contract. There is no online
   revocation (no OCSP/CRL fetch by the client); revocation is short leaf life, a player
   build with a denylist or a new bundle, and a raised minimum client build. Root rotation
   switches the node chain only after every admitted build trusts the next root.

## Consequences

- Release clients work without any hand-installed root; development stays unchanged.
- A root or intermediate compromise needs a player build and a raised minimum build; the
  overlap bundle and the adoption gate make a planned rotation a normal update, not a cutover.
- PR2 depends on Platform's minimum-client-build gate and on the edge relay of Platform #1980.
- Leaf renewal is an operations job on the game host, not a Platform job.
- Implementation (PR2) follows acceptance and must meet every item of the contract's §9.

## Owner decision: root key custody

- 2026-11-19: the owner holds the root key offline, with one encrypted backup copy. The online
  intermediate lives on the Oracle game host and signs short-lived leaf certificates for
  `game.oteryn.com`. The owner re-signs the intermediate at most every two years.
