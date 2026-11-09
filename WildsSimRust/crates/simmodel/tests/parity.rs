use std::path::{Path, PathBuf};

use simmodel::*;

struct TestRoot {
    path: PathBuf,
}

impl Drop for TestRoot {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn copy_dir(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let file_type = entry.file_type().unwrap();
        if file_type.is_dir() {
            copy_dir(&entry.path(), &target.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}

fn setup() -> (TestRoot, Simulator) {
    let root = TestRoot {
        path: std::env::temp_dir().join(format!("wildssim-parity-{}", uuid::Uuid::new_v4())),
    };
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    copy_dir(&testdata, &root.path);
    let simulator = Simulator::load(&root.path).unwrap();
    (root, simulator)
}

fn read(root: &TestRoot, relative: &str) -> String {
    std::fs::read_to_string(root.path.join(relative)).unwrap()
}

fn load_with_parallelism(parallelism: i32) -> (TestRoot, Simulator) {
    let root = TestRoot {
        path: std::env::temp_dir().join(format!("wildssim-parity-{}", uuid::Uuid::new_v4())),
    };
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    copy_dir(&testdata, &root.path);
    let conf = root.path.join("conf").join("LogicConfig.csv");
    let text = format!(
        "スロットの最大の大きさ,最近使ったスキルの記憶容量,入手不可装備の利用有無,下位互換護石の検出有無,最大並列処理数\n3,20,false,true,{}\n",
        parallelism
    );
    std::fs::write(&conf, text).unwrap();
    let simulator = Simulator::load(&root.path).unwrap();
    (root, simulator)
}

fn find_charm<'a>(simulator: &'a Simulator, disp_name: &str) -> &'a Equipment {
    simulator
        .masters
        .additional_charms
        .iter()
        .find(|charm| charm.disp_name() == disp_name)
        .unwrap_or_else(|| panic!("charm not found: {}", disp_name))
}

#[test]
fn upper_charm_detection_matches_across_parallelism() {
    let (_sequential_root, sequential) = load_with_parallelism(1);
    let (_parallel_root, parallel) = load_with_parallelism(4);

    let summarize = |simulator: &Simulator| {
        simulator
            .masters
            .additional_charms
            .iter()
            .map(|charm| {
                (
                    charm.name.clone(),
                    charm.upper.as_ref().map(|upper| {
                        (
                            upper.name.clone(),
                            upper.disp_name.clone(),
                            upper.is_full_upper,
                        )
                    }),
                )
            })
            .collect::<Vec<_>>()
    };

    let sequential = summarize(&sequential);
    let parallel = summarize(&parallel);
    assert_eq!(sequential, parallel);
    assert!(sequential.iter().any(|(_, upper)| upper.is_some()));
}

#[test]
fn load_data_matches_reference() {
    let (_root, simulator) = setup();

    assert!(!simulator.masters.skills.is_empty());
    assert!(simulator
        .masters
        .skills
        .iter()
        .all(|skill| skill.name != "未実装スキル"));

    let lower = find_charm(&simulator, "3(防)-2(防)-1(防)");
    let upper = lower.upper.as_ref().expect("upper charm");
    assert_eq!(upper.disp_name, "3(両)-2(両)-1(両)");
    assert!(upper.is_full_upper);

    let same = find_charm(&simulator, "未実装スキルLv3, 0-0-0");
    let same_upper = same.upper.as_ref().expect("same upper charm");
    assert!(!same_upper.is_full_upper);

    assert!(simulator
        .masters
        .my_conditions
        .iter()
        .any(|condition| condition.weapon_disp_name.as_deref() == Some("登録済アーティア")));

    let set = simulator
        .masters
        .my_sets
        .iter()
        .find(|set| set.name == "ワンセット")
        .expect("ワンセット");
    assert_eq!(set.weapon.name, "スロットのみ_0-0-0");
    assert_eq!(set.head.name, "セット頭");
    assert_eq!(set.body.name, "セット胴");
    assert_eq!(set.arm.name, "セット腕");
    assert_eq!(set.waist.name, "セット腰");
    assert_eq!(set.leg.name, "セット足");
    assert_eq!(set.charm.name, "テスト護石Ⅰ");
    assert_eq!(set.decos.len(), 5);
    assert!(set.decos.iter().all(|deco| deco.name == "防二珠【２】"));
    assert!(!set.is_transcending);

    let transcending = simulator
        .masters
        .my_sets
        .iter()
        .find(|set| set.name == "ワンセット限界突破")
        .expect("ワンセット限界突破");
    assert!(transcending.is_transcending);

    let additional = simulator
        .masters
        .my_sets
        .iter()
        .find(|set| set.name == "追加装備検証")
        .expect("追加装備検証");
    assert_eq!(additional.weapon.disp_name(), "登録済アーティア");
    assert_eq!(
        (
            additional.charm.slot1,
            additional.charm.slot2,
            additional.charm.slot3
        ),
        (3, 2, 1)
    );
    assert_eq!(
        (
            additional.charm.slot_type1,
            additional.charm.slot_type2,
            additional.charm.slot_type3
        ),
        (0, 0, 0)
    );

    assert!(simulator
        .masters
        .my_sets
        .iter()
        .any(|set| set.name == "歯抜けセット"));
    assert!(!simulator.masters.cludes.is_empty());
    assert!(!simulator.masters.recent_skill_names.is_empty());
}

#[test]
fn master_skill_max_level_matches_maximum() {
    let (_root, simulator) = setup();
    for skill in &simulator.masters.skills {
        assert_eq!(skill.max_level, skill.level, "{}", skill.name);
    }
}

#[test]
fn include_replaces_all_same_kind_includes() {
    let (_root, mut simulator) = setup();
    simulator.masters.cludes.push(Clude {
        name: "つよ頭".to_string(),
        kind: CludeKind::Include,
    });
    simulator.masters.cludes.push(Clude {
        name: "よわ頭".to_string(),
        kind: CludeKind::Include,
    });
    simulator.masters.cludes.push(Clude {
        name: "つよ腕".to_string(),
        kind: CludeKind::Exclude,
    });
    simulator.add_include("よわ頭").unwrap();
    let includes: Vec<&Clude> = simulator
        .masters
        .cludes
        .iter()
        .filter(|clude| clude.kind == CludeKind::Include)
        .collect();
    assert_eq!(includes.len(), 1);
    assert_eq!(includes[0].name, "よわ頭");
    assert!(simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.kind == CludeKind::Exclude && clude.name == "つよ腕"));
}

#[test]
fn my_set_missing_equipment_keeps_names() {
    let (_root, simulator) = setup();
    let mut equips: Vec<Equipment> = Vec::new();
    equips.extend(simulator.masters.weapons.iter().cloned());
    equips.extend(simulator.masters.heads.iter().cloned());
    equips.extend(simulator.masters.bodys.iter().cloned());
    equips.extend(simulator.masters.arms.iter().cloned());
    equips.extend(simulator.masters.waists.iter().cloned());
    equips.extend(simulator.masters.legs.iter().cloned());
    equips.extend(simulator.masters.charms.iter().cloned());
    equips.extend(simulator.masters.decos.iter().cloned());

    let sets = simulator.file.load_my_set_csv(&equips).unwrap();
    let set = sets
        .iter()
        .find(|set| set.name == "追加装備検証")
        .expect("追加装備検証");
    assert_eq!(set.weapon.name, "17d1d296-0c36-4b40-a954-87ef527bd2cd");
    assert!(set.weapon.skills.is_empty());
    assert_eq!(set.charm.name, "e0b2395f-5dff-419a-997b-d80218ef647a");
    assert_eq!(
        (set.charm.slot1, set.charm.slot2, set.charm.slot3),
        (0, 0, 0)
    );
}

