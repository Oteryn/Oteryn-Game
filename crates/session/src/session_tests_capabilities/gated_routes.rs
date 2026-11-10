use super::*;

#[test]
fn derived_gated_routes_match_previous_table() -> Result<(), &'static str> {
    // Former routes plus PKG5 hotkeys and CP01 property/service commands.
    // Compare sets because
    // the protocol registry generates its entries in capability order.
    let expected: &[(u32, &[u32], &[u32])] = &[
        (3, &[7, 8], &[7, 8]),
        (7, &[13, 6117], &[12]),
        (17, &[11, 12, 740], &[10]),
        (182, &[354, 355, 738, 6122, 6123, 6124, 6125], &[]),
        (49, &[56, 57, 6129], &[]),
        (51, &[60, 61], &[]),
        // P-C06 routes existing timed-item, cyclopedia-map and party-skull queries/intents.
        (39, &[38], &[]),
        (57, &[72, 73, 6178], &[]),
        (134, &[155], &[]),
        (40, &[40, 41], &[]),
        (42, &[45, 46], &[]),
        (43, &[48, 49, 50], &[]),
        (53, &[64], &[]),
        (54, &[66, 67], &[]),
        (55, &[68, 69], &[]),
        // P-C04: existing move-out, scope-transfer and guildhall leadership routes.
        (59, &[76, 77], &[]),
        (86, &[102], &[51]),
        (99, &[115], &[]),
        // P-C01 already routes charm progression and expert fight modes on this base.
        (67, &[84], &[]),
        (87, &[103], &[52]),
        (60, &[78], &[26]),
        (62, &[79], &[27]),
        (63, &[80, 6126, 6128], &[28]),
        (64, &[81], &[]),
        // P-C01 already enables charm progression and PvP expert; preserve that exact table.
        (67, &[84], &[]),
        (87, &[103], &[52]),
        (68, &[85], &[]),
        (72, &[89, 6120], &[]),
        (75, &[], &[40]),
        (81, &[97], &[]),
        (82, &[98], &[]),
        // P-C03 routes the existing character-info and raid-schedule commands.
        (97, &[113], &[]),
        (149, &[171, 172, 173], &[]),
        (98, &[114], &[63]),
        (132, &[153], &[97]),
        (200, &[390, 391], &[]),
        (232, &[444, 445, 446], &[]),
        (285, &[501], &[]),
        (287, &[], &[100]),
        (115, &[124], &[]),
        // P-C05 exposes the existing 117 routes (133, 169, 178-180, 340, 491); no table change.
        // C49: the boss status panel uses the registry-owned capability 117 route.
        (117, &[133, 169, 178, 179, 180, 340, 491, 710], &[99]),
        (140, &[160], &[]),
        (165, &[220], &[]),
        (16, &[22], &[16]),
        (18, &[], &[17]),
        (19, &[], &[18]),
        (23, &[], &[19, 20]),
        (288, &[], &[101]),
        // CLCAST: native caster controls and room; Watch 289 stays account-only.
        (290, &[502, 505], &[]),
        (291, &[504], &[103]),
        (292, &[503], &[]),
        (330, &[560, 561], &[]),
        (37, &[36], &[]),
        (31, &[], &[23]),
        (25, &[25], &[21]),
        (26, &[], &[22]),
        (1, &[4, 5], &[4, 5]),
        (8, &[], &[13]),
        (9, &[14], &[]),
        (10, &[734, 735], &[15]),
        // C12 enables the existing world-interaction and item-look command routes.
        (11, &[15, 31], &[]),
        (28, &[30], &[]),
        (14, &[21], &[14]),
        (179, &[348], &[]),
        (180, &[], &[]),
        // PREY-DIFF: native Prey controls use the existing command160.
        (140, &[160], &[]),
        (147, &[170], &[]),
        // Wheel and Atelier are playable through the selected capability 148.
        (148, &[161, 308, 309, 424, 6119, 6121], &[]),
        (150, &[174, 175, 176, 739], &[]),
        // Live balances use their own selected capability without changing Atelier V1.
        (272, &[484], &[]),
        // pkg4 additions: raid server messages arrive on domain 57 under 92 zones_nologout;
        // animus mastery (168) and loyalty skills (274) query through their gated commands.
        (92, &[], &[57]),
        (168, &[232], &[]),
        (274, &[488], &[]),
        (276, &[492], &[]),
        (277, &[493], &[]),
        (278, &[494], &[]),
        (279, &[495], &[]),
        (280, &[496], &[]),
        // NX-5: item-only visuals use a separate transient stream.
        (
            350,
            &[],
            &[oteryn_protocol_oteryn::presentation::STATE_DOMAIN_ITEM_EFFECTS],
        ),
        (370, &[640], &[]),
        (381, &[662], &[]),
        (382, &[664], &[]),
        (383, &[], &[]),
        (384, &[669], &[]),
        (448, &[732], &[]),
        (450, &[736, 737, 6196], &[]),
        (479, &[6187, 6188], &[]),
        (461, &[], &[]),
        (462, &[6144, 6145], &[]),
        (463, &[6147], &[]),
        (470, &[], &[]),
        (474, &[6172, 6174], &[]),
        (449, &[733], &[]),
        (207, &[402], &[]),
        (445, &[730], &[]),
        (446, &[], &[]),
        (447, &[731], &[]),
        // CL25J: native feature adapters now use these existing gated wire routes.
        (151, &[177], &[]),
        (152, &[181, 668, 6116], &[]),
        (153, &[182], &[]),
        (154, &[183, 184, 185, 186, 6118, 6127], &[]),
        (195, &[378, 379, 380, 392, 393, 394, 395, 641, 660], &[]),
        (197, &[383, 384], &[98]),
        (199, &[387, 388], &[]),
        (225, &[431], &[]),
        (230, &[440], &[]),
        (233, &[447], &[]),
        // P-C01: native Bestiary/Charms (command 84) and PvP expert (command 103,
        // confirmed-mode domain 52) panels use their existing gated routes.
        (67, &[84], &[]),
        (83, &[], &[48]),
        (87, &[103], &[52]),
    ];
    let expected: std::collections::BTreeSet<_> = expected.iter().copied().collect();
    let derived: std::collections::BTreeSet<_> = gated_routes().collect();
    assert_eq!(derived, expected);
    Ok(())
}
