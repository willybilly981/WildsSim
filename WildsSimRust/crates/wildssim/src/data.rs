use std::path::{Path, PathBuf};

const FILES: [(&str, &[u8]); 20] = [
    (
        "MHWilds_CHARM.csv",
        include_bytes!("../../simmodel/data/MHWilds_CHARM.csv"),
    ),
    (
        "MHWilds_COMBO_SHININGCHARM.csv",
        include_bytes!("../../simmodel/data/MHWilds_COMBO_SHININGCHARM.csv"),
    ),
    (
        "MHWilds_DECO.csv",
        include_bytes!("../../simmodel/data/MHWilds_DECO.csv"),
    ),
    (
        "MHWilds_DEF_UPGRADE.csv",
        include_bytes!("../../simmodel/data/MHWilds_DEF_UPGRADE.csv"),
    ),
    (
        "MHWilds_EQUIP_ARM.csv",
        include_bytes!("../../simmodel/data/MHWilds_EQUIP_ARM.csv"),
    ),
    (
        "MHWilds_EQUIP_BODY.csv",
        include_bytes!("../../simmodel/data/MHWilds_EQUIP_BODY.csv"),
    ),
    (
        "MHWilds_EQUIP_HEAD.csv",
        include_bytes!("../../simmodel/data/MHWilds_EQUIP_HEAD.csv"),
    ),
    (
        "MHWilds_EQUIP_LEG.csv",
        include_bytes!("../../simmodel/data/MHWilds_EQUIP_LEG.csv"),
    ),
    (
        "MHWilds_EQUIP_WST.csv",
        include_bytes!("../../simmodel/data/MHWilds_EQUIP_WST.csv"),
    ),
    (
        "MHWilds_GROUP_SHININGCHARM.csv",
        include_bytes!("../../simmodel/data/MHWilds_GROUP_SHININGCHARM.csv"),
    ),
    (
        "MHWilds_SKILL.csv",
        include_bytes!("../../simmodel/data/MHWilds_SKILL.csv"),
    ),
    (
        "MHWilds_WEAPON.csv",
        include_bytes!("../../simmodel/data/MHWilds_WEAPON.csv"),
    ),
    (
        "conf/LogicConfig.csv",
        include_bytes!("../../simmodel/data/conf/LogicConfig.csv"),
    ),
    (
        "conf/viewConfig.csv",
        include_bytes!("../../simmodel/data/conf/viewConfig.csv"),
    ),
    (
        "save/additionalCharm.csv",
        include_bytes!("../../simmodel/data/save/additionalCharm.csv"),
    ),
    (
        "save/artian.csv",
        include_bytes!("../../simmodel/data/save/artian.csv"),
    ),
    (
        "save/clude.csv",
        include_bytes!("../../simmodel/data/save/clude.csv"),
    ),
    (
        "save/condition.csv",
        include_bytes!("../../simmodel/data/save/condition.csv"),
    ),
    (
        "save/myset.csv",
        include_bytes!("../../simmodel/data/save/myset.csv"),
    ),
    (
        "save/recentSkill.csv",
        include_bytes!("../../simmodel/data/save/recentSkill.csv"),
    ),
];

const EXTRACT_DIR: &str = "WildsSimData";

pub fn resolve_data_root() -> PathBuf {
    if let Ok(root) = std::env::var("WILDSSIM_DATA_DIR") {
        return PathBuf::from(root);
    }
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()));
    // A C#-style portable layout next to the exe is user-owned data; use it as is.
    // The current directory is not searched: launching from a folder that happens to
    // hold another copy of the data silently switched where saves were written.
    if let Some(dir) = &exe_dir {
        if dir.join("MHWilds_SKILL.csv").exists() {
            return dir.clone();
        }
    }
    #[cfg(debug_assertions)]
    {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("simmodel")
            .join("data");
        if source.join("MHWilds_SKILL.csv").exists() {
            return source;
        }
    }
    // %TEMP% is last because Storage Sense and cleanup tools delete it, taking the
    // user's charms and sets with it.
    let candidates = [
        exe_dir.map(|dir| dir.join(EXTRACT_DIR)),
        std::env::var_os("LOCALAPPDATA").map(|dir| PathBuf::from(dir).join("WildsSim")),
        Some(std::env::temp_dir().join(EXTRACT_DIR)),
    ];
    let mut last = None;
    for root in candidates.into_iter().flatten() {
        if prepare(&root).is_ok() {
            return root;
        }
        last = Some(root);
    }
    last.unwrap_or_else(|| std::env::temp_dir().join(EXTRACT_DIR))
}

fn is_master(relative: &str) -> bool {
    relative.starts_with("MHWilds_")
}

// Masters are overwritten so a newer build's game data takes effect; conf/ and save/
// are only created when missing because they hold the user's edits.
fn prepare(root: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(root.join("save"))?;
    for (relative, bytes) in FILES.iter() {
        let path = root.join(relative);
        if path.exists() {
            if !is_master(relative) || std::fs::read(&path)? == *bytes {
                continue;
            }
        } else if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        simmodel::file_operation::write_atomic(&path, bytes)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempDir(PathBuf);

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn temp_root() -> TempDir {
        TempDir(std::env::temp_dir().join(format!("wildssim-data-{}", uuid::Uuid::new_v4())))
    }

    fn embedded(relative: &str) -> &'static [u8] {
        FILES.iter().find(|(name, _)| *name == relative).unwrap().1
    }

    #[test]
    fn prepare_extracts_all_files() {
        let root = temp_root();
        prepare(&root.0).unwrap();
        for (relative, bytes) in FILES.iter() {
            assert_eq!(std::fs::read(root.0.join(relative)).unwrap(), *bytes);
        }
    }

    #[test]
    fn prepare_refreshes_stale_masters_only() {
        let root = temp_root();
        prepare(&root.0).unwrap();
        let master = root.0.join("MHWilds_SKILL.csv");
        let conf = root.0.join("conf/LogicConfig.csv");
        let save = root.0.join("save/myset.csv");
        std::fs::write(&master, "stale").unwrap();
        std::fs::write(&conf, "user conf").unwrap();
        std::fs::write(&save, "user save").unwrap();

        prepare(&root.0).unwrap();

        assert_eq!(
            std::fs::read(&master).unwrap(),
            embedded("MHWilds_SKILL.csv")
        );
        assert_eq!(std::fs::read_to_string(&conf).unwrap(), "user conf");
        assert_eq!(std::fs::read_to_string(&save).unwrap(), "user save");
    }
}