#[test]
fn my_set_old_file_defaults_to_not_transcending() {
    let (root, simulator) = setup();
    std::fs::write(
        root.path.join("save/myset.csv"),
        "武器,頭,胴,腕,腰,足,護石,装飾品,名前\r\nスロットのみ_0-0-0,つよ頭,つよ胴,つよ腕,つよ腰,つよ足,テスト護石Ⅰ,,つよセット",
    )
    .unwrap();
    let equips: Vec<Equipment> = simulator
        .masters
        .all_equipments()
        .into_iter()
        .cloned()
        .collect();
    let sets = simulator.file.load_my_set_csv(&equips).unwrap();
    let set = sets
        .iter()
        .find(|set| set.name == "つよセット")
        .expect("つよセット");
    assert!(!set.is_transcending);
}

#[test]
fn my_set_invalid_file_fails() {
    let (root, simulator) = setup();
    std::fs::write(
        root.path.join("save/myset.csv"),
        "武器,頭,胴,腕,腰,足,護石,装飾品,名前,限界突破有無\r\nスロットのみ_0-0-0,セット頭,\r\n",
    )
    .unwrap();
    let equips: Vec<Equipment> = simulator
        .masters
        .all_equipments()
        .into_iter()
        .cloned()
        .collect();
    assert!(simulator.file.load_my_set_csv(&equips).is_err());
}

#[test]
fn clude_edge_cases() {
    let (root, mut simulator) = setup();
    simulator.masters.cludes.push(Clude {
        name: "つよ頭".to_string(),
        kind: CludeKind::Include,
    });

    assert!(simulator.add_exclude("よわ頭").unwrap().is_some());
    assert!(simulator.add_exclude("").unwrap().is_none());
    assert!(simulator.add_exclude("存在しない装備名").unwrap().is_none());
    assert!(simulator
        .add_exclude("スロットのみ_3-3-3")
        .unwrap()
        .is_none());
    let replaced = simulator.add_exclude("つよ頭").unwrap().unwrap();
    assert_eq!(replaced.kind, CludeKind::Exclude);

    simulator.masters.cludes.push(Clude {
        name: "つよ腕".to_string(),
        kind: CludeKind::Include,
    });
    assert!(simulator.add_include("よわ腕").unwrap().is_some());
    assert_eq!(
        simulator
            .masters
            .cludes
            .iter()
            .filter(|clude| {
                clude.kind == CludeKind::Include
                    && simulator
                        .masters
                        .get_equip_by_name(&clude.name)
                        .map(|equip| equip.kind == EquipKind::Arm)
                        .unwrap_or(false)
            })
            .count(),
        1
    );
    assert!(simulator.add_include("スキルつき大剣").unwrap().is_none());
    assert!(simulator.add_include("武一珠【１】").unwrap().is_none());
    assert!(simulator.add_include("").unwrap().is_none());

    let count = simulator.masters.cludes.len();
    simulator.delete_clude("存在しない装備名", true).unwrap();
    assert_eq!(simulator.masters.cludes.len(), count);
    simulator.delete_clude("つよ頭", true).unwrap();
    assert!(!simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.name == "つよ頭"));

    simulator.masters.cludes.push(Clude {
        name: "スキルつき大剣".to_string(),
        kind: CludeKind::Exclude,
    });
    simulator.delete_all_armor_clude().unwrap();
    assert!(!simulator.masters.cludes.iter().any(|clude| {
        simulator
            .masters
            .get_equip_by_name(&clude.name)
            .map(|equip| equip.kind.is_armor())
            .unwrap_or(false)
    }));
    assert!(simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.name == "未実装頭"));

    simulator.masters.cludes.push(Clude {
        name: "よわ頭".to_string(),
        kind: CludeKind::Exclude,
    });
    simulator.delete_all_weapon_clude().unwrap();
    assert!(!simulator.masters.cludes.iter().any(|clude| {
        simulator
            .masters
            .get_equip_by_name(&clude.name)
            .map(|equip| equip.kind == EquipKind::Weapon)
            .unwrap_or(false)
    }));
    assert!(simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.name == "未実装頭"));
    assert!(read(&root, "save/clude.csv").contains("未実装頭"));

    simulator.delete_all_clude().unwrap();
    assert!(simulator.masters.cludes.is_empty());
    assert_eq!(read(&root, "save/clude.csv").lines().count(), 1);
}

#[test]
fn my_set_mutations() {
    let (root, mut simulator) = setup();
    let base = simulator.masters.my_sets.len();

    let added = simulator.add_my_set(EquipSet::new()).unwrap().unwrap();
    assert_eq!(added.name, "マイセット");
    assert_eq!(simulator.masters.my_sets.len(), base + 1);
    assert_eq!(read(&root, "save/myset.csv").lines().count(), base + 2);

    let mut virtual_charm = EquipSet::new();
    virtual_charm.charm.name = "virtual".to_string();
    assert!(simulator.add_my_set(virtual_charm).unwrap().is_none());
    let mut virtual_weapon = EquipSet::new();
    virtual_weapon.weapon.name = "virtual".to_string();
    assert!(simulator.add_my_set(virtual_weapon).unwrap().is_none());
    assert_eq!(simulator.masters.my_sets.len(), base + 1);

    simulator.delete_my_set_at(1).unwrap();
    assert_eq!(simulator.masters.my_sets.len(), base);

    let names: Vec<String> = simulator
        .masters
        .my_sets
        .iter()
        .map(|set| set.name.clone())
        .collect();
    simulator.move_my_set(1, 2).unwrap();
    assert_eq!(simulator.masters.my_sets[2].name, names[1]);
    let moved: Vec<String> = simulator
        .masters
        .my_sets
        .iter()
        .map(|set| set.name.clone())
        .collect();
    simulator.move_my_set(1, 200).unwrap();
    simulator.move_my_set(100, 2).unwrap();
    simulator.move_my_set(1, 1).unwrap();
    assert_eq!(
        simulator
            .masters
            .my_sets
            .iter()
            .map(|set| set.name.clone())
            .collect::<Vec<_>>(),
        moved
    );

    let before = read(&root, "save/myset.csv");
    simulator.change_name_of_my_set("", 1).unwrap();
    simulator
        .change_name_of_my_set(&simulator.masters.my_sets[1].name.clone(), 1)
        .unwrap();
    simulator.change_name_of_my_set("RenameTest", 100).unwrap();
    assert_eq!(read(&root, "save/myset.csv"), before);

    simulator.change_name_of_my_set("RenameTest", 1).unwrap();
    assert_eq!(simulator.masters.my_sets[1].name, "RenameTest");
}

