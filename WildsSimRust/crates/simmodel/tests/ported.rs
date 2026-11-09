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
        path: std::env::temp_dir().join(format!("wildssim-test-{}", uuid::Uuid::new_v4())),
    };
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    copy_dir(&testdata, &root.path);
    let simulator = Simulator::load(&root.path).unwrap();
    (root, simulator)
}

fn condition_with(
    simulator: &Simulator,
    skills: &[(&str, i32)],
    fixed: &[(&str, i32)],
    weapon: Option<&str>,
) -> SearchCondition {
    let mut condition = SearchCondition::new();
    for (name, level) in skills {
        let skill = simulator.masters.make_skill(name, *level);
        condition.add_skill(&skill);
    }
    for (name, level) in fixed {
        let mut skill = simulator.masters.make_skill(name, *level);
        skill.is_fixed = true;
        condition.add_skill(&skill);
    }
    if let Some(weapon) = weapon {
        condition.is_specific_weapon = true;
        condition.weapon_name = Some(weapon.to_string());
    }
    condition
}

#[test]
fn search_normal() {
    let (_root, simulator) = setup();
    let condition = condition_with(
        &simulator,
        &[("防具スキル", 6), ("つよ防具スキル全", 5)],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let range = build_search_range(&condition, &simulator.masters);
    let mut searcher = Searcher::new(condition.clone(), range).unwrap();
    let all = searcher.exec_search(10, None).unwrap();
    assert_eq!(searcher.result_sets.len(), 1);
    assert!(all);
    let set = &searcher.result_sets[0];
    for skill in &condition.skills {
        let got = set
            .skills()
            .into_iter()
            .find(|s| s.name == skill.name)
            .unwrap();
        assert!(got.level >= skill.level);
    }
    assert_eq!(set.weapon.name, "スロットのみ_0-0-0");
}

#[test]
fn search_limit() {
    let (_root, simulator) = setup();
    for (expected, limit, all_expected) in [(6, 10, true), (3, 3, false)] {
        let condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("つよ防具スキル全", 4)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition.clone(), range).unwrap();
        let all = searcher.exec_search(limit, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected);
        assert_eq!(all, all_expected);
    }
}

#[test]
fn search_zero_or_negative_limit_adds_nothing() {
    let (_root, simulator) = setup();
    let condition = condition_with(
        &simulator,
        &[("防具スキル", 6), ("つよ防具スキル全", 5)],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let range = build_search_range(&condition, &simulator.masters);
    let mut searcher = Searcher::new(condition, range).unwrap();
    assert!(!searcher.exec_search(0, None).unwrap());
    assert!(searcher.result_sets.is_empty());
    assert!(!searcher.exec_search(-5, None).unwrap());
    assert!(searcher.result_sets.is_empty());
}

#[test]
fn search_def() {
    let (_root, simulator) = setup();
    for (expected, def, transcending) in [
        (1, 180, true),
        (1, 179, true),
        (7, 174, true),
        (1, 140, false),
        (1, 139, false),
        (0, 141, false),
        (6, 132, false),
    ] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        condition.is_transcending = transcending;
        condition.def = Some(def);
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(10, None).unwrap();
        assert_eq!(
            searcher.result_sets.len(),
            expected,
            "def={} transcending={}",
            def,
            transcending
        );
    }
}

#[test]
fn search_fix() {
    let (_root, simulator) = setup();
    let condition = condition_with(
        &simulator,
        &[("防具スキル", 6)],
        &[("つよ防具スキル全", 4)],
        Some("スロットのみ_0-0-0"),
    );
    let range = build_search_range(&condition, &simulator.masters);
    let mut searcher = Searcher::new(condition.clone(), range).unwrap();
    searcher.exec_search(10, None).unwrap();
    assert_eq!(searcher.result_sets.len(), 5);
    for set in &searcher.result_sets {
        let got = set
            .skills()
            .into_iter()
            .find(|s| s.name == "つよ防具スキル全")
            .unwrap();
        assert_eq!(got.level, 4);
    }
}

