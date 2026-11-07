# WP5 S1 upstream rustls external handshake bounds amendment

- Decision: `WP5_S1_UPSTREAM_RUSTLS_EXTERNAL_HANDSHAKE_BOUNDS/v1`
- Status: **CANDIDATE; acceptance requires independent review and protected integration**
- Owner decision: #319 comment `5764894847` (2026-09-21)
- Prior dispositions: #319 comments `5764377706` and `5764391057`
- Scope: first-playable native-source TLS resource enforcement only

## Precedence

This amendment supersedes only the earlier requirement in `OTERYN_GAME_NATIVE_SOURCE_RESOURCE_ENVELOPE_DECISION_2026-09-06.md` and its active registry wording that the 65,536-byte cumulative handshake cap or peer-chain caps be enforced by stock rustls before lower-layer allocation or verification. All requirements of that decision not expressly amended or deferred below remain binding. The caller-owned cumulative ingress boundary and the configured verifier boundary specified here govern where these retained numeric caps are enforced.

## Decision

Retain `NSRC-TLS-HANDSHAKE-INBOUND-BYTES` at the inclusive hard maximum 65,536 bytes across all inbound handshake reads. A thin caller-owned bounded ingress adapter uses checked cumulative accounting and rejects an increment that would overflow or raise the total above 65,536 before supplying any bytes from that increment to stock rustls. Rejection is `UNAVAILABLE`. Native rustls per-message and backing limits separately protect library-owned memory; proof of a native aggregate 65,536-byte bound is not required.

Use stock upstream rustls/tokio-rustls with TLS 1.3, AWS-LC and mTLS. This decision authorizes no dependency fork, custom TLS parser, generic transport framework or numeric increase. One exchange and the accepted deadlines remain binding; redirects, proxy/discovery, retries, connection pooling, HTTP/2 and compression remain unsupported.

At the configured verifier boundary, before delegating or accepting trust, Oteryn enforces the coupled peer limits: at most 4 certificates, at most 4,096 DER bytes per certificate and at most 16,384 aggregate peer-chain DER bytes. Checked arithmetic is required. Over-limit input is `UNAVAILABLE` and is never truncated. Stock rustls retains ownership of its finite native predecode backing. Existing configured trust-root limits and prevalidation remain unchanged.

## Deferred hardening metrics

The following prior targets are explicitly **DEFERRED** because stock rustls public APIs do not expose exact predecode enforcement for them in this slice:

- `NSRC-TLS-DER-DEPTH = 16` levels;
- `NSRC-TLS-DER-ELEMENTS = 512` elements per certificate;
- `NSRC-TLS-SIGNATURE-VERIFICATIONS = 4` verifications per handshake.

They are removed from the active first-slice resource registry without adding a lifecycle/status schema field. Their deferral does not weaken certificate identity, chain validation, the TLS profile, source authentication or the active byte/count limits. A later reviewed amendment may reactivate them only with an enforceable boundary.

## Boundary obligations

Qualification must cover:

- cumulative handshake totals of 65,536 and 65,537 bytes across multiple read segmentations;
- checked-add overflow and rejection before an over-limit increment reaches rustls;
- peer limits at 4/5 certificates, 4,096/4,097 DER bytes each and 16,384/16,385 aggregate DER bytes;
- coupled-limit and arithmetic-overflow cases;
- `UNAVAILABLE` and no-truncation behavior; and
- unchanged TLS 1.3, AWS-LC and mTLS verification.

Document and registry acceptance does not claim runtime implementation, source readiness, production authority or S1 completion. Runtime proof remains a separate S1 obligation.

## Bounded effect

Only this document and affected TLS entries in `docs/contracts/RESOURCE_LIMITS_REGISTRY.json` change. S2, #414, #415 and Server Seam remain held until their predecessor gates. Review and protected integration of this amendment precede runtime S1 release.
