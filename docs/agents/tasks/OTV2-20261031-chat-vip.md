# OTV2-20261031-chat-vip

Status: AUTHORING
Branch: chatgpt/chat-vip
Base: ae4188a9c8d461acb152044814ea23075de02ed4
Reference: Crystal 201189f871d23597b82caf88fd611454a2ddefb8
Comparison: Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446

## Authorized scope

Complete S04a VIP presence across the World, S04b private-channel cross-Channel
invitations and cross-node PM proof, S04c missing public channels and Help
moderation, and S19 spectator typing indication. Extend the existing owners and
retain session-generation fencing and accepted VIP consent/privacy rules.
Keep the pull request in draft. No main merge, rebase, force push or review request.

## Allocation

Reserved migrations: 0716-0719.
Reserved capabilities: 258-259.
Reserved commands: 486-489.
Reserved mechanic ordinals and offer orders: 626-629.
Existing command and capability owners are reused where compatible.

## Execution

One task branch with disjoint VIP, public-channel, typing and private-channel
implementation slices. The task lead integrates shared registrations and runs
serialized builds. Source evidence and executed checks are recorded in the PR.
Questions that source and existing contracts cannot settle use one QUESTION:
comment per decision on that PR; ANSWER: comments are read before each new step.

## Verification

Pending implementation: source-mapped rule and refusal tests, mounted PostgreSQL
dispatch/read-back cases, protocol and client compatibility checks, architecture
validation, formatting, changed-package clippy/tests and diff review.
No implementation or test completion is claimed by this initial task record.