#[test]
fn search_weapon_type() {
    let (_root, simulator) = setup();
    let mut condition = condition_with(
        &simulator,
        &[
            ("防具スキル", 6),
            ("つよ防具スキル全", 5),
            ("武器スキル", 1),
        ],
        &[],
        None,
    );
    condition.weapon_type = WeaponType::GreatSword;
    let range = build_search_range(&condition, &simulator.masters);
    let mut searcher = Searcher::new(condition, range).unwrap();
    searcher.exec_search(10, None).unwrap();
    assert_eq!(searcher.result_sets.len(), 1);
}

#[test]
fn search_regist() {
    let (_root, simulator) = setup();
    for (expected, attack) in [(1, 0), (3, 10), (2, 95)] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("つよ防具スキル全", 5)],
            &[],
            None,
        );
        condition.weapon_type = WeaponType::GreatSword;
        condition.min_attack = Some(attack);
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(10, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "attack={}", attack);
    }
}

#[test]
fn search_group_fix() {
    let (_root, simulator) = setup();
    for (expected, fix_level) in [(17, 0), (16, 3)] {
        let condition = condition_with(
            &simulator,
            &[("防具スキル", 6)],
            &[("つよグループ", fix_level)],
            Some("スロットのみ_0-0-0"),
        );
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(20, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "fix={}", fix_level);
    }
}

#[test]
fn search_series_fix() {
    let (_root, simulator) = setup();
    for (expected, fix_level) in [(7, 0), (20, 2), (6, 4)] {
        let condition = condition_with(
            &simulator,
            &[("防具スキル", 6)],
            &[("つよの力", fix_level)],
            Some("スロットのみ_0-0-0"),
        );
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(20, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "fix={}", fix_level);
    }
}

#[test]
fn search_transcending() {
    let (_root, simulator) = setup();
    for (expected, transcending) in [(0, false), (1, true)] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("スロ3防具スキル", 5)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        condition.is_transcending = transcending;
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(10, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected);
    }
}

#[test]
fn search_best_charm() {
    let (_root, simulator) = setup();
    for (expected, best) in [(0, false), (1, true)] {
        let mut condition = condition_with(
            &simulator,
            &[("つよ防具スキル全", 6), ("スロ1武器スキル", 1)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        condition.is_best_charm_search = best;
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(10, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "best={}", best);
    }
}

#[test]
fn search_best_artian() {
    let (_root, simulator) = setup();
    for (expected, best) in [(0, false), (10, true)] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("つよグループ", 3), ("よわの力", 4)],
            &[],
            None,
        );
        condition.weapon_type = WeaponType::GreatSword;
        condition.is_best_artian_search = best;
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(20, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "best={}", best);
    }
}