#[test]
fn recent_skill_limits() {
    for (expected, max, add_count) in [(4, 20, 3), (3, 3, 3), (1, 1, 0), (2, 0, 2)] {
        let (root, mut simulator) = setup();
        simulator.logic.max_recent_skill_count = max;
        let mut skills = Vec::new();
        for index in 0..add_count {
            let name = if index == 0 {
                "防具スキル".to_string()
            } else {
                format!("防具スキル{}", index)
            };
            skills.push(simulator.masters.make_skill(&name, 1));
        }
        simulator.update_recent_skill(&skills).unwrap();
        assert_eq!(
            simulator.masters.recent_skill_names.len(),
            expected,
            "max={} add={}",
            max,
            add_count
        );
        if add_count > 0 {
            assert_eq!(simulator.masters.recent_skill_names[0], "防具スキル");
        } else if max > 0 {
            assert_eq!(simulator.masters.recent_skill_names[0], "武器スキル");
        }
        assert_eq!(
            read(&root, "save/recentSkill.csv").lines().count(),
            expected + 1
        );
    }
}

#[test]
fn my_condition_mutations_and_roundtrip() {
    let (root, mut simulator) = setup();
    let base = simulator.masters.my_conditions.len();

    let mut condition = SearchCondition::new();
    condition.id = Some("parity-id".to_string());
    condition.disp_name = Some("ラウンドトリップ".to_string());
    condition.is_specific_weapon = false;
    condition.weapon_type = WeaponType::GreatSword;
    condition.min_attack = Some(20);
    condition.def = Some(0);
    condition.fire = Some(0);
    condition.water = Some(0);
    condition.thunder = Some(0);
    condition.ice = Some(0);
    condition.dragon = Some(0);
    condition.is_transcending = false;
    let skill = simulator.masters.make_skill("防具スキル", 2);
    condition.add_skill(&skill);
    let mut fixed = simulator.masters.make_skill("つよ防具スキル全", 1);
    fixed.is_fixed = true;
    condition.add_skill(&fixed);

    simulator.add_my_condition(condition).unwrap();
    assert_eq!(simulator.masters.my_conditions.len(), base + 1);

    let loaded = simulator
        .file
        .load_my_condition_csv(simulator.masters.skill_lookup())
        .unwrap();
    let got = loaded
        .iter()
        .find(|condition| condition.disp_name.as_deref() == Some("ラウンドトリップ"))
        .expect("roundtrip condition");
    assert_eq!(got.id.as_deref(), Some("parity-id"));
    assert_eq!(got.weapon_type, WeaponType::GreatSword);
    assert_eq!(got.min_attack, Some(20));
    assert_eq!(got.def, Some(0));
    assert!(!got.is_transcending);
    assert_eq!(got.skills.len(), 2);
    assert_eq!(got.skills[0].name, "防具スキル");
    assert_eq!(got.skills[0].level, 2);
    assert!(!got.skills[0].is_fixed);
    assert_eq!(got.skills[1].name, "つよ防具スキル全");
    assert!(got.skills[1].is_fixed);

    let old = loaded
        .iter()
        .find(|condition| condition.disp_name.as_deref() == Some("0入力"))
        .expect("0入力");
    assert!(!old.is_transcending);
    let default = loaded
        .iter()
        .find(|condition| condition.disp_name.as_deref() == Some("デフォルト"))
        .expect("デフォルト");
    assert!(default.is_transcending);

    let before = read(&root, "save/condition.csv");
    simulator.change_name_of_my_condition("", 1).unwrap();
    simulator
        .change_name_of_my_condition("ラウンドトリップ", 100)
        .unwrap();
    assert_eq!(read(&root, "save/condition.csv"), before);

    simulator.change_name_of_my_condition("Renamed", 1).unwrap();
    assert_eq!(
        simulator.masters.my_conditions[1].disp_name.as_deref(),
        Some("Renamed")
    );
    simulator.delete_my_condition_at(1).unwrap();
    assert_eq!(simulator.masters.my_conditions.len(), base);
    simulator.delete_my_condition_at(100).unwrap();
    assert_eq!(simulator.masters.my_conditions.len(), base);
}

#[test]
fn deco_count_persists() {
    let (root, mut simulator) = setup();
    let name = simulator.masters.decos[1].name.clone();
    simulator.save_deco_count(&name, 8).unwrap();
    let reloaded = simulator
        .file
        .load_deco_csv(simulator.masters.skill_lookup())
        .unwrap();
    let deco = reloaded.iter().find(|deco| deco.name == name).unwrap();
    assert_eq!(deco.deco_count, 8);

    let before = read(&root, "save/decocount.json");
    simulator.save_deco_count(&name, -4).unwrap();
    assert_eq!(read(&root, "save/decocount.json"), before);
}

#[test]
fn deco_count_json_keeps_master_order() {
    let (root, mut simulator) = setup();
    let name = simulator.masters.decos[0].name.clone();
    simulator.save_deco_count(&name, 2).unwrap();

    // The C# original serializes a Dictionary in insertion order. A sorted map
    // would load the same, but would rewrite the file in a different order.
    let expected = format!(
        "{{{}}}",
        simulator
            .masters
            .decos
            .iter()
            .map(|deco| format!(
                "{}:{}",
                serde_json::to_string(&deco.name).unwrap(),
                deco.deco_count
            ))
            .collect::<Vec<_>>()
            .join(",")
    );
    assert_eq!(read(&root, "save/decocount.json"), expected);
}

#[test]
fn charm_mutations() {
    let (root, mut simulator) = setup();
    let base = simulator.masters.additional_charms.len();

    let mut charm = Equipment::new(EquipKind::Charm);
    charm.name = "parity-charm".to_string();
    simulator.add_charm(charm.clone()).unwrap();
    assert_eq!(simulator.masters.additional_charms.len(), base + 1);
    let added = find_charm(&simulator, "parity-charm");
    assert!(added.upper.is_some());

    let before = read(&root, "save/additionalCharm.csv");
    simulator.add_charm(charm).unwrap();
    assert_eq!(simulator.masters.additional_charms.len(), base + 1);
    assert_eq!(read(&root, "save/additionalCharm.csv"), before);

    let lower = simulator
        .masters
        .additional_charms
        .iter()
        .find(|charm| charm.upper.is_some())
        .cloned()
        .expect("lower charm");
    let old_upper = lower.upper.as_ref().unwrap().name.clone();
    simulator.delete_charm(&old_upper).unwrap();
    let lower_after = simulator
        .masters
        .additional_charms
        .iter()
        .find(|charm| charm.name == lower.name)
        .unwrap();
    assert_ne!(
        lower_after.upper.as_ref().map(|upper| upper.name.clone()),
        Some(old_upper)
    );

    let target = "e0b2395f-5dff-419a-997b-d80218ef647a";
    simulator.masters.cludes.push(Clude {
        name: target.to_string(),
        kind: CludeKind::Exclude,
    });
    let sets_before = simulator.masters.my_sets.len();
    simulator.delete_charm(target).unwrap();
    assert!(!simulator
        .masters
        .additional_charms
        .iter()
        .any(|charm| charm.name == target));
    assert!(simulator.masters.my_sets.len() < sets_before);
    assert!(!simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.name == target));

    let count = simulator.masters.additional_charms.len();
    let names: Vec<String> = simulator
        .masters
        .additional_charms
        .iter()
        .map(|charm| charm.name.clone())
        .collect();
    simulator.move_charm(1, 2).unwrap();
    assert_eq!(simulator.masters.additional_charms[2].name, names[1]);
    simulator.move_charm(1, 200).unwrap();
    simulator.move_charm(100, 2).unwrap();
    simulator.move_charm(1, 1).unwrap();
    assert_eq!(simulator.masters.additional_charms.len(), count);
}

