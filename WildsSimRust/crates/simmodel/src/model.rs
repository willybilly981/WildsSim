use std::collections::BTreeMap;
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EquipKind {
    Head,
    Body,
    Arm,
    Waist,
    Leg,
    Deco,
    Charm,
    Weapon,
    Error,
}

impl EquipKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EquipKind::Weapon => "武器",
            EquipKind::Head => "頭",
            EquipKind::Body => "胴",
            EquipKind::Arm => "腕",
            EquipKind::Waist => "腰",
            EquipKind::Leg => "足",
            EquipKind::Deco => "装飾品",
            EquipKind::Charm => "護石",
            EquipKind::Error => "",
        }
    }

    pub fn str_with_colon(self) -> String {
        format!("{}：", self.as_str())
    }

    pub fn from_str(s: &str) -> EquipKind {
        match s {
            "武器" => EquipKind::Weapon,
            "頭" => EquipKind::Head,
            "胴" => EquipKind::Body,
            "腕" => EquipKind::Arm,
            "腰" => EquipKind::Waist,
            "足" => EquipKind::Leg,
            "脚" => EquipKind::Leg,
            "装飾品" => EquipKind::Deco,
            "護石" => EquipKind::Charm,
            _ => EquipKind::Error,
        }
    }

    pub fn is_armor(self) -> bool {
        matches!(
            self,
            EquipKind::Head | EquipKind::Body | EquipKind::Arm | EquipKind::Waist | EquipKind::Leg
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum CludeKind {
    Exclude,
    Include,
}

impl fmt::Display for CludeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CludeKind::Exclude => write!(f, "0"),
            CludeKind::Include => write!(f, "1"),
        }
    }
}

pub const WEAPON_TYPES: [WeaponType; 14] = [
    WeaponType::GreatSword,
    WeaponType::LongSword,
    WeaponType::SwordAndShield,
    WeaponType::DualBlades,
    WeaponType::Lance,
    WeaponType::Gunlance,
    WeaponType::Hammer,
    WeaponType::HuntingHorn,
    WeaponType::SwitchAxe,
    WeaponType::ChargeBlade,
    WeaponType::InsectGlaive,
    WeaponType::LightBowgun,
    WeaponType::HeavyBowgun,
    WeaponType::Bow,
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum WeaponType {
    #[default]
    None,
    GreatSword,
    LongSword,
    SwordAndShield,
    DualBlades,
    Lance,
    Gunlance,
    Hammer,
    HuntingHorn,
    SwitchAxe,
    ChargeBlade,
    InsectGlaive,
    LightBowgun,
    HeavyBowgun,
    Bow,
}

impl WeaponType {
    pub fn as_str(self) -> &'static str {
        match self {
            WeaponType::None => "指定なし",
            WeaponType::GreatSword => "大剣",
            WeaponType::LongSword => "太刀",
            WeaponType::SwordAndShield => "片手剣",
            WeaponType::DualBlades => "双剣",
            WeaponType::Lance => "ランス",
            WeaponType::Gunlance => "ガンランス",
            WeaponType::Hammer => "ハンマー",
            WeaponType::HuntingHorn => "狩猟笛",
            WeaponType::SwitchAxe => "スラッシュアックス",
            WeaponType::ChargeBlade => "チャージアックス",
            WeaponType::InsectGlaive => "操虫棍",
            WeaponType::LightBowgun => "ライトボウガン",
            WeaponType::HeavyBowgun => "ヘビィボウガン",
            WeaponType::Bow => "弓",
        }
    }

    pub fn from_str(s: &str) -> Option<WeaponType> {
        Some(match s {
            "指定なし" => WeaponType::None,
            "大剣" => WeaponType::GreatSword,
            "太刀" => WeaponType::LongSword,
            "片手剣" => WeaponType::SwordAndShield,
            "双剣" => WeaponType::DualBlades,
            "ランス" => WeaponType::Lance,
            "ガンランス" => WeaponType::Gunlance,
            "ハンマー" => WeaponType::Hammer,
            "狩猟笛" => WeaponType::HuntingHorn,
            "スラッシュアックス" => WeaponType::SwitchAxe,
            "チャージアックス" => WeaponType::ChargeBlade,
            "操虫棍" => WeaponType::InsectGlaive,
            "ライトボウガン" => WeaponType::LightBowgun,
            "ヘビィボウガン" => WeaponType::HeavyBowgun,
            "弓" => WeaponType::Bow,
            _ => return None,
        })
    }

    pub fn names_without_none() -> Vec<&'static str> {
        WEAPON_TYPES.iter().map(|w| w.as_str()).collect()
    }
}

impl fmt::Display for WeaponType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub const DISPLAY_RESTRICT_CATEGORIES: [&str; 2] = ["グループスキル", "シリーズスキル"];

