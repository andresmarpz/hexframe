use super::*;

#[test]
fn the_bundled_table_reads_and_has_every_row() {
    assert_eq!(table().modes.len(), 25);
    assert_eq!(table().maps.len(), 17);
}

#[test]
fn modes_are_found_ignoring_case() {
    assert_eq!(mode_name("CLASSIC"), Some("Summoner's Rift"));
    assert_eq!(mode_name("aram"), Some("ARAM"));
    assert_eq!(mode_name("KIWI"), Some("ARAM Mayhem"));
    assert_eq!(mode_name("CHERRY"), Some("Arena"));
    assert_eq!(mode_name("ONEFORALL"), Some("One for All"));
    assert_eq!(mode_name("NOT_A_MODE"), None);
}

#[test]
fn maps_are_found_by_id() {
    assert_eq!(map_name(11), Some("Summoner's Rift"));
    assert_eq!(map_name(12), Some("Howling Abyss"));
    assert_eq!(map_name(30), Some("Rings of Wrath"));
    assert_eq!(map_name(999), None);
}

#[test]
fn classic_structure_ids_decode_to_side_lane_and_tier() {
    let name = |id: &str| structure_name(id, Some(11));
    assert_eq!(
        name("Turret_T1_R_03_A").as_deref(),
        Some("Blue bot outer turret")
    );
    assert_eq!(
        name("Turret_T2_L_02_A").as_deref(),
        Some("Red top inner turret")
    );
    assert_eq!(
        name("Turret_T2_L_01_A").as_deref(),
        Some("Red top inhibitor turret")
    );
    assert_eq!(
        name("Turret_T1_C_05_A").as_deref(),
        Some("Blue mid outer turret")
    );
    assert_eq!(
        name("Turret_T1_C_04_A").as_deref(),
        Some("Blue mid inner turret")
    );
    assert_eq!(
        name("Turret_T2_C_03_A").as_deref(),
        Some("Red mid inhibitor turret")
    );
    assert_eq!(
        name("Turret_T2_C_01_A").as_deref(),
        Some("Red nexus turret")
    );
    assert_eq!(name("Barracks_T2_L1").as_deref(), Some("Red top inhibitor"));
    assert_eq!(
        name("Barracks_T1_C1").as_deref(),
        Some("Blue mid inhibitor")
    );
    assert_eq!(
        name("Barracks_T1_R1").as_deref(),
        Some("Blue bot inhibitor")
    );
}

#[test]
fn howling_abyss_structure_ids_decode_to_side_and_tier() {
    let name = |id: &str| structure_name(id, Some(12));
    assert_eq!(
        name("Turret_TChaos_L1_P3_671196429_0").as_deref(),
        Some("Red outer turret")
    );
    assert_eq!(
        name("Turret_TChaos_L1_P2_620863572_0").as_deref(),
        Some("Red inner turret")
    );
    assert_eq!(
        name("Turret_TOrder_L1_P4_587308334_0").as_deref(),
        Some("Blue nexus turret")
    );
    assert_eq!(
        name("Turret_TChaos_L1_P5_637641191_0").as_deref(),
        Some("Red nexus turret")
    );
    assert_eq!(
        name("Inhib_TChaos_L1_P1_2432143522_0").as_deref(),
        Some("Red inhibitor")
    );
}

#[test]
fn modern_turrets_without_tier_rules_name_only_the_side() {
    assert_eq!(
        structure_name("Turret_TOrder_L1_P3_671196429_0", Some(11)).as_deref(),
        Some("Blue turret")
    );
    assert_eq!(
        structure_name("Turret_TOrder_L1_P3_671196429_0", None).as_deref(),
        Some("Blue turret")
    );
}

#[test]
fn unknown_structure_ids_have_no_name_but_a_readable_label() {
    assert_eq!(structure_name("Turret_T9_Q_03_A", Some(11)), None);
    assert_eq!(structure_name("Turret_T1_Q_03_A", Some(11)), None);
    assert_eq!(structure_name("Turret_T1_L_09_A", Some(11)), None);
    assert_eq!(structure_name("Nexus_T1", Some(11)), None);
    assert_eq!(structure_label("Turret_Odd", None), "Turret");
    assert_eq!(structure_label("Inhib_TOrder_X", None), "Blue inhibitor");
    assert_eq!(structure_label("Barracks_T2_Z9", None), "Red inhibitor");
    assert_eq!(structure_label("Wall_7", None), "Structure");
}

#[test]
fn structure_ids_are_told_apart_from_players_and_minions() {
    assert!(is_structure_id("Turret_TChaos_L1_P3_671196429_0"));
    assert!(is_structure_id("Inhib_TChaos_L1_P1_2432143522_0"));
    assert!(is_structure_id("Barracks_T1_C1"));
    assert!(!is_structure_id("Minion_T100L0S1"));
    assert!(!is_structure_id("Zed"));
}

#[test]
fn a_structure_shape_ignores_the_numeric_ids() {
    assert_eq!(
        shape("Turret_TChaos_L1_P3_671196429_0"),
        shape("Turret_TChaos_L1_P3_620863572_0")
    );
    assert_eq!(
        shape("Turret_TChaos_L1_P3_671196429_0"),
        "Turret_TChaos_L1_P3"
    );
}
