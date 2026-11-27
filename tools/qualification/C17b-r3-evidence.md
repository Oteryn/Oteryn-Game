# C17b r3 retained qualification evidence

Tested head: `6c535a218436a12e0dd6df17070b604b4fc07b7b`. The four node logs contain qualification-profile info events for the placed Rat. No separate swing-skip event name appears for these identities. The melee verdicts include the logged skip reasons.

Position provenance: the row logs record the caster position at Rat placement. The pre-cast wait sends no movement commands. Server AI events carry a target handle and timestamp, with no actor coordinates. The placement position is the last direct caster position evidence during the three zero-hit windows. Exact per-tick positions, including post-disconnect motion and changes of target, cannot be recovered from these logs. All matching events are included below, even after the row exits.

## client-1

Placed Rat `edf97c3588a282040b6a261ee315d3e8/1`. The position observations and row verdicts below precede the unmodified AI event stream.

```text
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b.client caster=TilePos { x: 32369, y: 32230, floor: -7 } observer=TilePos { x: 32369, y: 32231, floor: -7 }
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b caster=TilePos { x: 32369, y: 32230, floor: -7 } rat=EntityRef { identity: [237, 249, 124, 53, 136, 162, 130, 4, 11, 106, 38, 30, 227, 21, 211, 232], generation: 1 }
SEAM_EVIDENCE e2e_all scenario=C17b.client status=FAIL seconds=0.000 evidence=FIXTURE_PLACEMENT_C17b_Rat_timeout_exact_rat=EntityRef_{_identity:_[237,_249,_124,_53,_136,_162,_130,_4,_11,_106,_38,_30,_227,_21,_211,_232],_generation:_1_}_last_tile=Some(TilePos_{_x:_32369,_y:_32229,_floor:_-7_})_saw_adjacent=true_attributed_hit=0_health_before=320_health_after=320 cause=assertion root=C17b.client/walk
SEAM_EVIDENCE e2e_all scenario=C17b.client replay=e2e_c17b_invisibility_keeps_player_visible_and_rejects_duplicate_frame status=FAIL cause=capture-missing
NODE_BOOT_RESULT=NODE_BOOT_FAIL mode=unprivileged
oteryn-game-server ts=1795755591650 level=info module=node event=character_bootstrap trace=01a21b5f-c498-71b6-94f5-0d5452d18a56 detail="result=spawned edf97c3588a282040b6a261ee315d3e8 1"
oteryn-game-server ts=1795755622160 level=warn module=gameplay_transport event=serve_end trace=01a21b5f-4bf9-7fb2-b9bb-da98d8effb8c reason=disconnected phase=frame_error source=crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs:506:32 next_command_id=10 server_sequence=46
```

```text
oteryn-game-server ts=1795755594562 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=nearest_eligible
oteryn-game-server ts=1795755594958 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755618001 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755618422 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755622516 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755623154 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755623632 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755624027 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755624736 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755625288 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755625901 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755626270 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795755627194 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755627629 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755628321 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755628751 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755630091 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755630571 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755631145 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755631570 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755632302 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755632741 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755633444 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755633854 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755634613 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755635027 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755635776 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755636239 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755636926 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755637351 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755638157 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755638630 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755639299 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755639719 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755640463 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755640901 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755641647 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755642075 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755642836 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755643265 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755643993 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755644431 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755645153 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755645567 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755646303 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755646773 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755647480 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755647887 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=landed reason=committed_damage
oteryn-game-server ts=1795755648662 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755649060 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755650231 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755650658 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755651345 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755651824 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755652468 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755652889 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755653708 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755654193 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755654830 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755655237 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755655977 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755656438 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755657143 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755657564 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755658352 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755658807 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755659519 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755660117 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=landed reason=committed_damage
oteryn-game-server ts=1795755660802 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755661229 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755662079 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755662486 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755663076 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755663510 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755664309 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755664738 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755665501 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755665922 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755666660 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755667076 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755667915 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755668508 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755669046 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755669510 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755670577 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755670974 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755671807 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755672243 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755673013 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755673583 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755674205 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755674643 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755675379 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755675828 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755676604 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755676997 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755677859 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755678273 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755679057 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755679466 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755680237 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755681101 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755681720 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755682130 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755683276 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755683763 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755684332 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755684741 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755685529 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755685988 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755686705 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755687379 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755687878 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755688310 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755689033 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755689645 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755690203 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755690615 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755691635 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755692052 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755692914 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755693347 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755694083 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755694536 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755695238 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755695672 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755696489 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755696926 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795755697645 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755698060 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795755698978 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11497/3 reason=retained
oteryn-game-server ts=1795755699463 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=blocked reason=no_damage_after_mitigation
```

