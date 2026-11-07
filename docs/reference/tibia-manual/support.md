# Tibia manual notes: support

- Source: <https://www.tibia.com/gameguides/?subtopic=manual&section=support>
- Capture: owner, 2026-09-28 (tibia.com blocks cloud containers and CI runners)
- Clean-text SHA-256: `089570618130f9005c37e0cdec4dbda6f2d0999083148f3f333989a2aeea6816`
- Full private text: Jira `KAN-33`, attachment `tibia-manual-2026-09-28.txt`
- Evidence class: `CIPSOFT_OFFICIAL` / `PRIMARY_OFFICIAL`
- Domain: Platform / web, not Game runtime; kept for completeness
- These are Oteryn-written notes, not the manual text. Cite as `tibia.com manual §support <heading>, capture 2026-09-28`.

## 8.1.1 Ingame Help

**Client Help** [client]: Activate via Help menu or Ctrl+H; highlights interface elements (magnifying-glass cursor), displays info on hover, deactivates on click [client]

**Help Channel** [client]: Community-moderated channel for gameplay/account/website questions; access via channel dialog (Ctrl+T) [client]

## 8.1.2 Offline Help

**Manual** [platform]: Comprehensive reference covering all game aspects [platform]

**Quickstart** [platform]: Beginner-focused step-by-step introduction (account creation, controls, basics) [platform]

**FAQ** [platform]: Quick reference for common problems and troubleshooting [platform]

**Tibia Rules** [platform]: Code of conduct with detailed commentary per rule [platform]

**Security Hints** [platform]: Preventative account protection guidance [platform]

**Lost Account Interface** [platform]: Self-service password recovery and account access restoration [platform]

**Library** [platform]: Reference pages for spells, creatures, maps, world history [platform]

**Technical Support Board** [platform]: Player-to-player troubleshooting for edge-case problems [platform]

**News** [platform]: Announcements for updates, issues, important announcements [platform]

**Company Information** [platform]: CipSoft company details, legal information [platform]

**Customer Support Tickets** [platform]: Last-resort support channel for unsolved individual problems; categories: Technical Problems, Payments, General. Support viewed as helpdesk, not proposal/feedback venue [platform]

## 8.2 Rule Enforcement: Principles

**Report-based system** [platform]: Players report violations; CipSoft customer support decides guilt and punishment level [platform]

**Punishment progression** [platform]: Repeat offenders receive harsher punishments for same violations; minor first-time violations can escalate to account deletion with repeated violations [platform]

## 8.2 Rule Enforcement: Objectives

Protect players from unfair or inappropriate behavior; not all unfriendly conduct violates rules (stealing and PvP kills permitted on applicable worlds) [platform]. Players must balance rule compliance against personal interest [platform]

## 8.2 Reporting Rule Violations

Channels: console/item-writable statements, character names, character/guild page comments, forum posts/signatures, undisclosed software usage (bots) [platform]. All reports reviewed by customer support [platform]

## 8.2.1 Rule Violation Record and Conduct Level

**Entry categories** [platform]: Very Mild (warning), Mild, Moderate, Severe, Very Severe (immediate deletion) [platform]

**Effects** [platform]: Vary by severity and history; examples: Mild → no effect if first, ban/delete if repeated [platform]

**Multiple violations** [platform]: Consecutive bans accumulate time (7 days + 7 days = 14 days total) [platform]

**Acknowledgment** [platform]: Player must actively click "Acknowledge" button; blocks game/forum access until done [platform]

**Complaints** [platform]: 60-day appeal window from entry date; reviews upon submission; removal if unjustified [platform]

**Conduct Level Indicator** [platform]: Visual color (green=clean, yellow→orange→red→black=severe); darker color = higher likelihood of harsher punishment on next violation; decreases over time if compliant [platform]

## 8.2.2 Punishments

**Entries** [platform]: Very Mild/Mild entries allow continued play post-acknowledgment; repeated entries escalate to banishment/deletion [platform]

**Namelocks** [platform]: Character receives random name post-acknowledgment; player proposes new name (3 attempts max); invalid proposals result in permanent random name retention [platform]. Serious offenses may incur additional banishment [platform]

**Account Banishments** [platform]: Temporary exclusion (2 or 7 days standard) from game and forum affecting entire account [platform]. Premium Time lost during banishment not compensated [platform]

**IP-Banishments** [platform]: Rare; blocks entire IP address temporarily [platform]

**Account Deletions** [platform]: Permanent removal for repeated/severe violations; no acknowledgment required but 60-day appeal window honored [platform]. Remaining Premium Time and Tibia Coins lost and not compensated [platform]

---

## Open questions for Oteryn

- Should Oteryn implement bot-detection automation, or rely solely on player reports?
- How should Oteryn handle timezone-independent punishment timing (server saves vs. player-local time)?
- Will Oteryn support appeal workflows, or implement automated/human review?
- Should namelock violations auto-rename immediately or provide player grace period?
- How should IP-banishments interact with shared residential networks (colleges, offices, VPNs)?
