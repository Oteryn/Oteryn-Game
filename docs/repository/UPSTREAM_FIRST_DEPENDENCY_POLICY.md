# Upstream-first dependency policy

Status: repository-wide engineering policy for `Oteryn/Oteryn-Game`.

## Principle

Oteryn uses mature upstream implementations by default. Do not fork, vendor-modify, reimplement or deeply instrument a third-party dependency merely because a theoretical concern might exist or because a local implementation appears more controllable.

The default doctrine is:

`UPSTREAM_FIRST / PATCH_ON_PROVEN_NEED`

A downstream modification is an exception. It requires concrete evidence that the exact upstream version cannot satisfy an accepted Oteryn requirement with its supported configuration, public API, extension points or a bounded Oteryn-owned adapter.

Upstream-first is not permission to weaken accepted correctness, security, durability, compatibility or authority invariants. When upstream cannot satisfy a required invariant, keep the invariant and prove the smallest necessary exception.

## Required decision order

Before modifying a dependency, evaluate these options in order:

1. **Use upstream as-is** with supported configuration and documented features.
2. **Use an upstream extension point** or supported public API.
3. **Solve the problem above the dependency** in an Oteryn-owned wrapper, adapter, executor, policy or validation layer.
4. **Prefer an upstream contribution or tracked upstream capability** when practical and when waiting does not violate an accepted delivery requirement.
5. **Add the smallest downstream seam or patch** that closes the demonstrated gap.
6. **Maintain a fork only as a last resort** when the smaller options are demonstrably insufficient.

Do not jump directly from a hypothetical risk to a fork.

## Evidence required for an exception

At least one concrete requirement gap must be demonstrated against an exact upstream version. Suitable evidence includes:

- a reproducible failing regression, integration, adversarial or compatibility test;
- source-level proof that an accepted requirement cannot be expressed through the available upstream API or configuration;
- a representative benchmark showing a material performance or resource problem when performance is the justification;
- a concrete threat model and reproducer showing a security or availability property is missing;
- an accepted protocol, durability, authority or compatibility contract whose required behavior is absent upstream.

A vague statement such as `hardening`, `more control`, `might allocate`, `might be faster`, `future-proofing` or `we may need this later` is not sufficient evidence by itself.

Performance claims require representative and reproducible measurement. Synthetic or component benchmarks may justify a component-level conclusion, but they must not be presented as production gameplay evidence unless they exercise a representative real product path.

## Maintenance contract for downstream patches

Every retained downstream dependency patch must:

- identify the exact upstream package/version/commit or archive used as its base;
- have documented provenance and an isolated, reviewable delta;
- state the exact accepted requirement and evidence that makes the patch necessary;
- include a regression or qualification test that would fail, be impossible, or materially regress without the patch;
- avoid unrelated algorithmic, stylistic or cleanup changes in the same dependency delta;
- preserve upstream behavior outside the explicitly modified surface;
- remain small enough to rebase, audit and remove independently where practical;
- be re-evaluated when the dependency is upgraded or when upstream gains the required capability;
- be removed when the accepted requirement can be met by upstream or by a simpler Oteryn-owned layer.

Where several patches are necessary, keep them logically separable rather than turning an upstream package into an effectively independent Oteryn implementation.

## Existing forks and vendored modifications

Existing downstream forks are not automatically justified forever. When they are touched, upgraded, superseded or materially reviewed, re-evaluate each custom family under this policy.

Do not delete useful historical work, provenance, tests, hostile vectors or research merely because an active fork is removed. Preserve them as evidence/reference so a proven requirement can later reuse the smallest validated mechanism without restoring an entire historical fork.

## Security and correctness

Prefer upstream security fixes and supported cryptographic implementations over maintaining broad local copies. A downstream patch may improve a specific resource-safety, availability or policy property without improving cryptographic correctness; describe the actual property rather than making a broader security claim.

Do not remove a downstream mechanism solely to reduce maintenance if doing so would silently weaken an accepted invariant. If the project intentionally changes the invariant or threat model, make that an explicit reviewed architecture/contract decision rather than disguising it as dependency cleanup.

## Project development and qualification

Build the real Oteryn product path as early as practical and use it to qualify dependency choices under representative workloads. Keep focused correctness/adversarial tests, but do not overfit the architecture to hypothetical production behavior when real gameplay, networking, persistence and recovery measurements can provide stronger evidence.

For each dependency customization considered or retained, the preferred decision statement is:

- `UPSTREAM_SUFFICIENT` — no downstream patch required;
- `OTERYN_LAYER_FIX` — solved above the dependency;
- `MINIMAL_PATCH_REQUIRED` — exact demonstrated gap and smallest patch identified;
- `FORK_REQUIRED` — only after smaller options are proven insufficient;
- `UNKNOWN` — evidence is incomplete; do not invent a justification.

## Scope

This policy applies repository-wide to third-party libraries, runtimes, engines, protocols, storage/client libraries, cryptographic stacks and other mature external components consumed by Oteryn. It does not prohibit Oteryn-owned product logic, domain models or intentionally proprietary/project-specific components where upstream equivalence is not the design goal.