#[test]
fn search_exclude_include() {
    let (_root, mut simulator) = setup();
    simulator.masters.cludes.push(Clude {
        name: "つよ腕".to_string(),
        kind: CludeKind::Exclude,
    });
    let condition = condition_with(
        &simulator,
        &[("防具スキル", 6), ("つよ防具スキル全", 4)],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let range = build_search_range(&condition, &simulator.masters);
    let mut searcher = Searcher::new(condition, range).unwrap();
    searcher.exec_search(20, None).unwrap();
    assert_eq!(searcher.result_sets.len(), 1);
    assert_ne!(searcher.result_sets[0].arm.name, "つよ腕");

    let (_root2, mut simulator2) = setup();
    simulator2.masters.cludes.push(Clude {
        name: "よわ腕".to_string(),
        kind: CludeKind::Include,
    });
    let condition2 = condition_with(
        &simulator2,
        &[("防具スキル", 6), ("つよ防具スキル全", 4)],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let range2 = build_search_range(&condition2, &simulator2.masters);
    let mut searcher2 = Searcher::new(condition2, range2).unwrap();
    searcher2.exec_search(20, None).unwrap();
    assert_eq!(searcher2.result_sets.len(), 1);
    assert_eq!(searcher2.result_sets[0].arm.name, "よわ腕");
}

#[test]
fn search_deco_count() {
    let (_root, simulator) = setup();
    for (expected, all_decos) in [(0, false), (1, true)] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("スロ1不足スキル", 5)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        condition.has_all_decos = all_decos;
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(20, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "all={}", all_decos);
    }
}

#[test]
fn search_deco() {
    let (_root, simulator) = setup();
    for (expected, deco_skill) in [
        (2, "スロ3防具スキル"),
        (2, "スロ3武器スキル"),
        (3, "スロ3両対応スキル"),
        (1, "スロ3両要求スキル"),
    ] {
        let condition = condition_with(
            &simulator,
            &[("防具スキル", 5), ("つよ防具スキル全", 5), (deco_skill, 1)],
            &[],
            Some("スロットのみ_0-0-0"),
        );
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(20, None).unwrap();
        assert_eq!(searcher.result_sets.len(), expected, "{}", deco_skill);
    }
}

#[test]
fn search_invalid_names() {
    let (_root, simulator) = setup();
    for weapon in [None, Some(""), Some("dummy")] {
        let mut condition = condition_with(
            &simulator,
            &[("防具スキル", 6), ("つよ防具スキル全", 5)],
            &[],
            None,
        );
        condition.is_specific_weapon = true;
        condition.weapon_name = weapon.map(|s| s.to_string());
        let range = build_search_range(&condition, &simulator.masters);
        let mut searcher = Searcher::new(condition, range).unwrap();
        searcher.exec_search(10, None).unwrap();
        assert!(searcher.result_sets.is_empty());
    }
}

#[test]
fn simulator_search_more() {
    let (_root, mut simulator) = setup();
    let condition = condition_with(
        &simulator,
        &[("防具スキル", 6), ("つよ防具スキル全", 4)],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let result1 = simulator.search(condition, 3, None).unwrap();
    assert_eq!(result1.len(), 3);
    let result2 = simulator.search_more(2, None).unwrap();
    assert_eq!(result2.len(), 5);
    let result3 = simulator.search_more(2, None).unwrap();
    assert_eq!(result3.len(), 6);
}

#[test]
fn simulator_extra_skill() {
    let (_root, mut simulator) = setup();
    let condition = condition_with(
        &simulator,
        &[
            ("防具スキル", 6),
            ("よわグループ", 3),
            ("よわの力", 2),
            ("つよの力", 2),
        ],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let progress = std::sync::atomic::AtomicUsize::new(0);
    let result = simulator
        .search_extra_skill(&condition, &cancel, Some(&progress))
        .unwrap();
    assert_eq!(result.len(), 10);
    for (name, level) in [
        ("つよ防具スキル全", 1),
        ("つよ防具スキル全", 2),
        ("つよ防具スキル頭", 1),
        ("つよ防具スキル胴", 1),
        ("つよ防具スキル腕", 1),
        ("つよ防具スキル腰", 1),
        ("つよ防具スキル足", 1),
        ("よわの力", 3),
        ("つよグループ", 1),
        ("つよグループ", 2),
    ] {
        assert!(
            result.iter().any(|s| s.name == name && s.level == level),
            "{} Lv{}",
            name,
            level
        );
    }
    assert_eq!(
        progress.load(std::sync::atomic::Ordering::Relaxed),
        simulator.masters.skills.len()
    );
}

#[test]
fn simulator_charm_search() {
    let (_root, mut simulator) = setup();
    let mut condition = condition_with(
        &simulator,
        &[
            ("防具スキル", 6),
            ("つよ防具スキル全", 1),
            ("スロ1防具スキル", 1),
        ],
        &[],
        Some("スロットのみ_0-0-0"),
    );
    simulator.masters.cludes.push(Clude {
        name: "テスト護石Ⅰ".to_string(),
        kind: CludeKind::Include,
    });
    let result1 = simulator.search(condition.clone(), 10, None).unwrap();
    assert!(result1.is_empty());
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result2 = simulator.search_charm(&cancel, None).unwrap();
    assert_eq!(result2.len(), 1);
    assert!(result2[0].charm.is_virtual);
    condition.skills.clear();
}

#[test]
fn simulator_charm_without_search() {
    let (_root, mut simulator) = setup();
    let cancel = std::sync::atomic::AtomicBool::new(false);
    let result = simulator.search_charm(&cancel, None).unwrap();
    assert!(result.is_empty());
}
