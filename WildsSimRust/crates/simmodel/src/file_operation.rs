use std::collections::HashMap;
use std::path::{Path, PathBuf};

use uuid::Uuid;

use crate::config::LogicConfig;
use crate::csv_util::{write_csv, CsvRow, CsvTable};
use crate::error::{SimError, SimResult};
use crate::model::*;
use crate::parse_util::{parse_i32, parse_i32_default};

pub const SKILL_CSV: &str = "MHWilds_SKILL.csv";
pub const HEAD_CSV: &str = "MHWilds_EQUIP_HEAD.csv";
pub const BODY_CSV: &str = "MHWilds_EQUIP_BODY.csv";
pub const ARM_CSV: &str = "MHWilds_EQUIP_ARM.csv";
pub const WAIST_CSV: &str = "MHWilds_EQUIP_WST.csv";
pub const LEG_CSV: &str = "MHWilds_EQUIP_LEG.csv";
pub const CHARM_CSV: &str = "MHWilds_CHARM.csv";
pub const DECO_CSV: &str = "MHWilds_DECO.csv";
pub const WEAPON_CSV: &str = "MHWilds_WEAPON.csv";
pub const SHINING_CHARM_COMBO_CSV: &str = "MHWilds_COMBO_SHININGCHARM.csv";
pub const SHINING_CHARM_GROUP_CSV: &str = "MHWilds_GROUP_SHININGCHARM.csv";
pub const DEF_UPGRADE_CSV: &str = "MHWilds_DEF_UPGRADE.csv";
pub const SAVE_FOLDER: &str = "save";
pub const DECO_COUNT_JSON: &str = "save/decocount.json";
pub const CLUDE_CSV: &str = "save/clude.csv";
pub const MY_SET_CSV: &str = "save/myset.csv";
pub const RECENT_SKILL_CSV: &str = "save/recentSkill.csv";
pub const CONDITION_CSV: &str = "save/condition.csv";
pub const ADDITIONAL_CHARM_CSV: &str = "save/additionalCharm.csv";
pub const ARTIAN_CSV: &str = "save/artian.csv";

#[derive(Clone, Debug)]
pub struct FileOperation {
    pub root: PathBuf,
    pub logic: LogicConfig,
}

impl FileOperation {
    pub fn new(root: impl Into<PathBuf>, logic: LogicConfig) -> FileOperation {
        FileOperation {
            root: root.into(),
            logic,
        }
    }

    fn path(&self, relative: &str) -> PathBuf {
        self.root.join(relative)
    }

