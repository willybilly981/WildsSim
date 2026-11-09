use std::path::Path;
use std::time::Instant;

use simmodel::*;

fn main() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let root = std::env::temp_dir().join(format!("wildssim-bench-{}", uuid::Uuid::new_v4()));
    copy_dir(&source, &root);
    let started = Instant::now();
    let simulator = Simulator::load(&root).unwrap();
    println!("load: {:?}", started.elapsed());

    println!(
        "skills={} weapons={}",
        simulator.masters.skills.len(),
        simulator.masters.weapons.len()
    );
    println!(
        "atk={:?} mim={:?}",
        simulator
            .masters
            .skills
            .iter()
            .find(|s| s.name == "攻撃")
            .map(|s| s.level),
        simulator
            .masters
            .skills
            .iter()
            .find(|s| s.name == "見切り")
            .map(|s| s.level)
    );
    let mut condition = SearchCondition::new();
    for (name, level) in [("攻撃", 3), ("見切り", 3)] {
        condition.add_skill(&simulator.masters.make_skill(name, level));
    }
    condition.weapon_type = WeaponType::GreatSword;

    let started = Instant::now();
    let mut searcher = Searcher::new(
        condition.clone(),
        build_search_range(&condition, &simulator.masters),
    )
    .unwrap();
    let all = searcher.exec_search(10, None).unwrap();
    println!(
        "search 10: {:?}, results={}, all={}",
        started.elapsed(),
        searcher.result_sets.len(),
        all
    );
}

fn copy_dir(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).unwrap();
    for entry in std::fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        if entry.file_type().unwrap().is_dir() {
            copy_dir(&entry.path(), &target.join(entry.file_name()));
        } else {
            std::fs::copy(entry.path(), target.join(entry.file_name())).unwrap();
        }
    }
}