## client-2

Placed Rat `13a3de4320a003b642ccf86aef3f2548/1`. The position observations and row verdicts below precede the unmodified AI event stream.

```text
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b.client caster=TilePos { x: 32369, y: 32230, floor: -7 } observer=TilePos { x: 32369, y: 32231, floor: -7 }
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b caster=TilePos { x: 32369, y: 32230, floor: -7 } rat=EntityRef { identity: [19, 163, 222, 67, 32, 160, 3, 182, 66, 204, 248, 106, 239, 63, 37, 72], generation: 1 }
SEAM_EVIDENCE e2e_all scenario=C17b.client status=FAIL seconds=0.000 evidence=FIXTURE_PLACEMENT_C17b_Rat_timeout_exact_rat=EntityRef_{_identity:_[19,_163,_222,_67,_32,_160,_3,_182,_66,_204,_248,_106,_239,_63,_37,_72],_generation:_1_}_last_tile=Some(TilePos_{_x:_32369,_y:_32229,_floor:_-7_})_saw_adjacent=true_attributed_hit=0_health_before=320_health_after=320 cause=assertion root=C17b.client/walk
SEAM_EVIDENCE e2e_all scenario=C17b.client replay=e2e_c17b_invisibility_keeps_player_visible_and_rejects_duplicate_frame status=FAIL cause=capture-missing
NODE_BOOT_RESULT=NODE_BOOT_FAIL mode=unprivileged
oteryn-game-server ts=1795757194478 level=info module=node event=character_bootstrap trace=01a21b78-3b5a-7c0f-a187-c754a05e0f8e detail="result=spawned 13a3de4320a003b642ccf86aef3f2548 1"
oteryn-game-server ts=1795757224731 level=warn module=gameplay_transport event=serve_end trace=01a21b77-c9fe-73ba-9f07-d577866a7892 reason=disconnected phase=frame_error source=crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs:506:32 next_command_id=8 server_sequence=40
```

```text
oteryn-game-server ts=1795757198861 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=nearest_eligible
oteryn-game-server ts=1795757199275 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757224859 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757225199 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757226020 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757226379 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757227190 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757227577 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757228353 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757228724 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757229529 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757229933 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757230662 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757231050 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757231835 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757232239 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757233327 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757233752 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757234463 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757234842 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757235615 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757236053 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757236822 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757237294 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757238028 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757238397 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757239160 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757239654 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757240284 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757240660 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757241443 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757241838 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757242593 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757243042 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757243914 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757244336 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757245059 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757245553 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757246202 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757246598 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757247341 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757247724 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757248495 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757248877 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=landed reason=committed_damage
oteryn-game-server ts=1795757249644 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757250012 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757250815 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757251310 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757251968 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757252362 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757253603 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757254139 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757254824 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757255259 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757256100 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757256533 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757257268 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757257660 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757258430 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757258820 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757259613 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757260180 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757260820 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757261295 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757261987 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757262528 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757263598 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757264002 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757264682 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757265113 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757265868 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757266393 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757267063 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757267443 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757268224 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757268790 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757269410 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757269801 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757270852 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757271246 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757272079 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757272581 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757273587 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757274045 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757274803 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757275199 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757275975 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757276424 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757277127 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757277534 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757278281 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757278691 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=zero_roll
oteryn-game-server ts=1795757279438 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757279819 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757280580 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757281079 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757281734 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757282171 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757282902 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757283298 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757284092 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757284473 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757285351 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757285735 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757286494 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757286880 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757287612 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757287977 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757288778 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757289218 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757289938 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757290360 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757291099 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757291537 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757292288 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757292799 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757294028 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757294426 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757295150 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757295568 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757296302 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757296732 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757297467 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757297870 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795757298602 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757299029 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795757299732 level=info module=monster_ai event=target_decision monster=13a3de4320a003b642ccf86aef3f2548/1 target=11494/3 reason=retained
oteryn-game-server ts=1795757300132 level=info module=monster_ai event=melee_drain monster=13a3de4320a003b642ccf86aef3f2548/1 outcome=landed reason=committed_damage
```