#[test]
fn artian_mutations() {
    let (root, mut simulator) = setup();
    let base = simulator.masters.artians.len();

    let mut artian = Equipment::new_weapon();
    artian.init_artian();
    artian.name = "parity-artian".to_string();
    artian.disp_name = Some("ParityArtian".to_string());
    artian.weapon_type = WeaponType::GreatSword;
    simulator.add_artian(artian.clone()).unwrap();
    assert_eq!(simulator.masters.artians.len(), base + 1);
    assert!(simulator
        .masters
        .all_equipments()
        .iter()
        .any(|equip| equip.name == "parity-artian"));

    let before = read(&root, "save/artian.csv");
    simulator.add_artian(artian).unwrap();
    assert_eq!(simulator.masters.artians.len(), base + 1);
    assert_eq!(read(&root, "save/artian.csv"), before);

    let used = "17d1d296-0c36-4b40-a954-87ef527bd2cd";
    simulator.masters.cludes.push(Clude {
        name: used.to_string(),
        kind: CludeKind::Exclude,
    });
    let sets_before = simulator.masters.my_sets.len();
    simulator.delete_artian(used).unwrap();
    assert!(!simulator
        .masters
        .artians
        .iter()
        .any(|artian| artian.name == used));
    assert!(simulator.masters.my_sets.len() < sets_before);
    assert!(!simulator
        .masters
        .cludes
        .iter()
        .any(|clude| clude.name == used));

    let unused = "17d1d296-ghkl-4b40-a954-87ef527bd2cd";
    let myset_before = read(&root, "save/myset.csv");
    simulator.delete_artian(unused).unwrap();
    assert!(!simulator
        .masters
        .artians
        .iter()
        .any(|artian| artian.name == unused));
    assert_eq!(read(&root, "save/myset.csv"), myset_before);

    let mut second = Equipment::new_weapon();
    second.init_artian();
    second.name = "parity-artian-2".to_string();
    second.disp_name = Some("ParityArtian2".to_string());
    second.weapon_type = WeaponType::GreatSword;
    simulator.add_artian(second).unwrap();

    let count = simulator.masters.artians.len();
    let names: Vec<String> = simulator
        .masters
        .artians
        .iter()
        .map(|artian| artian.name.clone())
        .collect();
    simulator.move_artian(1, 0).unwrap();
    assert_eq!(simulator.masters.artians[0].name, names[1]);
    simulator.move_artian(1, 200).unwrap();
    simulator.move_artian(100, 2).unwrap();
    simulator.move_artian(1, 1).unwrap();
    assert_eq!(simulator.masters.artians.len(), count);
}