#[derive(Clone, Debug, Default)]
pub struct Skill {
    pub name: String,
    pub level: i32,
    pub is_fixed: bool,
    pub category: String,
    pub can_with_artian: bool,
    pub max_level: i32,
    pub specific_names: BTreeMap<i32, String>,
}

#[derive(Clone, Debug, Default)]
pub struct SkillLookup {
    entries: HashMap<String, Skill>,
}

impl SkillLookup {
    pub fn new(master: &[Skill]) -> Self {
        let mut entries = HashMap::new();
        for skill in master {
            entries.insert(skill.name.clone(), skill.clone());
        }
        SkillLookup { entries }
    }

    pub fn make(&self, name: &str, level: i32) -> Skill {
        match self.entries.get(name) {
            Some(master) => Skill {
                name: name.to_string(),
                level,
                is_fixed: false,
                category: master.category.clone(),
                can_with_artian: master.can_with_artian,
                max_level: master.level,
                specific_names: master.specific_names.clone(),
            },
            None => Skill {
                name: name.to_string(),
                level,
                is_fixed: false,
                category: "未分類".to_string(),
                can_with_artian: false,
                max_level: 0,
                specific_names: BTreeMap::new(),
            },
        }
    }
}

impl Skill {
    pub fn new_master(name: String, level: i32, category: String, can_with_artian: bool) -> Skill {
        Skill {
            name,
            level,
            is_fixed: false,
            category,
            can_with_artian,
            max_level: level,
            specific_names: BTreeMap::new(),
        }
    }

    pub fn description(&self) -> String {
        if self.name.trim().is_empty() || self.level == 0 {
            return String::new();
        }
        match self.specific_names.get(&self.level) {
            Some(specific) => format!("{}({}Lv{})", specific, self.name, self.level),
            None => format!("{}Lv{}", self.name, self.level),
        }
    }

    pub fn is_hide_level(&self, level: Option<i32>) -> bool {
        let level = level.unwrap_or(self.level);
        DISPLAY_RESTRICT_CATEGORIES.contains(&self.category.as_str())
            && !self.specific_names.contains_key(&level)
    }
}

#[derive(Clone, Debug)]
pub struct UpperInfo {
    pub name: String,
    pub disp_name: String,
    pub is_full_upper: bool,
}

#[derive(Clone, Debug)]
pub struct Equipment {
    pub name: String,
    pub rare: i32,
    pub slot1: i32,
    pub slot2: i32,
    pub slot3: i32,
    pub slot_type1: i32,
    pub slot_type2: i32,
    pub slot_type3: i32,
    pub mindef: i32,
    pub maxdef: i32,
    pub transcending_def: Option<i32>,
    pub fire: i32,
    pub water: i32,
    pub thunder: i32,
    pub ice: i32,
    pub dragon: i32,
    pub is_one_set: bool,
    pub row_no: i32,
    pub skills: Vec<Skill>,
    pub kind: EquipKind,
    pub is_virtual: bool,
    pub disp_name: Option<String>,
    pub attack: i32,
    pub weapon_type: WeaponType,
    pub deco_count: i32,
    pub deco_category: Option<String>,
    pub upper: Option<UpperInfo>,
}

impl Default for Equipment {
    fn default() -> Self {
        Equipment {
            name: String::new(),
            rare: 0,
            slot1: 0,
            slot2: 0,
            slot3: 0,
            slot_type1: 0,
            slot_type2: 0,
            slot_type3: 0,
            mindef: 0,
            maxdef: 0,
            transcending_def: None,
            fire: 0,
            water: 0,
            thunder: 0,
            ice: 0,
            dragon: 0,
            is_one_set: false,
            row_no: i32::MAX,
            skills: Vec::new(),
            kind: EquipKind::Head,
            is_virtual: false,
            disp_name: None,
            attack: 0,
            weapon_type: WeaponType::None,
            deco_count: 0,
            deco_category: None,
            upper: None,
        }
    }
}

impl Equipment {
    pub fn new(kind: EquipKind) -> Equipment {
        Equipment {
            kind,
            ..Default::default()
        }
    }

    pub fn new_weapon() -> Equipment {
        Equipment::new(EquipKind::Weapon)
    }

    pub fn new_deco() -> Equipment {
        Equipment::new(EquipKind::Deco)
    }

    pub fn init_artian(&mut self) {
        self.rare = 8;
        self.slot1 = 3;
        self.slot2 = 3;
        self.slot3 = 3;
        self.slot_type1 = 1;
        self.slot_type2 = 1;
        self.slot_type3 = 1;
        self.mindef = 0;
        self.maxdef = 0;
        self.attack = 190;
        self.row_no = i32::MAX;
    }