    pub fn read_all_text(&self, relative: &str, required: bool) -> SimResult<String> {
        let path = self.path(relative);
        match std::fs::read(&path) {
            Ok(bytes) => {
                let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
                    &bytes[3..]
                } else {
                    &bytes[..]
                };
                let mut text = String::from_utf8_lossy(bytes).into_owned();
                if let Some(stripped) = text.strip_prefix('#') {
                    text = stripped.to_string();
                }
                Ok(text)
            }
            Err(_) if !required => Ok(String::new()),
            Err(error) => Err(SimError(format!(
                "ファイル {} の読み込みに失敗しました。: {}",
                path.display(),
                error
            ))),
        }
    }

    pub fn make_save_folder(&self) -> SimResult<()> {
        let path = self.path(SAVE_FOLDER);
        if !path.exists() {
            std::fs::create_dir_all(&path)?;
        }
        Ok(())
    }

    fn load_table(&self, relative: &str, required: bool) -> SimResult<CsvTable> {
        let text = self.read_all_text(relative, required)?;
        Ok(CsvTable::parse(&text))
    }

    pub fn load_skill_csv(&self) -> SimResult<Vec<Skill>> {
        let table = self.load_table(SKILL_CSV, true)?;
        let mut skills: Vec<Skill> = Vec::new();
        let mut indexes: HashMap<String, usize> = HashMap::new();
        for row in table.rows.iter() {
            let name = get(&table, row, "スキル系統", SKILL_CSV)?;
            let points = parse_i32_default(get(&table, row, "必要ポイント", SKILL_CSV)?);
            let skill_index = match indexes.get(name).copied() {
                Some(index) => index,
                None => {
                    let category = get(&table, row, "カテゴリ", SKILL_CSV)?.to_string();
                    let can_with_artian =
                        parse_i32_default(get(&table, row, "アーティア対応", SKILL_CSV)?) == 1;
                    let index = skills.len();
                    skills.push(Skill::new_master(
                        name.to_string(),
                        points,
                        category,
                        can_with_artian,
                    ));
                    indexes.insert(name.to_string(), index);
                    index
                }
            };
            let skill = &mut skills[skill_index];
            skill.level = skill.level.max(points);
            skill.max_level = skill.level;
            let activated = get(&table, row, "発動スキル", SKILL_CSV)?;
            if !activated.trim().is_empty() {
                skill
                    .specific_names
                    .entry(points)
                    .or_insert_with(|| activated.to_string());
            }
        }
        Ok(skills)
    }

    pub fn load_weapon_csv(&self, lookup: &SkillLookup) -> SimResult<Vec<Equipment>> {
        let mut weapons = Vec::new();
        let max_size = self.logic.max_slot_size;
        for i in 0..=max_size {
            for j in 0..=i {
                for k in 0..=j {
                    let mut weapon = Equipment::new_weapon();
                    weapon.name = format!("スロットのみ_{}-{}-{}", i, j, k);
                    weapon.slot1 = i;
                    weapon.slot2 = j;
                    weapon.slot3 = k;
                    weapon.slot_type1 = 1;
                    weapon.slot_type2 = 1;
                    weapon.slot_type3 = 1;
                    weapons.push(weapon);
                }
            }
        }
        let table = self.load_table(WEAPON_CSV, true)?;
        for row in table.rows.iter() {
            let period = get(&table, row, "入手時期", WEAPON_CSV)?;
            if period == "99" && !self.logic.allow_unavailable_equipments {
                continue;
            }
            let weapon_type_name = get(&table, row, "武器種", WEAPON_CSV)?;
            let weapon_type = WeaponType::from_str(weapon_type_name).ok_or_else(|| {
                SimError(format!(
                    "ファイル {} の読み込みに失敗しました。武器種 {} が不正です。",
                    WEAPON_CSV, weapon_type_name
                ))
            })?;
            let mut weapon = Equipment::new_weapon();
            weapon.weapon_type = weapon_type;
            weapon.name = get(&table, row, "名前", WEAPON_CSV)?.to_string();
            weapon.rare = parse_i32_default(get(&table, row, "レア度", WEAPON_CSV)?);
            weapon.slot1 = parse_i32_default(get(&table, row, "スロット1", WEAPON_CSV)?);
            weapon.slot2 = parse_i32_default(get(&table, row, "スロット2", WEAPON_CSV)?);
            weapon.slot3 = parse_i32_default(get(&table, row, "スロット3", WEAPON_CSV)?);
            weapon.slot_type1 = 1;
            weapon.slot_type2 = 1;
            weapon.slot_type3 = 1;
            weapon.mindef = parse_i32_default(get(&table, row, "防御ボーナス", WEAPON_CSV)?);
            weapon.maxdef = weapon.mindef;
            weapon.attack = parse_i32_default(get(&table, row, "表示攻撃力", WEAPON_CSV)?);
            weapon.row_no = parse_i32(get(&table, row, "仮番号", WEAPON_CSV)?, i32::MAX);
            weapon.skills = load_skills(&table, row, lookup, WEAPON_CSV)?;
            weapons.push(weapon);
        }
        Ok(weapons)
    }

    pub fn load_equip_csv(
        &self,
        file_name: &str,
        kind: EquipKind,
        def_upgrades: Option<&HashMap<i32, DefUpgrade>>,
        lookup: &SkillLookup,
    ) -> SimResult<Vec<Equipment>> {
        let table = self.load_table(file_name, true)?;
        let mut equips = Vec::new();
        for row in table.rows.iter() {
            let period = get(&table, row, "入手時期", file_name)?;
            if period == "99" && !self.logic.allow_unavailable_equipments {
                continue;
            }
            let mut equip = Equipment::new(kind);
            equip.name = get(&table, row, "名前", file_name)?.to_string();
            equip.rare = parse_i32_default(get(&table, row, "レア度", file_name)?);
            if kind != EquipKind::Charm {
                equip.slot1 = parse_i32_default(get(&table, row, "スロット1", file_name)?);
                equip.slot2 = parse_i32_default(get(&table, row, "スロット2", file_name)?);
                equip.slot3 = parse_i32_default(get(&table, row, "スロット3", file_name)?);
                equip.mindef = parse_i32_default(get(&table, row, "初期防御力", file_name)?);
                let maxdef = calc_maxdef(equip.rare, equip.mindef, kind, def_upgrades);
                equip.maxdef = parse_i32(get(&table, row, "最終防御力", file_name)?, maxdef);
                equip.transcending_def = Some(calc_transcending_def(
                    equip.rare,
                    equip.maxdef,
                    kind,
                    def_upgrades,
                ));
                equip.fire = parse_i32_default(get(&table, row, "火耐性", file_name)?);
                equip.water = parse_i32_default(get(&table, row, "水耐性", file_name)?);
                equip.thunder = parse_i32_default(get(&table, row, "雷耐性", file_name)?);
                equip.ice = parse_i32_default(get(&table, row, "氷耐性", file_name)?);
                equip.dragon = parse_i32_default(get(&table, row, "龍耐性", file_name)?);
            }
            equip.is_one_set =
                table.has_column("ワンセット") && get_opt(&table, row, "ワンセット") == Some("1");
            equip.row_no = parse_i32(get(&table, row, "仮番号", file_name)?, i32::MAX);
            equip.skills = load_skills(&table, row, lookup, file_name)?;
            equips.push(equip);
        }
        Ok(equips)
    }

    pub fn load_deco_csv(&self, lookup: &SkillLookup) -> SimResult<Vec<Equipment>> {
        let table = self.load_table(DECO_CSV, true)?;
        let mut decos = Vec::new();
        for row in table.rows.iter() {
            let period = get(&table, row, "入手時期", DECO_CSV)?;
            if period == "99" && !self.logic.allow_unavailable_equipments {
                continue;
            }
            let mut deco = Equipment::new_deco();
            deco.name = get(&table, row, "名前", DECO_CSV)?.to_string();
            deco.rare = parse_i32_default(get(&table, row, "レア度", DECO_CSV)?);
            deco.slot1 = parse_i32_default(get(&table, row, "スロットサイズ", DECO_CSV)?);
            deco.slot2 = 0;
            deco.slot3 = 0;
            deco.slot_type1 = parse_i32_default(get(&table, row, "スロットタイプ", DECO_CSV)?);
            deco.skills = load_skills(&table, row, lookup, DECO_CSV)?;
            deco.deco_count = if deco.slot1 == 4 { 0 } else { 7 };
            if deco.skills.len() > 1 {
                deco.deco_category = Some(format!("{}複合", deco.skills[0].name));
            }
            decos.push(deco);
        }
        self.load_deco_count_json(&mut decos)?;
        Ok(decos)
    }

    fn load_deco_count_json(&self, decos: &mut [Equipment]) -> SimResult<()> {
        let text = self.read_all_text(DECO_COUNT_JSON, false)?;
        if text.trim().is_empty() {
            return Ok(());
        }
        let counts: HashMap<String, i32> = serde_json::from_str(&text).map_err(|error| {
            SimError(format!(
                "ファイル {} の読み込みに失敗しました。: {}",
                DECO_COUNT_JSON, error
            ))
        })?;
        for deco in decos.iter_mut() {
            if let Some(count) = counts.get(&deco.name) {
                deco.deco_count = *count;
            }
        }
        Ok(())
    }

    pub fn save_deco_count_json(&self, decos: &[Equipment]) -> SimResult<()> {
        // Keep the master order: the C# original serializes a Dictionary, which
        // preserves insertion order, so a sorted map would rewrite the file.
        let mut text = String::from("{");
        for (index, deco) in decos.iter().enumerate() {
            if index > 0 {
                text.push(',');
            }
            let name = serde_json::to_string(&deco.name)
                .map_err(|error| SimError(format!("装飾品名の変換に失敗しました。: {}", error)))?;
            text.push_str(&name);
            text.push(':');
            text.push_str(&deco.deco_count.to_string());
        }
        text.push('}');
        let path = self.path(DECO_COUNT_JSON);
        write_atomic(&path, text.as_bytes()).map_err(|error| {
            SimError(format!(
                "ファイル {} への書き込みに失敗しました。: {}",
                path.display(),
                error
            ))
        })
    }

    pub fn load_clude_csv(&self) -> SimResult<Vec<Clude>> {
        let table = self.load_table(CLUDE_CSV, false)?;
        let mut cludes = Vec::new();
        for row in table.rows.iter() {
            let kind = match parse_i32_default(get(&table, row, "種別", CLUDE_CSV)?) {
                1 => CludeKind::Include,
                _ => CludeKind::Exclude,
            };
            cludes.push(Clude {
                name: get(&table, row, "対象", CLUDE_CSV)?.to_string(),
                kind,
            });
        }
        Ok(cludes)
    }

    pub fn save_clude_csv(&self, cludes: &[Clude]) -> SimResult<()> {
        let headers = vec!["対象".to_string(), "種別".to_string()];
        let rows = cludes
            .iter()
            .map(|clude| vec![clude.name.clone(), clude.kind.to_string()])
            .collect::<Vec<_>>();
        let text = write_csv(&headers, &rows)?;
        self.write_text(CLUDE_CSV, &text)
    }

    pub fn load_my_set_csv(&self, equips: &[Equipment]) -> SimResult<Vec<EquipSet>> {
        let table = self.load_table(MY_SET_CSV, false)?;
        let mut sets = Vec::new();
        for row in table.rows.iter() {
            let mut set = EquipSet::new();
            let weapon_name = get(&table, row, "武器", MY_SET_CSV)?;
            set.weapon = match get_equip_by_name(weapon_name, equips) {
                Some(equip) if equip.kind == EquipKind::Weapon => equip.clone(),
                _ => {
                    let mut weapon = Equipment::new_weapon();
                    weapon.name = weapon_name.to_string();
                    weapon
                }
            };
            set.head = get_equip_by_name(get(&table, row, "頭", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "頭").unwrap_or("").to_string();
                    equip
                });
            set.body = get_equip_by_name(get(&table, row, "胴", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "胴").unwrap_or("").to_string();
                    equip
                });
            set.arm = get_equip_by_name(get(&table, row, "腕", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "腕").unwrap_or("").to_string();
                    equip
                });
            set.waist = get_equip_by_name(get(&table, row, "腰", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "腰").unwrap_or("").to_string();
                    equip
                });
            set.leg = get_equip_by_name(get(&table, row, "足", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "足").unwrap_or("").to_string();
                    equip
                });
            set.charm = get_equip_by_name(get(&table, row, "護石", MY_SET_CSV)?, equips)
                .cloned()
                .unwrap_or_else(|| {
                    let mut equip = Equipment::default();
                    equip.name = get_opt(&table, row, "護石").unwrap_or("").to_string();
                    equip
                });
            set.head.kind = EquipKind::Head;
            set.body.kind = EquipKind::Body;
            set.arm.kind = EquipKind::Arm;
            set.waist.kind = EquipKind::Waist;
            set.leg.kind = EquipKind::Leg;
            set.charm.kind = EquipKind::Charm;
            set.decos = decos_from_csv(get(&table, row, "装飾品", MY_SET_CSV)?, equips);
            set.sort_decos();
            set.name = get(&table, row, "名前", MY_SET_CSV)?.to_string();
            set.is_transcending = table.has_column("限界突破有無")
                && get_opt(&table, row, "限界突破有無") == Some("1");
            sets.push(set);
        }
        Ok(sets)
    }

    pub fn save_my_set_csv(&self, my_sets: &[EquipSet]) -> SimResult<()> {
        let headers = [
            "武器",
            "頭",
            "胴",
            "腕",
            "腰",
            "足",
            "護石",
            "装飾品",
            "名前",
            "限界突破有無",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
        let rows = my_sets
            .iter()
            .map(|set| {
                vec![
                    set.weapon.name.clone(),
                    set.head.name.clone(),
                    set.body.name.clone(),
                    set.arm.name.clone(),
                    set.waist.name.clone(),
                    set.leg.name.clone(),
                    set.charm.name.clone(),
                    set.deco_name_csv(),
                    set.name.clone(),
                    if set.is_transcending { "1" } else { "" }.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let text = write_csv(&headers, &rows)?;
        self.write_text(MY_SET_CSV, &text)
    }

    pub fn load_recent_skill_csv(&self) -> SimResult<Vec<String>> {
        let table = self.load_table(RECENT_SKILL_CSV, false)?;
        let mut names = Vec::new();
        for row in table.rows.iter() {
            names.push(get(&table, row, "スキル名", RECENT_SKILL_CSV)?.to_string());
        }
        Ok(names)
    }

    pub fn save_recent_skill_csv(&self, names: &[String]) -> SimResult<()> {
        let headers = vec!["スキル名".to_string()];
        let rows = names
            .iter()
            .map(|name| vec![name.clone()])
            .collect::<Vec<_>>();
        let text = write_csv(&headers, &rows)?;
        self.write_text(RECENT_SKILL_CSV, &text)
    }

    pub fn load_my_condition_csv(&self, lookup: &SkillLookup) -> SimResult<Vec<SearchCondition>> {
        let table = self.load_table(CONDITION_CSV, false)?;
        let mut conditions = Vec::new();
        for row in table.rows.iter() {
            let mut condition = SearchCondition::new();
            condition.id = Some(get(&table, row, "ID", CONDITION_CSV)?.to_string());
            condition.disp_name = Some(get(&table, row, "名前", CONDITION_CSV)?.to_string());
            condition.is_specific_weapon = parse_bool_field(
                get(&table, row, "武器指定有無", CONDITION_CSV)?,
                CONDITION_CSV,
            )?;
            let weapon_type_name = get(&table, row, "武器種", CONDITION_CSV)?;
            condition.weapon_type = WeaponType::from_str(weapon_type_name).ok_or_else(|| {
                SimError(format!(
                    "ファイル {} の読み込みに失敗しました。武器種 {} が不正です。",
                    CONDITION_CSV, weapon_type_name
                ))
            })?;
            if condition.is_specific_weapon {
                condition.weapon_name =
                    Some(get(&table, row, "武器名", CONDITION_CSV)?.to_string());
            } else {
                condition.min_attack =
                    parse_optional_i32(get(&table, row, "攻撃力", CONDITION_CSV)?);
            }
            condition.def = parse_optional_i32(get(&table, row, "防御力", CONDITION_CSV)?);
            condition.fire = parse_optional_i32(get(&table, row, "火耐性", CONDITION_CSV)?);
            condition.water = parse_optional_i32(get(&table, row, "水耐性", CONDITION_CSV)?);
            condition.thunder = parse_optional_i32(get(&table, row, "雷耐性", CONDITION_CSV)?);
            condition.ice = parse_optional_i32(get(&table, row, "氷耐性", CONDITION_CSV)?);
            condition.dragon = parse_optional_i32(get(&table, row, "龍耐性", CONDITION_CSV)?);
            condition.set_skill_csv(get(&table, row, "スキル", CONDITION_CSV)?, lookup);
            condition.is_transcending = !table.has_column("限界突破有無")
                || get_opt(&table, row, "限界突破有無") == Some("1");
            conditions.push(condition);
        }
        Ok(conditions)
    }

    pub fn save_my_condition_csv(&self, conditions: &[SearchCondition]) -> SimResult<()> {
        let headers = [
            "ID",
            "名前",
            "武器指定有無",
            "武器名",
            "武器種",
            "攻撃力",
            "防御力",
            "火耐性",
            "水耐性",
            "雷耐性",
            "氷耐性",
            "龍耐性",
            "スキル",
            "限界突破有無",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
        let rows = conditions
            .iter()
            .map(|condition| {
                vec![
                    condition
                        .id
                        .clone()
                        .unwrap_or_else(|| Uuid::new_v4().to_string()),
                    condition.disp_name.clone().unwrap_or_default(),
                    if condition.is_specific_weapon {
                        "True"
                    } else {
                        "False"
                    }
                    .to_string(),
                    condition
                        .weapon_name
                        .clone()
                        .unwrap_or_else(|| "null".to_string()),
                    condition.weapon_type.to_string(),
                    option_string(condition.min_attack),
                    option_string(condition.def),
                    option_string(condition.fire),
                    option_string(condition.water),
                    option_string(condition.thunder),
                    option_string(condition.ice),
                    option_string(condition.dragon),
                    condition.skill_csv(),
                    if condition.is_transcending { "1" } else { "" }.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        let text = write_csv(&headers, &rows)?;
        self.write_text(CONDITION_CSV, &text)
    }

    pub fn load_additional_charm_csv(&self, lookup: &SkillLookup) -> SimResult<Vec<Equipment>> {
        let table = self.load_table(ADDITIONAL_CHARM_CSV, false)?;
        let mut charms = Vec::new();
        for row in table.rows.iter() {
            let mut charm = Equipment::new(EquipKind::Charm);
            charm.name = match get_opt(&table, row, "内部管理ID") {
                Some(name) if !name.trim().is_empty() => name.to_string(),
                _ => Uuid::new_v4().to_string(),
            };
            let has_slot_values = [
                "スロット1",
                "スロット2",
                "スロット3",
                "スロット1タイプ",
                "スロット2タイプ",
                "スロット3タイプ",
            ]
            .iter()
            .all(|name| get_opt(&table, row, name).is_some());
            if has_slot_values {
                charm.slot1 =
                    parse_i32_default(get(&table, row, "スロット1", ADDITIONAL_CHARM_CSV)?);
                charm.slot2 =
                    parse_i32_default(get(&table, row, "スロット2", ADDITIONAL_CHARM_CSV)?);
                charm.slot3 =
                    parse_i32_default(get(&table, row, "スロット3", ADDITIONAL_CHARM_CSV)?);
                charm.slot_type1 =
                    parse_i32_default(get(&table, row, "スロット1タイプ", ADDITIONAL_CHARM_CSV)?);
                charm.slot_type2 =
                    parse_i32_default(get(&table, row, "スロット2タイプ", ADDITIONAL_CHARM_CSV)?);
                charm.slot_type3 =
                    parse_i32_default(get(&table, row, "スロット3タイプ", ADDITIONAL_CHARM_CSV)?);
            } else {
                let mut slots_data: Vec<(i32, i32)> = Vec::new();
                for (name, slot_type) in [
                    ("(泣用武器スロ1)", 1),
                    ("(泣用武器スロ2)", 1),
                    ("(泣用武器スロ3)", 1),
                    ("(泣用防具スロ1)", 0),
                    ("(泣用防具スロ2)", 0),
                    ("(泣用防具スロ3)", 0),
                ] {
                    let size = parse_i32_default(get(&table, row, name, ADDITIONAL_CHARM_CSV)?);
                    if size != 0 {
                        slots_data.push((size, slot_type));
                    }
                }
                if let Some((size, slot_type)) = slots_data.first() {
                    charm.slot1 = *size;
                    charm.slot_type1 = *slot_type;
                }
                if let Some((size, slot_type)) = slots_data.get(1) {
                    charm.slot2 = *size;
                    charm.slot_type2 = *slot_type;
                }
                if let Some((size, slot_type)) = slots_data.get(2) {
                    charm.slot3 = *size;
                    charm.slot_type3 = *slot_type;
                }
            }
            charm.skills = load_skills_limited(
                &table,
                row,
                lookup,
                self.logic.max_charm_skill_count,
                ADDITIONAL_CHARM_CSV,
            )?;
            charm.set_charm_disp_name();
            charms.push(charm);
        }
        Ok(charms)
    }

    pub fn save_additional_charm_csv(
        &self,
        charms: &[Equipment],
        my_sets: &[EquipSet],
    ) -> SimResult<()> {
        let mut headers = Vec::new();
        for i in 1..=self.logic.max_charm_skill_count {
            headers.push(format!("スキル系統{}", i));
            headers.push(format!("スキル値{}", i));
        }
        for name in [
            "(泣用防具スロ1)",
            "(泣用防具スロ2)",
            "(泣用防具スロ3)",
            "(泣用武器スロ1)",
            "(泣用武器スロ2)",
            "(泣用武器スロ3)",
            "スロット1",
            "スロット2",
            "スロット3",
            "スロット1タイプ",
            "スロット2タイプ",
            "スロット3タイプ",
            "内部管理ID",
            "マイセット登録有無",
        ] {
            headers.push(name.to_string());
        }
        let mut rows = Vec::new();
        for charm in charms {
            let mut row = Vec::new();
            for i in 0..self.logic.max_charm_skill_count {
                let index = i as usize;
                row.push(
                    charm
                        .skills
                        .get(index)
                        .map(|s| s.name.clone())
                        .unwrap_or_default(),
                );
                row.push(
                    charm
                        .skills
                        .get(index)
                        .map(|s| s.level.to_string())
                        .unwrap_or_default(),
                );
            }
            let mut w_slot = Vec::new();
            let mut a_slot = Vec::new();
            for (size, slot_type) in [
                (charm.slot1, charm.slot_type1),
                (charm.slot2, charm.slot_type2),
                (charm.slot3, charm.slot_type3),
            ] {
                if slot_type == 1 {
                    w_slot.push(size);
                } else if slot_type == 0 {
                    a_slot.push(size);
                }
            }
            while w_slot.len() < 3 {
                w_slot.push(0);
            }
            while a_slot.len() < 3 {
                a_slot.push(0);
            }
            for value in a_slot {
                row.push(value.to_string());
            }
            for value in w_slot {
                row.push(value.to_string());
            }
            row.push(charm.slot1.to_string());
            row.push(charm.slot2.to_string());
            row.push(charm.slot3.to_string());
            row.push(charm.slot_type1.to_string());
            row.push(charm.slot_type2.to_string());
            row.push(charm.slot_type3.to_string());
            row.push(charm.name.clone());
            row.push(if my_sets.iter().any(|set| set.charm.name == charm.name) {
                "マイセット登録中".to_string()
            } else {
                String::new()
            });
            rows.push(row);
        }
        let text = write_csv(&headers, &rows)?;
        self.write_text(ADDITIONAL_CHARM_CSV, &text)
    }

    pub fn load_shining_charm_combo_csv(&self) -> SimResult<Vec<CharmCombo>> {
        let table = self.load_table(SHINING_CHARM_COMBO_CSV, true)?;
        let mut combos = Vec::new();
        for row in table.rows.iter() {
            let mut combo = CharmCombo::default();
            combo.rare = parse_i32_default(get(&table, row, "レア度", SHINING_CHARM_COMBO_CSV)?);
            combo.group1 =
                parse_i32_default(get(&table, row, "グループ1", SHINING_CHARM_COMBO_CSV)?);
            combo.group2 =
                parse_i32_default(get(&table, row, "グループ2", SHINING_CHARM_COMBO_CSV)?);
            combo.group3 =
                parse_i32_default(get(&table, row, "グループ3", SHINING_CHARM_COMBO_CSV)?);
            combo.slot1 =
                parse_i32_default(get(&table, row, "スロット1", SHINING_CHARM_COMBO_CSV)?);
            combo.slot2 =
                parse_i32_default(get(&table, row, "スロット2", SHINING_CHARM_COMBO_CSV)?);
            combo.slot3 =
                parse_i32_default(get(&table, row, "スロット3", SHINING_CHARM_COMBO_CSV)?);
            combo.slot_type1 = parse_i32_default(get(
                &table,
                row,
                "スロット1タイプ",
                SHINING_CHARM_COMBO_CSV,
            )?);
            combo.slot_type2 = parse_i32_default(get(
                &table,
                row,
                "スロット2タイプ",
                SHINING_CHARM_COMBO_CSV,
            )?);
            combo.slot_type3 = parse_i32_default(get(
                &table,
                row,
                "スロット3タイプ",
                SHINING_CHARM_COMBO_CSV,
            )?);
            combos.push(combo);
        }
        Ok(combos)
    }

    pub fn load_shining_charm_group_csv(
        &self,
        lookup: &SkillLookup,
    ) -> SimResult<HashMap<i32, Vec<Skill>>> {
        let table = self.load_table(SHINING_CHARM_GROUP_CSV, true)?;
        let mut groups: HashMap<i32, Vec<Skill>> = HashMap::new();
        groups.insert(0, Vec::new());
        for row in table.rows.iter() {
            let group = parse_i32_default(get(&table, row, "グループ", SHINING_CHARM_GROUP_CSV)?);
            let skill_name = get(&table, row, "スキル名", SHINING_CHARM_GROUP_CSV)?;
            let level = parse_i32_default(get(&table, row, "レベル", SHINING_CHARM_GROUP_CSV)?);
            groups
                .entry(group)
                .or_default()
                .push(lookup.make(skill_name, level));
        }
        Ok(groups)
    }

    pub fn load_def_upgrade_csv(&self) -> SimResult<HashMap<i32, DefUpgrade>> {
        let table = self.load_table(DEF_UPGRADE_CSV, true)?;
        let mut upgrades = HashMap::new();
        for row in table.rows.iter() {
            let rare = parse_i32_default(get(&table, row, "レア度", DEF_UPGRADE_CSV)?);
            if rare != 0 {
                let upgrade = DefUpgrade {
                    upgrade_def: parse_i32_default(get(&table, row, "最大強化", DEF_UPGRADE_CSV)?),
                    transcending_def: parse_i32_default(get(
                        &table,
                        row,
                        "限界突破強化",
                        DEF_UPGRADE_CSV,
                    )?),
                };
                upgrades.insert(rare, upgrade);
            }
        }
        Ok(upgrades)
    }

    pub fn load_artian_csv(&self, lookup: &SkillLookup) -> SimResult<Vec<Equipment>> {
        let table = self.load_table(ARTIAN_CSV, false)?;
        let mut artians = Vec::new();
        for row in table.rows.iter() {
            let mut artian = Equipment::new_weapon();
            artian.init_artian();
            artian.name = match get_opt(&table, row, "内部管理ID") {
                Some(name) if !name.trim().is_empty() => name.to_string(),
                _ => Uuid::new_v4().to_string(),
            };
            let weapon_type_name = get(&table, row, "武器種", ARTIAN_CSV)?;
            artian.weapon_type = WeaponType::from_str(weapon_type_name).ok_or_else(|| {
                SimError(format!(
                    "ファイル {} の読み込みに失敗しました。武器種 {} が不正です。",
                    ARTIAN_CSV, weapon_type_name
                ))
            })?;
            artian.disp_name = Some(get(&table, row, "名前", ARTIAN_CSV)?.to_string());
            artian.skills = load_skills_limited(
                &table,
                row,
                lookup,
                self.logic.artian_skill_count,
                ARTIAN_CSV,
            )?;
            artians.push(artian);
        }
        Ok(artians)
    }

    pub fn save_artian_csv(&self, artians: &[Equipment], my_sets: &[EquipSet]) -> SimResult<()> {
        let mut headers = vec!["武器種".to_string(), "名前".to_string()];
        for i in 1..=self.logic.artian_skill_count {
            headers.push(format!("スキル系統{}", i));
            headers.push(format!("スキル値{}", i));
        }
        headers.push("内部管理ID".to_string());
        headers.push("マイセット登録有無".to_string());
        let mut rows = Vec::new();
        for artian in artians {
            let mut row = vec![
                artian.weapon_type.to_string(),
                artian.disp_name().to_string(),
            ];
            for i in 0..self.logic.artian_skill_count {
                let index = i as usize;
                row.push(
                    artian
                        .skills
                        .get(index)
                        .map(|s| s.name.clone())
                        .unwrap_or_default(),
                );
                row.push(
                    artian
                        .skills
                        .get(index)
                        .map(|s| s.level.to_string())
                        .unwrap_or_default(),
                );
            }
            row.push(artian.name.clone());
            row.push(
                if my_sets.iter().any(|set| set.weapon.name == artian.name) {
                    "マイセット登録中".to_string()
                } else {
                    String::new()
                },
            );
            rows.push(row);
        }
        let text = write_csv(&headers, &rows)?;
        self.write_text(ARTIAN_CSV, &text)
    }

    fn write_text(&self, relative: &str, text: &str) -> SimResult<()> {
        let path = self.path(relative);
        write_atomic(&path, text.as_bytes()).map_err(|error| {
            SimError(format!(
                "ファイル {} への書き込みに失敗しました。: {}",
                path.display(),
                error
            ))
        })
    }
}

// A save file cut off mid-row makes load fail and the app refuse to start, so a crash
// during a save must leave either the old or the new file, never a partial one.
// std::fs::rename replaces the target on Windows (MoveFileExW with REPLACE_EXISTING).
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);
    {
        let mut file = std::fs::File::create(&temp)?;
        std::io::Write::write_all(&mut file, bytes)?;
        file.sync_all()?;
    }
    std::fs::rename(&temp, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temp);
    })
}

fn get<'a>(table: &CsvTable, row: &'a CsvRow, name: &str, file: &str) -> SimResult<&'a str> {
    table.get(row, name).map_err(|error| {
        SimError(format!(
            "ファイル {} の読み込みに失敗しました。: {}",
            file, error
        ))
    })
}

fn get_opt<'a>(table: &CsvTable, row: &'a CsvRow, name: &str) -> Option<&'a str> {
    table.get_opt(row, name)
}