## full-1

Placed Rat `4c2514b929dcc9d1f515132092a62883/2`. The position observations and row verdicts below precede the unmodified AI event stream.

```text
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b caster=TilePos { x: 32490, y: 32169, floor: -7 } rat=EntityRef { identity: [76, 37, 20, 185, 41, 220, 201, 209, 245, 21, 19, 32, 146, 166, 40, 131], generation: 2 }
SEAM_EVIDENCE C17B_GAP step-failure observed=FIXTURE_PLACEMENT C17b Rat timeout exact_rat=EntityRef { identity: [76, 37, 20, 185, 41, 220, 201, 209, 245, 21, 19, 32, 146, 166, 40, 131], generation: 2 } last_tile=Some(TilePos { x: 32489, y: 32169, floor: -7 }) saw_adjacent=true attributed_hit=0 health_before=147 health_after=90
SEAM_EVIDENCE e2e_all scenario=C17b.full status=FAIL seconds=670.916 evidence=runtime-step-failure-see-C17B_GAP cause=assertion root=C17b.full/walk
NODE_BOOT_RESULT=NODE_BOOT_FAIL mode=unprivileged
oteryn-game-server ts=1795756695965 level=info module=node event=character_bootstrap trace=01a21b70-a039-7a2a-a2e5-05e33b1fc4c0 detail="result=spawned 4c2514b929dcc9d1f515132092a62883 2"
oteryn-game-server ts=1795756726399 level=warn module=gameplay_transport event=serve_end trace=01a21b66-ff44-70d0-8b1c-e9c7870840bf reason=disconnected phase=visibility source=crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs:506:32 next_command_id=207 server_sequence=717
```

```text
oteryn-game-server ts=1795756703739 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=nearest_eligible
oteryn-game-server ts=1795756704188 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756708844 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756709230 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756710125 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756710509 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756720217 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756720587 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756726537 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756726914 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756727848 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756728504 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756729303 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756729763 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756730673 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756731145 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756731938 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756732319 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756733205 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756733706 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756734634 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756735002 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756735932 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756736319 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756737223 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756737631 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756738550 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756738937 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=zero_roll
oteryn-game-server ts=1795756739839 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756740214 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756740849 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756741230 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756742409 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756742794 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756743623 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756743988 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=zero_roll
oteryn-game-server ts=1795756744888 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756745276 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756746070 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756746429 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756747285 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756747677 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756748573 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756748933 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756749717 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756750101 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756750884 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756751242 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756752246 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756752640 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=missed reason=no_native_target
oteryn-game-server ts=1795756753605 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=11498/1 reason=retained
oteryn-game-server ts=1795756753974 level=info module=monster_ai event=melee_drain monster=4c2514b929dcc9d1f515132092a62883/2 outcome=blocked reason=no_damage_after_mitigation
oteryn-game-server ts=1795756754949 level=info module=monster_ai event=target_decision monster=4c2514b929dcc9d1f515132092a62883/2 target=none reason=no_visible_players
```