    pub fn disp_name(&self) -> &str {
        match &self.disp_name {
            Some(name) if !name.trim().is_empty() => name,
            _ => &self.name,
        }
    }

    pub fn detail_disp_name(&self) -> &str {
        self.disp_name()
    }

    pub fn transcending_def(&self) -> i32 {
        self.transcending_def.unwrap_or(self.maxdef)
    }

    fn is_transcending_slot_target(&self) -> bool {
        self.kind.is_armor() && (self.rare == 5 || self.rare == 6)
    }

    pub fn transcending_slot1(&self) -> i32 {
        if self.is_transcending_slot_target() {
            (self.slot1 + 1).min(3)
        } else {
            self.slot1
        }
    }

    pub fn transcending_slot2(&self) -> i32 {
        if self.is_transcending_slot_target() {
            (self.slot2 + 1).min(3)
        } else {
            self.slot2
        }
    }

    pub fn transcending_slot3(&self) -> i32 {
        if self.is_transcending_slot_target() {
            if self.rare == 6 {
                self.slot3
            } else {
                (self.slot3 + 1).min(3)
            }
        } else {
            self.slot3
        }
    }

    pub fn description(&self) -> String {
        if self.name.trim().is_empty() {
            return String::new();
        }
        let mut sb = String::new();
        sb.push_str(self.disp_name());
        if self.kind != EquipKind::Deco {
            sb.push(',');
            sb.push_str(&self.slot1.to_string());
            sb.push('-');
            sb.push_str(&self.slot2.to_string());
            sb.push('-');
            sb.push_str(&self.slot3.to_string());
            if self.is_transcending_slot_target() {
                sb.push('→');
                sb.push_str(&self.transcending_slot1().to_string());
                sb.push('-');
                sb.push_str(&self.transcending_slot2().to_string());
                sb.push('-');
                sb.push_str(&self.transcending_slot3().to_string());
            }
            sb.push('\n');
            sb.push_str("防御:");
            sb.push_str(&self.mindef.to_string());
            sb.push('→');
            sb.push_str(&self.maxdef.to_string());
            sb.push('→');
            sb.push_str(&self.transcending_def().to_string());
            sb.push(',');
            sb.push_str("火:");
            sb.push_str(&self.fire.to_string());
            sb.push(',');
            sb.push_str("水:");
            sb.push_str(&self.water.to_string());
            sb.push(',');
            sb.push_str("雷:");
            sb.push_str(&self.thunder.to_string());
            sb.push(',');
            sb.push_str("氷:");
            sb.push_str(&self.ice.to_string());
            sb.push(',');
            sb.push_str("龍:");
            sb.push_str(&self.dragon.to_string());
        }
        for skill in &self.skills {
            sb.push('\n');
            sb.push_str(&skill.description());
        }
        sb
    }

    pub fn simple_description(&self) -> String {
        let mut sb = String::new();
        sb.push_str(&self.kind.str_with_colon());
        if !self.name.trim().is_empty() {
            sb.push_str(self.disp_name());
            if self.kind != EquipKind::Deco {
                sb.push(',');
                sb.push_str(&self.slot1.to_string());
                sb.push('-');
                sb.push_str(&self.slot2.to_string());
                sb.push('-');
                sb.push_str(&self.slot3.to_string());
                if self.is_transcending_slot_target() {
                    sb.push('→');
                    sb.push_str(&self.transcending_slot1().to_string());
                    sb.push('-');
                    sb.push_str(&self.transcending_slot2().to_string());
                    sb.push('-');
                    sb.push_str(&self.transcending_slot3().to_string());
                }
            }
        }
        sb
    }

    pub fn set_charm_disp_name(&mut self) {
        if self.kind != EquipKind::Charm {
            return;
        }
        let mut disp = String::new();
        if self.is_virtual {
            disp.push_str("(仮) ");
        }
        for skill in &self.skills {
            disp.push_str(&skill.name);
            if skill.level > 0 {
                disp.push_str(&format!("Lv{}", skill.level));
            }
            disp.push_str(", ");
        }
        disp.push_str(&self.slot1.to_string());
        disp.push_str(&slot_type_suffix(self.slot1, self.slot_type1));
        disp.push('-');
        disp.push_str(&self.slot2.to_string());
        disp.push_str(&slot_type_suffix(self.slot2, self.slot_type2));
        disp.push('-');
        disp.push_str(&self.slot3.to_string());
        disp.push_str(&slot_type_suffix(self.slot3, self.slot_type3));
        self.disp_name = Some(disp);
    }

    pub fn deco_category(&self) -> String {
        if let Some(category) = &self.deco_category {
            return category.clone();
        }
        if let Some(skill) = self.skills.first() {
            return skill.category.clone();
        }
        "未分類".to_string()
    }
}

