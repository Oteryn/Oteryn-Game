N=/opt/node22/bin/node; X=/home/user/.local/bin/oteryn-cloud.mjs
while read t id; do s=$($N $X status $id 2>/dev/null | jq -r '[.recentTurns.data.values[]|.status]|.[0]' 2>/dev/null); echo "$t $s"; done <<'L'
questpc 01a1b322-dcf9-7009-af25-c8f8e528e0c9
questpc8 01a1b322-e7d2-74ce-b47b-2f7bfe7ce0f1
questl2 01a1b322-f393-708e-8a7f-c01a94c7a0e3
questpc3 01a1b323-0835-71f4-9563-ff45c1cd40e6
npcfix 01a1b323-1aac-71ad-a9ac-dd136f1610f4
qengine2 01a1b584-d9dc-726e-befc-af31797eb0ed
questpc2 01a1b322-fefb-7237-bd3d-1a2a584040ea
questpc4 01a1b323-10d0-7716-92b6-c36e1fa710ea
questpc5 01a1b2cc-c883-71de-8db0-a600017220cc
questpc7 01a1b2cc-dd84-76df-bf23-a3117aa130dc
srv7 01a1b018-042e-70da-926a-17bf81a200e6
boss2 01a1b003-0214-705f-8c50-5b65940160c3
qready1 01a1b76b-c836-708c-83ce-034438f1c0e5
qready2 01a1b76b-d313-7536-9d92-681d481520c1
L
qworld 01a1b591-0181-77a1-8233-83f8d35070ec
qnpcfx 01a1b591-0bc2-730a-9626-9bf4e704d0d1
wpkg 01a1b595-5daf-7042-b233-ac072ba8f0c8
srvenc 01a1b59a-6a41-7131-a0f6-0cf253d9f0e5
srvwld 01a1b59a-744d-77d4-92c3-b08078daa0d9
clcast 01a1b59a-8758-71f7-8e84-e85a4c1a60ce
soulpit 01a1b5ac-0e76-73a6-9cc3-22eaa29a50f2
encx 01a1b5ac-18bf-749e-b55a-62c6f60400d5
season 01a1b5ac-22c2-729b-903b-264ad09b40e7
qengpr 01a1b58e-0e9f-745d-bf16-ed899bd270cb
pubproj 01a1b642-355a-73d8-a7fc-a340ba8230df
catalog 01a1b646-c0ec-714c-b819-353b7d2700cd
qaudit 01a1ba57-e7f5-7033-91db-621cf7ab00e6
aud1108 01a1bb56-b99a-724d-a943-5cf3a5c2e0f0
mainred1108 01a1bb57-85ed-7041-95e7-56405fb510ff
mainred1108 01a1bb57-85ed-7041-95e7-56405fb510ff
otcaud1108 01a1bb58-1a74-71cf-aaa2-1c3e1bb2b0d2
otcaud1108 01a1bb58-1a74-71cf-aaa2-1c3e1bb2b0d2
npcfix2 01a1bb61-d854-73cd-8390-addd3d21c0d1
npcfix2 01a1bb61-d854-73cd-8390-addd3d21c0d1
engnpc 01a1bb62-bc09-74ae-9fae-7822e08730fa
01a1bb62-bc09-74ae-9fae-7822e08730fa
engenc 01a1bb62-c648-7162-aed5-2d55ab6d30e8
01a1bb62-c648-7162-aed5-2d55ab6d30e8
engitem 01a1bb62-d0d0-760e-9cea-6be2b60640f6
01a1bb62-d0d0-760e-9cea-6be2b60640f6
mainred1108b 01a1bb66-d153-7334-9672-49ae0ffa30c8
01a1bb66-d153-7334-9672-49ae0ffa30c8
srvgap1108 01a1bb6d-02ea-7669-a677-81c556e5d0dc
01a1bb6d-02ea-7669-a677-81c556e5d0dc
engnpcr2 01a1bb70-21cd-71c2-b4dc-9009e3f570dd
01a1bb70-21cd-71c2-b4dc-9009e3f570dd
engencr2 01a1bb70-2bf3-725c-b198-fa79ca7700cc
01a1bb70-2bf3-725c-b198-fa79ca7700cc
engitemr2 01a1bb70-3953-75d5-b8f9-4a46c25780e8
01a1bb70-3953-75d5-b8f9-4a46c25780e8
npcfix2r2 01a1bb70-4471-76b5-8744-b3c5574a30f5
01a1bb70-4471-76b5-8744-b3c5574a30f5
npcfix2r3 01a1bb84-a6b8-7450-b819-73b00a77f0c6
01a1bb84-a6b8-7450-b819-73b00a77f0c6
seasonfix 01a1bb8a-6058-70b5-8b91-c6b513cb30c9
cg_options 01a1bb9d-2553-70a5-acab-fc26beee60d1
cg_cyclo 01a1bb9d-2ff6-71a7-9a63-934a917070f8
cg_analytics 01a1bb9d-3a34-70d0-b8da-168cd4fc20dd
cg_ui 01a1bb9d-439f-7079-96fc-749c169da0f1
cg_account 01a1bb9d-4e2c-72be-987c-466eced1b0d4
srvgap2 01a1bbb0-dd3b-7759-903f-8c57febb40e5
01a1bbb0-dd3b-7759-903f-8c57febb40e5
engenc2 01a1bbe4-e0c6-77e5-b3bd-27249e70e0eb
mainrednb 01a1bbf2-0216-746c-82c0-aef672efa0e9
cinbpr 01a1bbfc-d2b4-7264-a78e-e7a2ba7f60d4
engnpc2 01a1bc15-292f-7593-a724-45ddc47bd0fb
engoutfit 01a1bc39-9815-748a-afb5-daf03afd50d4
engclaim 01a1bc39-a32a-73f6-9e03-095333eae0f0
engworldchg 01a1bc39-ae73-70dd-944e-58cc370600ea
qn1 01a1bcb8-d075-76bd-a252-e3347ecd30d0
qn2 01a1bcb8-dbf9-7174-bc86-b780c64b00cb
qn3 01a1bcb8-e7b2-7111-b8ff-d56c9b30f0c8
qn4 01a1bcb8-f169-770a-90cd-52c491abb0e3
qn5 01a1bcb8-fc3c-70e6-b470-83453e4270ec
qn6 01a1bcb9-0698-7372-ae44-d85ced8dd0c6
qnpcfx 01a1b591-0bc2-730a-9626-9bf4e704d0d1
qmmm 01a1bcdc-26e6-7783-9575-a216c81e80f7
qm49 01a1bced-9bfe-726b-84c5-5a930d5ed0ca
qm50 01a1bced-a790-727b-a49d-856bd77d50c2
qm54 01a1bced-b26e-7259-a421-a653b04af0fb
qm03 01a1bced-bde9-74b9-8ffb-b786984bf0e4
qm46 01a1bced-c7a5-726c-9e54-cce1189680ff
qm22 01a1bced-d125-73d3-9f43-e9bf540e60cd
qm56 01a1bced-da7c-736a-b543-33d5bf72d0d7
qm47 01a1bced-e42c-736f-bebe-4980c8b4e0f5
qm51 01a1bced-eeff-777a-8688-3567ee0cb0f7
qm48 01a1bced-f9c5-7507-bc3a-e3c02c3ef0dd
cg-opt1 01a1bcef-a170-738a-8837-23e7d09920e9
cg-opt2 01a1bcef-acaf-70e4-baa0-7c8c0e3380dc
cg-gamewin 01a1bcef-b82b-7061-bc14-bfcb98c480fc
cg-cyclo2 01a1bcef-c2ce-7213-baa9-1fb9fa9380e3
cg-trackers 01a1bcef-ccf5-73d9-9494-4bc9dacc20ca
cg-house 01a1bcef-d80a-700a-8c00-9e8c283350ef
cg-login 01a1bcef-e34b-741e-a004-0e60e91ee0d4
cg-modal 01a1bcef-ecdf-7436-881b-60cd715de0d7
cg-social 01a1bcef-f7a5-72e6-a1bf-5b73223d90cd