fn parse_optional_i32(value: &str) -> Option<i32> {
    if value == "null" {
        None
    } else {
        Some(parse_i32_default(value))
    }
}

fn option_string(value: Option<i32>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn parse_bool_field(value: &str, file: &str) -> SimResult<bool> {
    if value.eq_ignore_ascii_case("true") {
        Ok(true)
    } else if value.eq_ignore_ascii_case("false") {
        Ok(false)
    } else {
        Err(SimError(format!(
            "ファイル {} の読み込みに失敗しました。真偽値 {} が不正です。",
            file, value
        )))
    }
}

fn calc_maxdef(
    rare: i32,
    mindef: i32,
    kind: EquipKind,
    def_upgrades: Option<&HashMap<i32, DefUpgrade>>,
) -> i32 {
    if kind == EquipKind::Charm {
        return mindef;
    }
    match def_upgrades.and_then(|m| m.get(&rare)) {
        Some(upgrade) => mindef + upgrade.upgrade_def,
        None => mindef,
    }
}

fn calc_transcending_def(
    rare: i32,
    maxdef: i32,
    kind: EquipKind,
    def_upgrades: Option<&HashMap<i32, DefUpgrade>>,
) -> i32 {
    if kind == EquipKind::Charm {
        return maxdef;
    }
    match def_upgrades.and_then(|m| m.get(&rare)) {
        Some(upgrade) => maxdef + upgrade.transcending_def,
        None => maxdef,
    }
}

fn skill_count(table: &CsvTable) -> usize {
    let mut max = 0;
    let mut i = 1;
    while table.has_column(&format!("スキル系統{}", i)) {
        max = i;
        i += 1;
    }
    max as usize
}

fn load_skills(
    table: &CsvTable,
    row: &CsvRow,
    lookup: &SkillLookup,
    file: &str,
) -> SimResult<Vec<Skill>> {
    let count = skill_count(table);
    let mut skills = Vec::new();
    for i in 1..=count {
        let name = get(table, row, &format!("スキル系統{}", i), file)?;
        if name.trim().is_empty() {
            break;
        }
        let level = parse_i32_default(get(table, row, &format!("スキル値{}", i), file)?);
        skills.push(lookup.make(name, level));
    }
    Ok(skills)
}

fn load_skills_limited(
    table: &CsvTable,
    row: &CsvRow,
    lookup: &SkillLookup,
    max_count: i32,
    file: &str,
) -> SimResult<Vec<Skill>> {
    let mut skills = Vec::new();
    for i in 1..=max_count {
        let name = get(table, row, &format!("スキル系統{}", i), file)?;
        if name.trim().is_empty() {
            break;
        }
        let level = parse_i32_default(get(table, row, &format!("スキル値{}", i), file)?);
        skills.push(lookup.make(name, level));
    }
    Ok(skills)
}

fn get_equip_by_name<'a>(name: &str, equips: &'a [Equipment]) -> Option<&'a Equipment> {
    let name = name.trim();
    equips.iter().find(|equip| equip.name == name)
}

fn decos_from_csv(csv: &str, equips: &[Equipment]) -> Vec<Equipment> {
    let mut decos = Vec::new();
    for name in csv.split(',') {
        if name.trim().is_empty() {
            continue;
        }
        if let Some(deco) = get_equip_by_name(name, equips) {
            decos.push(deco.clone());
        }
    }
    decos
}

pub fn data_root_from_env() -> PathBuf {
    if let Ok(root) = std::env::var("WILDSSIM_DATA_DIR") {
        return PathBuf::from(root);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            if dir.join(SKILL_CSV).exists() {
                return dir.to_path_buf();
            }
        }
    }
    Path::new(".").to_path_buf()
}