## full-2

Placed Rat `edf97c3588a282040b6a261ee315d3e8/1`. The position observations and row verdicts below precede the unmodified AI event stream.

```text
SEAM_EVIDENCE FIXTURE_PLACEMENT C17b caster=TilePos { x: 32490, y: 32169, floor: -7 } rat=EntityRef { identity: [237, 249, 124, 53, 136, 162, 130, 4, 11, 106, 38, 30, 227, 21, 211, 232], generation: 1 }
SEAM_EVIDENCE FIXTURE C17b attributed_rat_damage sequence=792 health_before=250 health_after=214
SEAM_EVIDENCE C17B_GAP step-failure observed=C17b no eligible return route from TilePos { x: 32536, y: 32514, floor: -7 } to TilePos { x: 32369, y: 32238, floor: -7 }
SEAM_EVIDENCE e2e_all scenario=C17b.full status=FAIL seconds=1083.527 evidence=runtime-step-failure-see-C17B_GAP cause=assertion root=C17b.full/walk
NODE_BOOT_RESULT=NODE_BOOT_FAIL mode=unprivileged
oteryn-game-server ts=1795758297356 level=info module=node event=character_bootstrap trace=01a21b89-0ebf-7845-baa8-e1ce9b431a82 detail="result=spawned edf97c3588a282040b6a261ee315d3e8 1"
oteryn-game-server ts=1795758729031 level=warn module=gameplay_transport event=serve_end trace=01a21b7f-4000-7d7f-a5b1-5059c770598d reason=disconnected phase=frame_error source=crates/gameplay/dispatch/src/gameplay_transport/connection/serve/mod.rs:506:32 next_command_id=207 server_sequence=846
```

```text
oteryn-game-server ts=1795758321724 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11494/3 reason=nearest_eligible
oteryn-game-server ts=1795758322179 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758322756 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11494/3 reason=retained
oteryn-game-server ts=1795758323169 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=landed reason=committed_damage
oteryn-game-server ts=1795758340425 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=nearest_eligible
oteryn-game-server ts=1795758340804 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758355331 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758355715 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758371140 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758371545 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758378977 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758379397 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758391282 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758391666 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758408378 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758408738 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758427986 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758428415 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758437113 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758437567 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758455821 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758456206 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758495409 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758495814 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758545383 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758545775 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758576832 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758577251 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758635377 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=11495/1 reason=retained
oteryn-game-server ts=1795758635790 level=info module=monster_ai event=melee_drain monster=edf97c3588a282040b6a261ee315d3e8/1 outcome=missed reason=no_native_target
oteryn-game-server ts=1795758676466 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
oteryn-game-server ts=1795758698070 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
oteryn-game-server ts=1795758719249 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
oteryn-game-server ts=1795758724107 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
oteryn-game-server ts=1795758725623 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
oteryn-game-server ts=1795758726453 level=info module=monster_ai event=target_decision monster=edf97c3588a282040b6a261ee315d3e8/1 target=none reason=no_visible_players
```

The final cleanup position was (32536, 32514, -7). The node separately logged that origin on a map snapshot. That later origin does not identify the player position at earlier target decisions.

```text
oteryn-game-server ts=1795758645441 level=warn module=gameplay_transport event=map_view_snapshot trace=01a21b7f-4000-7d7f-a5b1-5059c770598d revision=208 tiles=317 items=517 origin_x=32536 origin_y=32514 origin_floor=7 first_visible_floor=0 origin_tile=true
```

## Focused PostgreSQL fixture failure

```text
Error: "\"ItemReference\""
```

The error propagates from the fixture bundle boot call in `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/rat_target_retention.rs:348` through the error mapping at line 363. Line 316 changes the palette key to `oteryn:item.tibia.i100`. The palette item reference must match the native item index resolved at line 361. The refusal does not name the individual palette entry.