fn slot_type_suffix(size: i32, slot_type: i32) -> &'static str {
    if size == 0 {
        return "";
    }
    match slot_type {
        0 => "(防)",
        1 => "(武)",
        2 => "(両)",
        _ => "",
    }
}

#[derive(Clone, Debug, Default)]
pub struct CharmCombo {
    pub rare: i32,
    pub group1: i32,
    pub group2: i32,
    pub group3: i32,
    pub slot1: i32,
    pub slot2: i32,
    pub slot3: i32,
    pub slot_type1: i32,
    pub slot_type2: i32,
    pub slot_type3: i32,
}

#[derive(Clone, Debug, Default)]
pub struct Clude {
    pub name: String,
    pub kind: CludeKind,
}

impl Default for CludeKind {
    fn default() -> Self {
        CludeKind::Exclude
    }
}

#[derive(Clone, Debug, Default)]
pub struct DefUpgrade {
    pub upgrade_def: i32,
    pub transcending_def: i32,
}

#[derive(Clone, Debug, Default)]
pub struct EquipSet {
    pub weapon: Equipment,
    pub head: Equipment,
    pub body: Equipment,
    pub arm: Equipment,
    pub waist: Equipment,
    pub leg: Equipment,
    pub charm: Equipment,
    pub decos: Vec<Equipment>,
    pub name: String,
    pub is_transcending: bool,
}

impl EquipSet {
    pub fn new() -> EquipSet {
        EquipSet {
            weapon: Equipment::new_weapon(),
            head: Equipment::new(EquipKind::Head),
            body: Equipment::new(EquipKind::Body),
            arm: Equipment::new(EquipKind::Arm),
            waist: Equipment::new(EquipKind::Waist),
            leg: Equipment::new(EquipKind::Leg),
            charm: Equipment::new(EquipKind::Charm),
            decos: Vec::new(),
            name: String::new(),
            is_transcending: false,
        }
    }

    pub fn equipments(&self) -> Vec<&Equipment> {
        let mut ret = vec![
            &self.weapon,
            &self.head,
            &self.body,
            &self.arm,
            &self.waist,
            &self.leg,
            &self.charm,
        ];
        for deco in &self.decos {
            ret.push(deco);
        }
        ret
    }

    pub fn mindef(&self) -> i32 {
        self.equipments().iter().map(|e| e.mindef).sum()
    }

    pub fn maxdef(&self) -> i32 {
        self.equipments()
            .iter()
            .map(|e| {
                if self.is_transcending {
                    e.transcending_def()
                } else {
                    e.maxdef
                }
            })
            .sum()
    }

    pub fn fire(&self) -> i32 {
        self.equipments().iter().map(|e| e.fire).sum()
    }

    pub fn water(&self) -> i32 {
        self.equipments().iter().map(|e| e.water).sum()
    }

    pub fn thunder(&self) -> i32 {
        self.equipments().iter().map(|e| e.thunder).sum()
    }

    pub fn ice(&self) -> i32 {
        self.equipments().iter().map(|e| e.ice).sum()
    }

    pub fn dragon(&self) -> i32 {
        self.equipments().iter().map(|e| e.dragon).sum()
    }

    pub fn skills(&self) -> Vec<Skill> {
        let mut ret: Vec<Skill> = Vec::new();
        for equip in self.equipments() {
            join_skill(&mut ret, &equip.skills);
        }
        for skill in &mut ret {
            if skill.level > skill.max_level {
                skill.level = skill.max_level;
            }
        }
        ret.sort_by(|a, b| b.level.cmp(&a.level));
        ret
    }

    pub fn glpk_row_name(&self) -> String {
        format!(
            "{},{},{},{},{},{},{}",
            self.weapon.name,
            self.head.name,
            self.body.name,
            self.arm.name,
            self.waist.name,
            self.leg.name,
            self.charm.name
        )
    }

    pub fn existing_equips_without_decos(&self) -> Vec<&Equipment> {
        let mut list = Vec::new();
        for equip in [
            &self.weapon,
            &self.head,
            &self.body,
            &self.arm,
            &self.waist,
            &self.leg,
            &self.charm,
        ] {
            if !equip.name.trim().is_empty() {
                list.push(equip);
            }
        }
        list
    }

    pub fn deco_name_csv(&self) -> String {
        self.decos
            .iter()
            .map(|d| d.name.as_str())
            .collect::<Vec<_>>()
            .join(",")
    }