#[test]
fn weapon_load_matches_reference() {
    let (root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup().clone();
    let weapons = simulator.file.load_weapon_csv(&lookup).unwrap();

    let sample = weapons
        .iter()
        .find(|weapon| weapon.name == "スキルつき太刀")
        .expect("スキルつき太刀");
    assert_eq!(sample.attack, 90);
    assert_eq!(sample.weapon_type, WeaponType::LongSword);
    assert_eq!(sample.rare, 1);
    assert_eq!((sample.slot1, sample.slot2, sample.slot3), (3, 2, 1));
    assert_eq!(
        (sample.slot_type1, sample.slot_type2, sample.slot_type3),
        (1, 1, 1)
    );
    assert_eq!(sample.mindef, 10);
    assert_eq!(sample.maxdef, 10);
    assert_eq!(sample.transcending_def(), 10);
    assert_eq!(
        (
            sample.fire,
            sample.water,
            sample.thunder,
            sample.ice,
            sample.dragon
        ),
        (0, 0, 0, 0, 0)
    );
    assert!(!sample.is_one_set);
    assert_eq!(sample.row_no, i32::MAX);
    assert!(!sample.is_virtual);
    assert!(sample.upper.is_none());
    assert_eq!(sample.skills.len(), 1);
    assert_eq!(sample.skills[0].name, "武器スキル");
    assert_eq!(sample.skills[0].level, 1);

    assert!(!weapons.iter().any(|weapon| weapon.name == "未実装大剣"));
    assert_eq!(
        weapons
            .iter()
            .filter(|weapon| weapon.weapon_type == WeaponType::None)
            .count(),
        20
    );

    let mut logic = simulator.logic.clone();
    logic.max_slot_size = 4;
    logic.allow_unavailable_equipments = true;
    let file = FileOperation::new(root.path.clone(), logic);
    let weapons = file.load_weapon_csv(&lookup).unwrap();
    let unavailable = weapons
        .iter()
        .find(|weapon| weapon.name == "未実装大剣")
        .expect("未実装大剣");
    assert_eq!(unavailable.row_no, 1);
    assert_eq!(
        weapons
            .iter()
            .filter(|weapon| weapon.weapon_type == WeaponType::None)
            .count(),
        35
    );
}

#[test]
fn armor_load_matches_reference() {
    let (root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup();
    let upgrades = simulator.file.load_def_upgrade_csv().unwrap();
    let heads = simulator
        .file
        .load_equip_csv(
            "MHWilds_EQUIP_HEAD.csv",
            EquipKind::Head,
            Some(&upgrades),
            lookup,
        )
        .unwrap();

    let sample = heads
        .iter()
        .find(|head| head.name == "つよ頭")
        .expect("つよ頭");
    assert_eq!(sample.kind, EquipKind::Head);
    assert_eq!(sample.rare, 8);
    assert_eq!((sample.slot1, sample.slot2, sample.slot3), (0, 0, 0));
    assert_eq!(sample.mindef, 20);
    assert_eq!(sample.maxdef, 28);
    assert_eq!(sample.transcending_def(), 36);
    assert_eq!(sample.fire, 5);
    assert_eq!(sample.water, -10);
    assert_eq!(sample.thunder, -10);
    assert_eq!(sample.ice, -10);
    assert_eq!(sample.dragon, -10);
    assert!(!sample.is_one_set);
    assert_eq!(sample.row_no, 20);
    assert_eq!(sample.skills.len(), 5);
    for (name, level) in [
        ("つよグループ", 1),
        ("つよの力", 1),
        ("防具スキル", 1),
        ("つよ防具スキル全", 1),
        ("つよ防具スキル頭", 1),
    ] {
        assert!(
            sample
                .skills
                .iter()
                .any(|skill| skill.name == name && skill.level == level),
            "{}",
            name
        );
    }

    let set = heads
        .iter()
        .find(|head| head.name == "セット頭")
        .expect("セット頭");
    assert!(set.is_one_set);
    assert_eq!((set.slot1, set.slot2, set.slot3), (2, 1, 0));
    assert!(!heads.iter().any(|head| head.name == "未実装頭"));

    let heads = simulator
        .file
        .load_equip_csv("MHWilds_EQUIP_HEAD.csv", EquipKind::Head, None, lookup)
        .unwrap();
    let sample = heads
        .iter()
        .find(|head| head.name == "つよ頭")
        .expect("つよ頭");
    assert_eq!(sample.maxdef, 20);
    assert_eq!(sample.transcending_def(), 20);

    let mut logic = simulator.logic.clone();
    logic.allow_unavailable_equipments = true;
    let file = FileOperation::new(root.path.clone(), logic);
    let heads = file
        .load_equip_csv(
            "MHWilds_EQUIP_HEAD.csv",
            EquipKind::Head,
            Some(&upgrades),
            lookup,
        )
        .unwrap();
    let unavailable = heads
        .iter()
        .find(|head| head.name == "未実装頭")
        .expect("未実装頭");
    assert_eq!(unavailable.mindef, 10);
    assert_eq!(unavailable.maxdef, 10);
    assert_eq!(unavailable.transcending_def(), 10);
}

#[test]
fn charm_and_deco_load_match_reference() {
    let (root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup();

    let charms = simulator
        .file
        .load_equip_csv("MHWilds_CHARM.csv", EquipKind::Charm, None, lookup)
        .unwrap();
    assert_eq!(charms.len(), 1);
    let charm = &charms[0];
    assert_eq!(charm.kind, EquipKind::Charm);
    assert_eq!(charm.rare, 1);
    assert_eq!((charm.slot1, charm.slot2, charm.slot3), (0, 0, 0));
    assert_eq!(charm.maxdef, 0);
    assert_eq!(charm.skills.len(), 1);
    assert_eq!(charm.skills[0].name, "防具スキル");
    assert!(!charms.iter().any(|charm| charm.name == "未実装護石Ⅰ"));

    let decos = simulator.file.load_deco_csv(lookup).unwrap();
    let sample = decos
        .iter()
        .find(|deco| deco.name == "武三珠【３】")
        .expect("武三珠【３】");
    assert_eq!(sample.kind, EquipKind::Deco);
    assert_eq!(sample.slot1, 3);
    assert_eq!(sample.slot_type1, 1);
    assert_eq!(sample.skills.len(), 1);
    assert_eq!(sample.skills[0].name, "スロ3武器スキル");
    assert_eq!(sample.deco_category(), sample.skills[0].category);
    assert_eq!(sample.deco_count, 7);
    let short = decos
        .iter()
        .find(|deco| deco.name == "不足珠【１】")
        .expect("不足珠【１】");
    assert_eq!(short.deco_count, 1);
    assert!(!decos.iter().any(|deco| deco.name == "複合珠【４】"));

    let mut logic = simulator.logic.clone();
    logic.allow_unavailable_equipments = true;
    let file = FileOperation::new(root.path.clone(), logic);
    let decos = file.load_deco_csv(lookup).unwrap();
    let composite = decos
        .iter()
        .find(|deco| deco.name == "複合珠【４】")
        .expect("複合珠【４】");
    assert_eq!(composite.deco_category(), "未実装スキル複合");
    assert_eq!(composite.deco_count, 0);
}

#[test]
fn additional_charm_old_format_loads() {
    let (root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup();
    std::fs::write(
        root.path.join("save/additionalCharm.csv"),
        "スキル系統1,スキル値1,スキル系統2,スキル値2,スキル系統3,スキル値3,(泣用防具スロ1),(泣用防具スロ2),(泣用防具スロ3),(泣用武器スロ1),(泣用武器スロ2),(泣用武器スロ3),スロット1,スロット2,スロット3,スロット1タイプ,スロット2タイプ,スロット3タイプ,内部管理ID,マイセット登録有無\r\n\
         ,,,,,,3,2,1,0,0,0\r\n\
         ,,,,,,0,0,0,3,2,1\r\n\
         スロ1不足スキル,2,,,,,0,0,0,0,0,0,0,0,0,0,0,0,,\r\n",
    )
    .unwrap();
    let charms = simulator.file.load_additional_charm_csv(lookup).unwrap();

    let armor = find_charm_in(&charms, "3(防)-2(防)-1(防)");
    assert_eq!((armor.slot1, armor.slot2, armor.slot3), (3, 2, 1));
    assert_eq!(
        (armor.slot_type1, armor.slot_type2, armor.slot_type3),
        (0, 0, 0)
    );
    assert!(armor.skills.is_empty());

    let weapon = find_charm_in(&charms, "3(武)-2(武)-1(武)");
    assert_eq!((weapon.slot1, weapon.slot2, weapon.slot3), (3, 2, 1));
    assert_eq!(
        (weapon.slot_type1, weapon.slot_type2, weapon.slot_type3),
        (1, 1, 1)
    );

    let skill = find_charm_in(&charms, "スロ1不足スキルLv2, 0-0-0");
    assert_eq!(skill.skills.len(), 1);
    assert_eq!(skill.skills[0].name, "スロ1不足スキル");
    assert_eq!(skill.skills[0].level, 2);
    assert!(charms.iter().all(|charm| !charm.name.is_empty()));
}

fn find_charm_in<'a>(charms: &'a [Equipment], disp_name: &str) -> &'a Equipment {
    charms
        .iter()
        .find(|charm| charm.disp_name() == disp_name)
        .unwrap_or_else(|| panic!("charm not found: {}", disp_name))
}

#[test]
fn artian_and_def_upgrade_load_match_reference() {
    let (_root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup();

    let upgrades = simulator.file.load_def_upgrade_csv().unwrap();
    assert_eq!(upgrades[&5].upgrade_def, 10);
    assert_eq!(upgrades[&8].transcending_def, 8);

    let artians = simulator.file.load_artian_csv(lookup).unwrap();
    let sample = artians
        .iter()
        .find(|artian| artian.disp_name() == "登録済アーティア")
        .expect("登録済アーティア");
    assert_eq!(sample.attack, 190);
    assert_eq!(sample.weapon_type, WeaponType::GreatSword);
    assert_eq!(sample.rare, 8);
    assert_eq!((sample.slot1, sample.slot2, sample.slot3), (3, 3, 3));
    assert_eq!(
        (sample.slot_type1, sample.slot_type2, sample.slot_type3),
        (1, 1, 1)
    );
    assert_eq!(sample.skills.len(), 2);
    assert_eq!(sample.skills[0].name, "よわグループ");
    assert_eq!(sample.skills[1].name, "つよの力");
}

#[test]
fn loader_edge_cases() {
    let (root, simulator) = setup();
    let lookup = simulator.masters.skill_lookup();

    std::fs::remove_file(root.path.join("save/decocount.json")).unwrap();
    let decos = simulator.file.load_deco_csv(lookup).unwrap();
    let short = decos
        .iter()
        .find(|deco| deco.name == "不足珠【１】")
        .expect("不足珠【１】");
    assert_eq!(short.deco_count, 7);

    std::fs::write(root.path.join("save/decocount.json"), "{{").unwrap();
    assert!(simulator.file.load_deco_csv(lookup).is_err());

    std::fs::write(
        root.path.join("save/condition.csv"),
        "ID,名前,武器指定有無,武器名,武器種,攻撃力,防御力,火耐性,水耐性,雷耐性,氷耐性,龍耐性,スキル\r\n\
         id1,旧条件,False,null,大剣,null,null,null,null,null,null,null,\"防具スキル,2\"\r\n",
    )
    .unwrap();
    let conditions = simulator.file.load_my_condition_csv(lookup).unwrap();
    assert_eq!(conditions.len(), 1);
    assert!(conditions[0].is_transcending);

    std::fs::write(
        root.path.join("save/artian.csv"),
        "武器種,名前,スキル系統1,スキル値1,スキル系統2,スキル値2,内部管理ID,マイセット登録有無\r\n\
         大剣,登録済アーティア,よわグループ,1,つよの力,1,,\r\n",
    )
    .unwrap();
    let artians = simulator.file.load_artian_csv(lookup).unwrap();
    assert_eq!(artians.len(), 1);
    assert!(!artians[0].name.is_empty());
}

#[test]
fn equip_set_reference_values() {
    let (_root, simulator) = setup();
    let find = |name: &str| {
        simulator
            .masters
            .my_sets
            .iter()
            .find(|set| set.name == name)
            .unwrap_or_else(|| panic!("set not found: {}", name))
    };

    let set = find("歯抜けセット");
    assert_eq!(set.mindef(), 80);
    assert_eq!(set.maxdef(), 144);
    assert_eq!(
        (
            set.fire(),
            set.water(),
            set.thunder(),
            set.ice(),
            set.dragon()
        ),
        (-25, -25, -25, -25, -40)
    );
    assert_eq!(
        set.glpk_row_name(),
        "スロットのみ_0-0-0,つよ頭,つよ胴,つよ腕,つよ腰,,テスト護石Ⅰ"
    );
    assert_eq!(set.existing_equips_without_decos().len(), 6);

    let set = find("ワンセット");
    assert_eq!(set.mindef(), 75);
    assert_eq!(set.maxdef(), 125);
    assert_eq!(
        (
            set.fire(),
            set.water(),
            set.thunder(),
            set.ice(),
            set.dragon()
        ),
        (0, 0, 0, 0, 0)
    );
    assert_eq!(set.existing_equips_without_decos().len(), 7);
    assert_eq!(
        set.deco_name_csv(),
        "防二珠【２】,防二珠【２】,防二珠【２】,防二珠【２】,防二珠【２】"
    );
    assert_eq!(
        set.deco_name_csv_multi_line(),
        "防二珠【２】,\n防二珠【２】,防二珠【２】,\n防二珠【２】,防二珠【２】"
    );
    assert_eq!(
        set.skills_disp(),
        "防具スキルLv6, スロ2防具スキルLv5, 強欲Ⅱ(強欲の力Lv4), 激運(強欲グループLv3), つよ防具スキル頭Lv1, つよ防具スキル胴Lv1, つよ防具スキル腕Lv1, つよ防具スキル腰Lv1, つよ防具スキル足Lv1"
    );
    assert_eq!(
        set.skills_disp_multi_line(),
        "防具スキルLv6,スロ2防具スキルLv5,強欲Ⅱ(強欲の力Lv4),\n激運(強欲グループLv3),つよ防具スキル頭Lv1,つよ防具スキル胴Lv1,\nつよ防具スキル腕Lv1,つよ防具スキル腰Lv1,つよ防具スキル足Lv1"
    );
    assert_eq!(
        set.description(),
        "防御:75→125,火:0,水:0,雷:0,氷:0,龍:0\n武器：スロットのみ_0-0-0,0-0-0\n頭：セット頭,2-1-0→3-2-1\n胴：セット胴,2-1-0→3-2-1\n腕：セット腕,2-1-0→3-2-1\n腰：セット腰,2-1-0→3-2-1\n足：セット足,2-1-0→3-2-1\n護石：テスト護石Ⅰ,0-0-0\n装飾品：防二珠【２】,防二珠【２】,防二珠【２】,防二珠【２】,防二珠【２】\n-----------\n防具スキルLv6\nスロ2防具スキルLv5\n強欲Ⅱ(強欲の力Lv4)\n激運(強欲グループLv3)\nつよ防具スキル頭Lv1\nつよ防具スキル胴Lv1\nつよ防具スキル腕Lv1\nつよ防具スキル腰Lv1\nつよ防具スキル足Lv1"
    );

    let set = find("ワンセット限界突破");
    assert_eq!(
        set.description(),
        "防御:75→175(限界突破),火:0,水:0,雷:0,氷:0,龍:0\n武器：スロットのみ_0-0-0,0-0-0\n頭：セット頭,2-1-0→3-2-1\n胴：セット胴,2-1-0→3-2-1\n腕：セット腕,2-1-0→3-2-1\n腰：セット腰,2-1-0→3-2-1\n足：セット足,2-1-0→3-2-1\n護石：テスト護石Ⅰ,0-0-0\n装飾品：防三珠【３】,防三珠【３】,防三珠【３】,防三珠【３】,防三珠【３】\n-----------\n防具スキルLv6\nスロ3防具スキルLv5\n強欲Ⅱ(強欲の力Lv4)\n激運(強欲グループLv3)\nつよ防具スキル頭Lv1\nつよ防具スキル胴Lv1\nつよ防具スキル腕Lv1\nつよ防具スキル腰Lv1\nつよ防具スキル足Lv1"
    );
}

#[test]
fn search_condition_reference_behaviour() {
    let (_root, simulator) = setup();
    let find = |name: &str| {
        simulator
            .masters
            .my_conditions
            .iter()
            .find(|condition| condition.disp_name.as_deref() == Some(name))
            .unwrap_or_else(|| panic!("condition not found: {}", name))
    };

    let first = simulator.masters.make_skill("防具スキル", 2);
    let mut second = simulator.masters.make_skill("つよの力", 2);
    second.is_fixed = true;
    let mut condition = SearchCondition::new();
    condition.skills = vec![first.clone(), second.clone()];
    assert_eq!(condition.skill_csv(), "防具スキル,2,つよの力,2固定");

    let mut parsed = SearchCondition::new();
    parsed.set_skill_csv(&condition.skill_csv(), simulator.masters.skill_lookup());
    assert_eq!(parsed.skills.len(), 2);
    assert_eq!(parsed.skills[0].name, "防具スキル");
    assert_eq!(parsed.skills[0].level, 2);
    assert!(!parsed.skills[0].is_fixed);
    assert_eq!(parsed.skills[1].name, "つよの力");
    assert!(parsed.skills[1].is_fixed);

    let mut weapon = SearchCondition::new();
    weapon.weapon_name = Some("test".to_string());
    assert_eq!(weapon.weapon_disp_name(), Some("test"));
    weapon.weapon_disp_name = Some("disptest".to_string());
    assert_eq!(weapon.weapon_disp_name(), Some("disptest"));

    let description = |name: &str| normalize_newlines(&find(name).description());
    assert_eq!(
        description("0入力"),
        "限界突破強化:なし\n武器種:大剣, 最低攻撃力:20\n防御力:0\n火:0,水:0,雷:0,氷:0,龍:0\n防具スキルLv2\nつよ防具スキル全Lv1(固定)"
    );
    assert_eq!(
        description("攻撃null"),
        "限界突破強化:なし\n武器種:大剣, 最低攻撃力:なし\n防御力:0\n火:0,水:0,雷:0,氷:0,龍:0\n防具スキルLv2\nつよ防具スキル全Lv1(固定)"
    );
    assert_eq!(
        description("デフォルト"),
        "限界突破強化:あり\n武器:スロットのみ_0-0-0\n防御力:なし\n火:なし,水:なし,雷:なし,氷:なし,龍:なし\n防具スキルLv2\nつよ防具スキル全Lv1"
    );
    assert_eq!(
        description("武器指定"),
        "限界突破強化:あり\n武器:ホープブレイドⅠ\n防御力:0\n火:0,水:0,雷:0,氷:0,龍:0\n防具スキルLv2\nつよ防具スキル全Lv1"
    );

    for name in [
        "0入力",
        "攻撃null",
        "デフォルト",
        "武器指定",
        "アーティア指定",
    ] {
        let original = find(name);
        let copy = original.copy_for_search();
        assert_eq!(original.description(), copy.description());
        assert_ne!(original.disp_name, copy.disp_name);
    }

    for (expected, count, expected_level, expected_fixed, name, level, fixed) in [
        (true, 3, 1, false, "武器スキル", 1, false),
        (true, 2, 5, false, "防具スキル", 5, false),
        (false, 2, 4, false, "防具スキル", 1, false),
        (true, 2, 1, true, "防具スキル", 1, true),
        (false, 2, 2, true, "つよ防具スキル", 4, false),
        (true, 2, 4, true, "つよ防具スキル", 4, true),
        (false, 2, 4, false, "防具スキル", 4, false),
        (false, 2, 2, true, "つよ防具スキル", 2, true),
    ] {
        let mut condition = SearchCondition::new();
        let armor = simulator.masters.make_skill("防具スキル", 4);
        condition.add_skill(&armor);
        let mut strong = simulator.masters.make_skill("つよ防具スキル", 2);
        strong.is_fixed = true;
        condition.add_skill(&strong);

        let mut added = simulator.masters.make_skill(name, level);
        added.is_fixed = fixed;
        assert_eq!(condition.add_skill(&added), expected, "{}", name);
        assert_eq!(condition.skills.len(), count, "{}", name);
        let skill = condition
            .skills
            .iter()
            .find(|skill| skill.name == name)
            .unwrap();
        assert_eq!(skill.level, expected_level, "{}", name);
        assert_eq!(skill.is_fixed, expected_fixed, "{}", name);
    }

    let mut combos_condition = SearchCondition::new();
    combos_condition
        .skills
        .push(simulator.masters.make_skill("武器スキル", 3));
    combos_condition
        .skills
        .push(simulator.masters.make_skill("つよ防具スキル頭", 3));
    combos_condition
        .skills
        .push(simulator.masters.make_skill("つよ防具スキル胴", 3));
    let combos = vec![
        CharmCombo {
            group1: 1,
            group2: 2,
            group3: 2,
            rare: 8,
            slot1: 1,
            slot2: 1,
            slot3: 1,
            slot_type1: 1,
            slot_type2: 0,
            slot_type3: 0,
        },
        CharmCombo {
            group1: 2,
            group2: 0,
            group3: 0,
            rare: 7,
            slot1: 0,
            slot2: 0,
            slot3: 0,
            slot_type1: 0,
            slot_type2: 0,
            slot_type3: 0,
        },
    ];
    let mut groups: std::collections::HashMap<i32, Vec<Skill>> = std::collections::HashMap::new();
    groups.insert(0, Vec::new());
    groups.insert(1, vec![simulator.masters.make_skill("武器スキル", 2)]);
    groups.insert(
        2,
        vec![
            simulator.masters.make_skill("つよ防具スキル頭", 1),
            simulator.masters.make_skill("つよ防具スキル胴", 1),
            simulator.masters.make_skill("つよ防具スキル腕", 1),
            simulator.masters.make_skill("つよ防具スキル腰", 1),
            simulator.masters.make_skill("つよ防具スキル足", 1),
        ],
    );
    let charms = combos_condition.make_related_charms(&combos, &groups);
    assert_eq!(charms.len(), 11);
    assert!(!charms.iter().any(|charm| charm
        .skills
        .iter()
        .any(|skill| skill.name == "つよ防具スキル腰")));

    let mut artian_condition = SearchCondition::new();
    for (name, level) in [
        ("つよの力", 2),
        ("よわの力", 2),
        ("つよグループ", 3),
        ("強欲グループ", 3),
    ] {
        artian_condition
            .skills
            .push(simulator.masters.make_skill(name, level));
    }
    let artians = artian_condition.make_related_artians();
    assert_eq!(artians.len(), 5);
    assert!(!artians.iter().any(|artian| artian
        .skills
        .iter()
        .any(|skill| skill.name == "強欲グループ")));
}

#[test]
fn equipment_and_skill_reference_values() {
    let (_root, simulator) = setup();
    for (expected, slot1, slot2, slot3, rare, kind) in [
        ((3, 2, 1), 2, 1, 0, 5, EquipKind::Head),
        ((3, 2, 1), 3, 1, 0, 5, EquipKind::Body),
        ((3, 2, 0), 2, 1, 0, 6, EquipKind::Arm),
        ((3, 2, 0), 3, 1, 0, 6, EquipKind::Waist),
        ((2, 1, 0), 2, 1, 0, 7, EquipKind::Leg),
        ((2, 1, 0), 2, 1, 0, 5, EquipKind::Charm),
        ((2, 1, 0), 2, 1, 0, 5, EquipKind::Deco),
        ((2, 1, 0), 2, 1, 0, 5, EquipKind::Error),
    ] {
        let equip = Equipment {
            slot1,
            slot2,
            slot3,
            rare,
            kind,
            ..Equipment::default()
        };
        assert_eq!(
            (
                equip.transcending_slot1(),
                equip.transcending_slot2(),
                equip.transcending_slot3()
            ),
            expected
        );
    }

    let equip = Equipment {
        maxdef: 10,
        ..Equipment::default()
    };
    assert_eq!(equip.transcending_def(), 10);
    let equip = Equipment {
        maxdef: 10,
        transcending_def: Some(20),
        ..Equipment::default()
    };
    assert_eq!(equip.transcending_def(), 20);

    let equip = Equipment {
        name: "testname".to_string(),
        ..Equipment::default()
    };
    assert_eq!(equip.disp_name(), "testname");
    assert_eq!(equip.detail_disp_name(), "testname");
    let equip = Equipment {
        name: "testname".to_string(),
        disp_name: Some("dispname".to_string()),
        ..Equipment::default()
    };
    assert_eq!(equip.disp_name(), "dispname");
    assert_eq!(equip.detail_disp_name(), "dispname");

    let description = |name: &str| {
        simulator
            .masters
            .all_equipments()
            .into_iter()
            .find(|equip| equip.name == name)
            .map(|equip| equip.description())
            .unwrap_or_else(|| Equipment::default().description())
    };
    assert_eq!(
        description("つよ腕"),
        "つよ腕,0-0-0\n防御:20→28→36,火:-10,水:-10,雷:5,氷:-10,龍:-10\nつよグループLv1\nつよの力Lv1\n防具スキルLv1\nつよ防具スキル全Lv1\nつよ防具スキル腕Lv1"
    );
    assert_eq!(
        description("セット腕"),
        "セット腕,2-1-0→3-2-1\n防御:15→25→35,火:0,水:0,雷:0,氷:0,龍:0\n強欲グループLv1\n強欲の力Lv1\n防具スキルLv1\nつよ防具スキル腕Lv1"
    );
    assert_eq!(
        description("武一珠【１】"),
        "武一珠【１】\nスロ1武器スキルLv1"
    );
    assert_eq!(description("dummy"), "");

    let simple = |name: &str| {
        simulator
            .masters
            .all_equipments()
            .into_iter()
            .find(|equip| equip.name == name)
            .map(|equip| equip.simple_description())
            .unwrap_or_else(|| Equipment::default().simple_description())
    };
    assert_eq!(simple("つよ腕"), "腕：つよ腕,0-0-0");
    assert_eq!(simple("セット腕"), "腕：セット腕,2-1-0→3-2-1");
    assert_eq!(simple("武一珠【１】"), "装飾品：武一珠【１】");
    assert_eq!(simple("dummy"), "頭：");

    let make = |name: &str, level: i32| simulator.masters.make_skill(name, level);
    let skill = make("防具スキル", 2);
    assert_eq!(skill.max_level, 6);
    assert_eq!(skill.description(), "防具スキルLv2");
    let skill = make("つよの力", 2);
    assert_eq!(skill.max_level, 4);
    assert_eq!(skill.description(), "つよⅠ(つよの力Lv2)");
    let skill = make("dummy", 2);
    assert_eq!(skill.max_level, 0);
    assert_eq!(skill.description(), "dummyLv2");
    let skill = make("防具スキル", 0);
    assert_eq!(skill.description(), "");

    for (expected, name, level, arg) in [
        (false, "防具スキル", 2, None),
        (true, "つよの力", 1, None),
        (false, "つよの力", 3, Some(2)),
        (true, "つよの力", 2, Some(3)),
        (false, "つよグループ", 3, None),
        (true, "つよグループ", 2, None),
    ] {
        let skill = make(name, level);
        assert_eq!(
            skill.is_hide_level(arg),
            expected,
            "{}({}) {:?}",
            name,
            level,
            arg
        );
    }

    assert!(simulator.masters.is_skill_name("防具スキル"));
    assert!(!simulator.masters.is_skill_name(""));
    assert!(!simulator.masters.is_skill_name("dummy"));
    assert_eq!(simulator.masters.skill_max_level("防具スキル"), 6);
    assert_eq!(simulator.masters.skill_max_level("つよの力"), 4);
    assert_eq!(simulator.masters.skill_max_level("dummy"), 0);

    assert_eq!(EquipKind::Weapon.as_str(), "武器");
    assert_eq!(EquipKind::Head.as_str(), "頭");
    assert_eq!(EquipKind::Body.as_str(), "胴");
    assert_eq!(EquipKind::Arm.as_str(), "腕");
    assert_eq!(EquipKind::Waist.as_str(), "腰");
    assert_eq!(EquipKind::Leg.as_str(), "足");
    assert_eq!(EquipKind::Deco.as_str(), "装飾品");
    assert_eq!(EquipKind::Charm.as_str(), "護石");
    assert_eq!(EquipKind::Error.as_str(), "");
    assert_eq!(EquipKind::Weapon.str_with_colon(), "武器：");
    assert_eq!(EquipKind::from_str("脚"), EquipKind::Leg);
    assert_eq!(EquipKind::from_str("dummy"), EquipKind::Error);

    let mut weapon = Equipment::new_weapon();
    weapon.init_artian();
    assert_eq!(weapon.rare, 8);
    assert_eq!((weapon.slot1, weapon.slot2, weapon.slot3), (3, 3, 3));
    assert_eq!(
        (weapon.slot_type1, weapon.slot_type2, weapon.slot_type3),
        (1, 1, 1)
    );
    assert_eq!(weapon.mindef, 0);
    assert_eq!(weapon.maxdef, 0);
    assert_eq!(weapon.attack, 190);
    assert_eq!(weapon.row_no, i32::MAX);
}

fn normalize_newlines(text: &str) -> String {
    text.replace("\r\n", "\n")
}

#[test]
fn save_output_matches_fixtures() {
    let (root, simulator) = setup();
    let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata/save");

    let expected = std::fs::read_to_string(base.join("myset.csv")).unwrap();
    simulator
        .file
        .save_my_set_csv(&simulator.masters.my_sets)
        .unwrap();
    assert_eq!(
        normalize_newlines(&read(&root, "save/myset.csv")),
        normalize_newlines(&expected)
    );

    let expected = std::fs::read_to_string(base.join("condition.csv")).unwrap();
    simulator
        .file
        .save_my_condition_csv(&simulator.masters.my_conditions)
        .unwrap();
    assert_eq!(
        normalize_newlines(&read(&root, "save/condition.csv")),
        normalize_newlines(&expected)
    );

    let expected = std::fs::read_to_string(base.join("additionalCharm.csv")).unwrap();
    simulator
        .file
        .save_additional_charm_csv(&simulator.masters.additional_charms, &[])
        .unwrap();
    assert_eq!(
        normalize_newlines(&read(&root, "save/additionalCharm.csv")),
        normalize_newlines(&expected)
    );

    let expected = std::fs::read_to_string(base.join("artian.csv")).unwrap();
    simulator
        .file
        .save_artian_csv(&simulator.masters.artians, &[])
        .unwrap();
    assert_eq!(
        normalize_newlines(&read(&root, "save/artian.csv")),
        normalize_newlines(&expected)
    );
}

#[test]
fn save_clude_and_recent_match_reference_format() {
    let (root, simulator) = setup();
    simulator
        .file
        .save_clude_csv(&[
            Clude {
                name: "除外テスト".to_string(),
                kind: CludeKind::Exclude,
            },
            Clude {
                name: "固定テスト".to_string(),
                kind: CludeKind::Include,
            },
        ])
        .unwrap();
    assert_eq!(
        read(&root, "save/clude.csv"),
        "対象,種別\r\n除外テスト,0\r\n固定テスト,1\r\n"
    );

    simulator
        .file
        .save_recent_skill_csv(&["武器スキル".to_string(), "防具スキル".to_string()])
        .unwrap();
    assert_eq!(
        read(&root, "save/recentSkill.csv"),
        "スキル名\r\n武器スキル\r\n防具スキル\r\n"
    );
}

#[test]
fn logic_config_defaults_and_errors() {
    let (_root, simulator) = setup();
    assert_eq!(simulator.logic.max_slot_size, 3);
    assert_eq!(simulator.logic.max_recent_skill_count, 20);
    assert_eq!(simulator.logic.max_charm_skill_count, 3);
    assert_eq!(simulator.logic.max_degree_of_parallelism, -1);
    assert!(!simulator.logic.allow_unavailable_equipments);
    assert!(simulator.logic.use_calc_upper_charm);
    assert_eq!(simulator.logic.artian_skill_count, 2);
    assert_eq!(simulator.logic.default_my_set_name, "マイセット");

    let root = std::env::temp_dir().join(format!("wildssim-config-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(root.join("conf")).unwrap();
    std::fs::write(root.join("conf/LogicConfig.csv"), "").unwrap();
    let config = LogicConfig::load(&root.join("conf/LogicConfig.csv")).unwrap();
    assert_eq!(config.max_slot_size, 4);
    assert_eq!(config.max_recent_skill_count, 20);
    assert_eq!(config.max_charm_skill_count, 3);
    assert_eq!(config.artian_skill_count, 2);
    assert!(LogicConfig::load(&root.join("conf/missing.csv")).is_err());
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn save_replaces_file_without_leaving_temp() {
    let (root, mut simulator) = setup();
    let save_dir = root.path.join("save");
    std::fs::write(save_dir.join("clude.csv"), "対象,種別\r\nold,0\r\n").unwrap();

    simulator.delete_all_clude().unwrap();

    assert_eq!(read(&root, "save/clude.csv"), "対象,種別\r\n");
    let leftovers: Vec<_> = std::fs::read_dir(&save_dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
        .collect();
    assert!(leftovers.is_empty());
}
