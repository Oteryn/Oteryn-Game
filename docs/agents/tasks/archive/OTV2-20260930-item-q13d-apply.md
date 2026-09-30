# OTV2-20260930-item-q13d-apply

**Task ID:** OTV2-20260930-item-q13d-apply

**Status:** COMPLETED

**Issue:** #162

**Date:** 2026-09-30

## Objective

Apply owner decision D165 (Q13d disputed item families, 41 ids) to the ITEM-ID-1b register and `owner-item-family-decisions.json`, and add the route reason `non_pickupable_blocking_prop`.

## Execution

- 3 bound ids (54262, 23547, 21212, `quest_item`) added to `owner-item-family-decisions.json`; 37 client-only ids stay `PENDING_MINT` (44044 is `EXCLUDED`, D165 26a).
- Id 9132 routes `WorldObject` / `non_pickupable_blocking_prop` in `engine_items.non_item_route`, with a test.
- Register rows moved to status `APPROVED_OWNER_D165`. All profile names already exist in `profile-catalog.json`.

## Closeout

PR: see the PR for this branch. Review: none required. Merge result: squash merge of the PR.