    pub fn deco_name_csv_multi_line(&self) -> String {
        let mut sb = String::new();
        let second_line_idx = self.decos.len() / 3;
        let third_line_idx = self.decos.len() * 2 / 3;
        for (i, deco) in self.decos.iter().enumerate() {
            if i == 0 {
            } else if i == second_line_idx {
                sb.push(',');
                sb.push('\n');
            } else if i == third_line_idx {
                sb.push(',');
                sb.push('\n');
            } else {
                sb.push(',');
            }
            sb.push_str(&deco.name);
        }
        sb
    }

    pub fn skills_disp(&self) -> String {
        self.skills()
            .iter()
            .filter(|s| s.level > 0)
            .map(|s| s.description())
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn skills_disp_multi_line(&self) -> String {
        let exist_skill: Vec<Skill> = self.skills().into_iter().filter(|s| s.level > 0).collect();
        let mut sb = String::new();
        let second_line_idx = exist_skill.len() / 3;
        let third_line_idx = exist_skill.len() * 2 / 3;
        for (i, skill) in exist_skill.iter().enumerate() {
            if i == 0 {
            } else if i == second_line_idx {
                sb.push(',');
                sb.push('\n');
            } else if i == third_line_idx {
                sb.push(',');
                sb.push('\n');
            } else {
                sb.push(',');
            }
            sb.push_str(&skill.description());
        }
        sb
    }

    pub fn description(&self) -> String {
        let mut sb = String::new();
        sb.push_str("防御:");
        sb.push_str(&self.mindef().to_string());
        sb.push('→');
        sb.push_str(&self.maxdef().to_string());
        if self.is_transcending {
            sb.push_str("(限界突破)");
        }
        sb.push(',');
        sb.push_str("火:");
        sb.push_str(&self.fire().to_string());
        sb.push(',');
        sb.push_str("水:");
        sb.push_str(&self.water().to_string());
        sb.push(',');
        sb.push_str("雷:");
        sb.push_str(&self.thunder().to_string());
        sb.push(',');
        sb.push_str("氷:");
        sb.push_str(&self.ice().to_string());
        sb.push(',');
        sb.push_str("龍:");
        sb.push_str(&self.dragon().to_string());
        sb.push('\n');
        sb.push_str(&self.weapon.simple_description());
        sb.push('\n');
        sb.push_str(&self.head.simple_description());
        sb.push('\n');
        sb.push_str(&self.body.simple_description());
        sb.push('\n');
        sb.push_str(&self.arm.simple_description());
        sb.push('\n');
        sb.push_str(&self.waist.simple_description());
        sb.push('\n');
        sb.push_str(&self.leg.simple_description());
        sb.push('\n');
        sb.push_str(&self.charm.simple_description());
        sb.push('\n');
        sb.push_str(&EquipKind::Deco.str_with_colon());
        sb.push_str(&self.deco_name_csv());
        sb.push('\n');
        sb.push_str("-----------");
        for skill in self.skills() {
            if skill.level > 0 {
                sb.push('\n');
                sb.push_str(&skill.description());
            }
        }
        sb
    }

    pub fn sort_decos(&mut self) {
        let mut new_decos = Vec::with_capacity(self.decos.len());
        for i in (1..=4).rev() {
            for deco in &self.decos {
                if deco.slot1 == i {
                    new_decos.push(deco.clone());
                }
            }
        }
        self.decos = new_decos;
    }
}

fn join_skill(base_skills: &mut Vec<Skill>, new_skills: &[Skill]) {
    for new_skill in new_skills {
        if new_skill.name.trim().is_empty() {
            continue;
        }
        let mut exist = false;
        for base_skill in base_skills.iter_mut() {
            if base_skill.name == new_skill.name {
                exist = true;
                base_skill.level += new_skill.level;
            }
        }
        if !exist {
            base_skills.push(new_skill.clone());
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct SearchCondition {
    pub skills: Vec<Skill>,
    pub is_specific_weapon: bool,
    pub weapon_name: Option<String>,
    pub weapon_disp_name: Option<String>,
    pub weapon_type: WeaponType,
    pub min_attack: Option<i32>,
    pub def: Option<i32>,
    pub fire: Option<i32>,
    pub water: Option<i32>,
    pub thunder: Option<i32>,
    pub ice: Option<i32>,
    pub dragon: Option<i32>,
    pub is_transcending: bool,
    pub id: Option<String>,
    pub disp_name: Option<String>,
    pub is_best_charm_search: bool,
    pub is_best_artian_search: bool,
    pub has_all_decos: bool,
}

impl SearchCondition {
    pub fn new() -> SearchCondition {
        SearchCondition {
            is_transcending: true,
            ..Default::default()
        }
    }

    pub fn weapon_disp_name(&self) -> Option<&str> {
        self.weapon_disp_name
            .as_deref()
            .or(self.weapon_name.as_deref())
    }

    pub fn skill_csv(&self) -> String {
        let mut parts = Vec::new();
        for skill in &self.skills {
            let mut level = skill.level.to_string();
            if skill.is_fixed {
                level.push_str("固定");
            }
            parts.push(skill.name.clone());
            parts.push(level);
        }
        parts.join(",")
    }

    pub fn set_skill_csv(&mut self, value: &str, lookup: &SkillLookup) {
        self.skills.clear();
        let splitted: Vec<&str> = value.split(',').collect();
        let mut i = 0usize;
        while i < splitted.len() {
            if splitted[i].trim().is_empty() {
                i += 2;
                continue;
            }
            let name = splitted[i].to_string();
            i += 1;
            if i >= splitted.len() {
                break;
            }
            let level_str = splitted[i];
            i += 1;
            let is_fixed = level_str.ends_with("固定");
            let level_str = level_str.replace("固定", "");
            let mut skill = lookup.make(&name, crate::parse_util::parse_i32(&level_str, 0));
            skill.is_fixed = is_fixed;
            self.skills.push(skill);
        }
    }

    pub fn description(&self) -> String {
        let none = "なし";
        let mut sb = String::new();
        if self.is_transcending {
            sb.push_str("限界突破強化:あり\n");
        } else {
            sb.push_str("限界突破強化:なし\n");
        }
        if self.is_specific_weapon {
            sb.push_str(&format!("武器:{}\n", self.weapon_disp_name().unwrap_or("")));
        } else {
            sb.push_str(&format!("武器種:{}, ", self.weapon_type));
            sb.push_str(&format!(
                "最低攻撃力:{}\n",
                self.min_attack
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| none.to_string())
            ));
        }
        sb.push_str(&format!(
            "防御力:{}\n",
            self.def
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        sb.push_str(&format!(
            "火:{},",
            self.fire
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        sb.push_str(&format!(
            "水:{},",
            self.water
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        sb.push_str(&format!(
            "雷:{},",
            self.thunder
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        sb.push_str(&format!(
            "氷:{},",
            self.ice
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        sb.push_str(&format!(
            "龍:{}",
            self.dragon
                .map(|v| v.to_string())
                .unwrap_or_else(|| none.to_string())
        ));
        for skill in &self.skills {
            sb.push('\n');
            sb.push_str(&skill.description());
            if skill.is_fixed {
                sb.push_str("(固定)");
            }
        }
        sb
    }

    pub fn copy_for_search(&self) -> SearchCondition {
        let mut condition = SearchCondition::new();
        condition.skills = self.skills.clone();
        condition.is_specific_weapon = self.is_specific_weapon;
        condition.weapon_name = self.weapon_name.clone();
        if self.weapon_name.is_some() && self.weapon_name.as_deref() != self.weapon_disp_name() {
            condition.weapon_disp_name = self.weapon_disp_name.clone();
        }
        condition.weapon_type = self.weapon_type;
        condition.min_attack = self.min_attack;
        condition.def = self.def;
        condition.fire = self.fire;
        condition.water = self.water;
        condition.thunder = self.thunder;
        condition.ice = self.ice;
        condition.dragon = self.dragon;
        condition.is_best_charm_search = self.is_best_charm_search;
        condition.is_best_artian_search = self.is_best_artian_search;
        condition.is_transcending = self.is_transcending;
        condition
    }

    pub fn add_skill(&mut self, additional: &Skill) -> bool {
        for skill in self.skills.iter_mut() {
            if skill.name == additional.name {
                if !skill.is_fixed && additional.is_fixed {
                    skill.level = additional.level;
                    skill.is_fixed = true;
                    return true;
                }
                if skill.is_fixed && !additional.is_fixed {
                    return false;
                }
                if skill.level < additional.level {
                    skill.level = additional.level;
                    return true;
                }
                return false;
            }
        }
        self.skills.push(additional.clone());
        true
    }

    pub fn make_related_charms(
        &self,
        charm_combos: &[CharmCombo],
        charm_groups: &HashMap<i32, Vec<Skill>>,
    ) -> Vec<Equipment> {
        let mut target_charms: Vec<Equipment> = Vec::new();
        let cond_skills: Vec<&Skill> = self.skills.iter().filter(|s| s.level != 0).collect();
        let empty: Vec<Skill> = Vec::new();
        for combo in charm_combos {
            let group1 = charm_groups.get(&combo.group1).unwrap_or(&empty);
            let group2 = charm_groups.get(&combo.group2).unwrap_or(&empty);
            let group3 = charm_groups.get(&combo.group3).unwrap_or(&empty);
            let skill1_list: Vec<Option<&Skill>> = group1
                .iter()
                .filter(|skill| cond_skills.iter().any(|s| s.name == skill.name))
                .map(Some)
                .chain(std::iter::once(None))
                .collect();
            let skill2_list: Vec<Option<&Skill>> = group2
                .iter()
                .filter(|skill| cond_skills.iter().any(|s| s.name == skill.name))
                .map(Some)
                .chain(std::iter::once(None))
                .collect();
            let skill3_list: Vec<Option<&Skill>> = group3
                .iter()
                .filter(|skill| cond_skills.iter().any(|s| s.name == skill.name))
                .map(Some)
                .chain(std::iter::once(None))
                .collect();
            for skill1 in &skill1_list {
                for skill2 in &skill2_list {
                    for skill3 in &skill3_list {
                        let mut charm = Equipment::new(EquipKind::Charm);
                        charm.name = Uuid::new_v4().to_string();
                        charm.rare = combo.rare;
                        charm.slot1 = combo.slot1;
                        charm.slot2 = combo.slot2;
                        charm.slot3 = combo.slot3;
                        charm.slot_type1 = combo.slot_type1;
                        charm.slot_type2 = combo.slot_type2;
                        charm.slot_type3 = combo.slot_type3;
                        charm.is_virtual = true;
                        if let Some(skill) = skill1 {
                            charm.skills.push((*skill).clone());
                        }
                        if let Some(skill) = skill2 {
                            if !charm.skills.iter().any(|s| s.name == skill.name) {
                                charm.skills.push((*skill).clone());
                            }
                        }
                        if let Some(skill) = skill3 {
                            if !charm.skills.iter().any(|s| s.name == skill.name) {
                                charm.skills.push((*skill).clone());
                            }
                        }
                        charm.set_charm_disp_name();
                        let mut is_exist = false;
                        for exist in &target_charms {
                            if is_same_charm(&charm, exist) {
                                is_exist = true;
                                break;
                            }
                        }
                        if !is_exist {
                            target_charms.push(charm);
                        }
                    }
                }
            }
        }
        target_charms
    }

    pub fn make_related_artians(&self) -> Vec<Equipment> {
        let mut target: Vec<Equipment> = Vec::new();
        let group_skills: Vec<&Skill> = self
            .skills
            .iter()
            .filter(|s| s.level != 0 && s.category == "グループスキル" && s.can_with_artian)
            .collect();
        let series_skills: Vec<&Skill> = self
            .skills
            .iter()
            .filter(|s| s.level != 0 && s.category == "シリーズスキル" && s.can_with_artian)
            .collect();
        for group_skill in &group_skills {
            for series_skill in &series_skills {
                let mut artian = Equipment::new_weapon();
                artian.init_artian();
                artian.name = Uuid::new_v4().to_string();
                artian.weapon_type = self.weapon_type;
                artian.disp_name = Some(format!(
                    "{}({},{})",
                    artian.weapon_type, group_skill.name, series_skill.name
                ));
                artian.skills.push((*group_skill).clone());
                artian.skills[0].level = 1;
                artian.skills.push((*series_skill).clone());
                artian.skills[1].level = 1;
                artian.is_virtual = true;
                target.push(artian);
            }
        }
        for group_skill in &group_skills {
            let mut artian = Equipment::new_weapon();
            artian.init_artian();
            artian.name = Uuid::new_v4().to_string();
            artian.weapon_type = self.weapon_type;
            artian.disp_name = Some(format!("{}({})", artian.weapon_type, group_skill.name));
            artian.skills.push((*group_skill).clone());
            artian.skills[0].level = 1;
            artian.is_virtual = true;
            target.push(artian);
        }
        for series_skill in &series_skills {
            let mut artian = Equipment::new_weapon();
            artian.init_artian();
            artian.name = Uuid::new_v4().to_string();
            artian.weapon_type = self.weapon_type;
            artian.disp_name = Some(format!("{}({})", artian.weapon_type, series_skill.name));
            artian.skills.push((*series_skill).clone());
            artian.skills[0].level = 1;
            artian.is_virtual = true;
            target.push(artian);
        }
        target
    }
}

fn is_same_charm(left: &Equipment, right: &Equipment) -> bool {
    if left.skills.len() != right.skills.len() {
        return false;
    }
    if left.slot1 != right.slot1
        || left.slot2 != right.slot2
        || left.slot3 != right.slot3
        || left.slot_type1 != right.slot_type1
        || left.slot_type2 != right.slot_type2
        || left.slot_type3 != right.slot_type3
    {
        return false;
    }
    for skill in &left.skills {
        if !right
            .skills
            .iter()
            .any(|s| s.name == skill.name && s.level == skill.level)
        {
            return false;
        }
    }
    true
}

#[derive(Clone, Debug, Default)]
pub struct SearchRange {
    pub weapons: Vec<Equipment>,
    pub heads: Vec<Equipment>,
    pub bodys: Vec<Equipment>,
    pub arms: Vec<Equipment>,
    pub waists: Vec<Equipment>,
    pub legs: Vec<Equipment>,
    pub charms: Vec<Equipment>,
    pub decos: Vec<Equipment>,
    pub cludes: Vec<Clude>,
}

impl SearchRange {
    pub fn new_empty() -> SearchRange {
        SearchRange::default()
    }
}

#[derive(Clone, Debug, Default)]
pub struct Masters {
    pub skills: Vec<Skill>,
    pub weapons: Vec<Equipment>,
    pub artians: Vec<Equipment>,
    pub heads: Vec<Equipment>,
    pub bodys: Vec<Equipment>,
    pub arms: Vec<Equipment>,
    pub waists: Vec<Equipment>,
    pub legs: Vec<Equipment>,
    pub charms: Vec<Equipment>,
    pub additional_charms: Vec<Equipment>,
    pub shining_charm_combos: Vec<CharmCombo>,
    pub shining_charm_groups: HashMap<i32, Vec<Skill>>,
    pub decos: Vec<Equipment>,
    pub cludes: Vec<Clude>,
    pub my_sets: Vec<EquipSet>,
    pub recent_skill_names: Vec<String>,
    pub my_conditions: Vec<SearchCondition>,
    pub def_upgrades: HashMap<i32, DefUpgrade>,
    skill_lookup: SkillLookup,
}

impl Masters {
    pub fn set_skills(&mut self, skills: Vec<Skill>) {
        self.skill_lookup = SkillLookup::new(&skills);
        self.skills = skills;
    }

    pub fn skill_lookup(&self) -> &SkillLookup {
        &self.skill_lookup
    }

    pub fn make_skill(&self, name: &str, level: i32) -> Skill {
        self.skill_lookup.make(name, level)
    }

    pub fn all_equipments(&self) -> Vec<&Equipment> {
        let mut all = Vec::new();
        all.extend(self.weapons.iter());
        all.extend(self.artians.iter());
        all.extend(self.heads.iter());
        all.extend(self.bodys.iter());
        all.extend(self.arms.iter());
        all.extend(self.waists.iter());
        all.extend(self.legs.iter());
        all.extend(self.charms.iter());
        all.extend(self.additional_charms.iter());
        all.extend(self.decos.iter());
        all
    }

    pub fn get_equip_by_name(&self, equip_name: &str) -> Option<&Equipment> {
        let name = equip_name.trim();
        self.all_equipments().into_iter().find(|e| e.name == name)
    }

    pub fn is_skill_name(&self, value: &str) -> bool {
        let name = value.trim();
        if name.is_empty() {
            return false;
        }
        self.skills.iter().any(|skill| skill.name == name)
    }

    pub fn skill_max_level(&self, name: &str) -> i32 {
        for skill in &self.skills {
            if skill.name == name {
                return skill.level;
            }
        }
        0
    }
}

pub fn build_search_range(condition: &SearchCondition, masters: &Masters) -> SearchRange {
    let mut range = SearchRange::new_empty();
    if condition.is_specific_weapon {
        if let Some(weapon_name) = &condition.weapon_name {
            if let Some(weapon) = masters
                .weapons
                .iter()
                .chain(masters.artians.iter())
                .find(|w| w.name == *weapon_name)
            {
                range.weapons.push(weapon.clone());
            }
        }
    } else if condition.is_best_artian_search {
        for w in masters.weapons.iter().chain(masters.artians.iter()) {
            if w.weapon_type == condition.weapon_type {
                range.weapons.push(w.clone());
            }
        }
        for w in condition.make_related_artians() {
            if w.weapon_type == condition.weapon_type {
                range.weapons.push(w);
            }
        }
    } else {
        for w in masters.weapons.iter().chain(masters.artians.iter()) {
            if w.weapon_type == condition.weapon_type {
                range.weapons.push(w.clone());
            }
        }
    }

    if condition.is_best_charm_search {
        range.charms.extend(masters.charms.iter().cloned());
        range
            .charms
            .extend(masters.additional_charms.iter().cloned());
        range.charms.extend(
            condition
                .make_related_charms(&masters.shining_charm_combos, &masters.shining_charm_groups),
        );
    } else {
        range.charms.extend(masters.charms.iter().cloned());
        range
            .charms
            .extend(masters.additional_charms.iter().cloned());
    }

    range.heads = masters.heads.clone();
    range.bodys = masters.bodys.clone();
    range.arms = masters.arms.clone();
    range.waists = masters.waists.clone();
    range.legs = masters.legs.clone();
    range.decos = masters.decos.clone();
    range.cludes = masters.cludes.clone();
    range
}
