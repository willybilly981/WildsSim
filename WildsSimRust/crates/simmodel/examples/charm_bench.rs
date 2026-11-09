use std::path::Path;
use std::time::Instant;

use simmodel::*;

fn main() {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("data");
    let root = std::env::temp_dir().join(format!("wildssim-charm-bench-{}", uuid::Uuid::new_v4()));
    copy_dir(&source, &root);
    let simulator = Simulator::load(&root).unwrap();

    let count = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(60usize);
    let first = simulator.masters.skills[0].name.clone();
    let second = simulator.masters.skills[1].name.clone();
    let mut charms = Vec::new();
    for index in 0..count {
        let mut charm = Equipment::new(EquipKind::Charm);
        charm.name = format!("bench-charm-{}", index);
        charm.rare = 8;
        charm.slot1 = (index % 3) as i32;
        charm.slot2 = ((index / 3) % 3) as i32;
        charm.slot3 = ((index / 9) % 2) as i32;
        charm.slot_type1 = 0;
        charm.slot_type2 = 1;
        charm.slot_type3 = 0;
        charm
            .skills
            .push(simulator.masters.make_skill(&first, (index % 4 + 1) as i32));
        charm.skills.push(
            simulator
                .masters
                .make_skill(&second, ((index / 4) % 4 + 1) as i32),
        );
        charms.push(charm);
    }
    simulator
        .file
        .save_additional_charm_csv(&charms, &simulator.masters.my_sets)
        .unwrap();

    for threads in [1, 4] {
        write_logic_config(&root, threads);
        let started = Instant::now();
        let loaded = Simulator::load(&root).unwrap();
        let elapsed = started.elapsed();
        let uppers = loaded
            .masters
            .additional_charms
            .iter()
            .filter(|charm| charm.upper.is_some())
            .count();
        println!(
            "charms={} threads={} load={:?} upper={}",
            loaded.masters.additional_charms.len(),
            threads,
            elapsed,
            uppers
        );
    }
}

fn write_logic_config(root: &Path, threads: i32) {
    let conf = root.join("conf").join("LogicConfig.csv");
    let text = format!(
        "スロットの最大の大きさ,最近使ったスキルの記憶容量,入手不可装備の利用有無,下位互換護石の検出有無,最大並列処理数\n3,20,false,true,{}\n",
        threads
    );
    std::fs::write(conf, text).unwrap();
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
