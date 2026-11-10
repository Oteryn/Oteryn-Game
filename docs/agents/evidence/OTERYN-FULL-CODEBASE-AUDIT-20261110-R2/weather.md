# Weather — REVIEWED

Base: `4f0b6c62a789489fca6c37dd36d5917d096f328e`. All three package files (692 lines) reviewed.

`from_json` validates schema/design tags, nonempty revision, exact slot size, positive bounded duration/intensity, recognized nonzero weighted kinds, unique area keys, resolvable climates and coordinates. `period` reconstructs a period from the public area/revision/block tuple; restart and reconnect do not create mutable random state. At most 12 positive-duration iterations are possible. `area_at` widens i32 differences to i128 before squaring. `shipped` withholds the source on invalid data.

The production authority exposes only `shipped()` (`dispatch/.../connection/authority/composed.rs:232-237`); `connection.rs:1033-1043` requires both source and authoritative UTC time, otherwise sends Unavailable. The period API's theoretical u64::MAX block overflow cannot arise from this caller because `observe` first divides millisecond time by 900000. The committed areas are sorted by key; tied anchors therefore match the documented tie policy for the shipped data. No gameplay RNG secrecy is required because this value is explicitly public and visual-only.

Existing tests cover all parser rejection branches, deterministic reconnect-equivalent behavior, boundaries, area independence and approximate distributions. Tests were inspected, not executed. No new confirmed finding. No package files remain; client rendering and live weather wire delivery are separate scopes.
