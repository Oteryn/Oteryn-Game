---
name: frontend-design
description: Optional visual design for a new Oteryn screen or intentional redesign, only after explicit user authorization to use this skill or select relevant optional skills for the current task. Not for routine fixes, backend work or performance diagnosis.
---

# Oteryn frontend design

Check the current-task authorization in root AGENTS.md before using the procedure. Without a grant, continue ordinary work without this skill.

After authorization, read `references/upstream.md` for the pinned Anthropic design procedure. Apply it within the requested scope and existing Oteryn architecture:

- Read the relevant sections of `docs/architecture/OTERYN_NATIVE_CLIENT_UI_ARCHITECTURE_BASELINE_2026-09-10.md` for client work and ADR-0005 for Studio concepts.
- The client is native Rust/wgpu. Web CSS, browser animation and responsive-page examples are conceptual references, not permission to introduce a web renderer or framework.
- Preserve retained UI identity, input/focus routing, docking and the separation between world and UI rendering. Do not invent a parallel widget system.
- Prioritize gameplay readability, dense information, pixel-art clarity and viewport space. Follow established design before proposing a new visual language.
- Use existing capture/preview tools when available. Label mockups as mockups; do not claim screenshots prove native implementation or performance.
- Do not add packages, fonts, remote services or asset sources just because a design example uses them. Existing task scope and provenance rules still apply.

The wrapper is Oteryn-owned (MPL-2.0); the unmodified upstream reference and its license are in `references/`.
