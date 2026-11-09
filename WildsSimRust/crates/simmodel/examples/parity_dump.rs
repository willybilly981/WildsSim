use std::path::Path;
use std::sync::atomic::AtomicBool;

use serde_json::Value;
use simmodel::*;

fn arg_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .cloned()
}

fn str_field(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(|x| x.as_str()).map(|s| s.to_string())
}

fn int_field(v: &Value, key: &str) -> Option<i32> {
    v.get(key).and_then(|x| x.as_i64()).map(|x| x as i32)
}

fn bool_field(v: &Value, key: &str, default: bool) -> bool {
    v.get(key).and_then(|x| x.as_bool()).unwrap_or(default)
}

fn kind_code(kind: EquipKind) -> i32 {
    match kind {
        EquipKind::Head => 0,
        EquipKind::Body => 1,
        EquipKind::Arm => 2,
        EquipKind::Waist => 3,
        EquipKind::Leg => 4,
        EquipKind::Deco => 5,
        EquipKind::Charm => 6,
        EquipKind::Weapon => 7,
        EquipKind::Error => 8,
    }
}

fn weapon_type_code(weapon_type: WeaponType) -> i32 {
    match weapon_type {
        WeaponType::None => 0,
        WeaponType::GreatSword => 1,
        WeaponType::LongSword => 2,
        WeaponType::SwordAndShield => 3,
        WeaponType::DualBlades => 4,
        WeaponType::Lance => 5,
        WeaponType::Gunlance => 6,
        WeaponType::Hammer => 7,
        WeaponType::HuntingHorn => 8,
        WeaponType::SwitchAxe => 9,
        WeaponType::ChargeBlade => 10,
        WeaponType::InsectGlaive => 11,
        WeaponType::LightBowgun => 12,
        WeaponType::HeavyBowgun => 13,
        WeaponType::Bow => 14,
    }
}

fn equip_line(equip: &Equipment) -> String {
    let skills = equip
        .skills
        .iter()
        .map(|s| format!("{}:{}:{}", s.name, s.level, s.max_level))
        .collect::<Vec<_>>()
        .join(";");
    let (attack, weapon_type) = if equip.kind == EquipKind::Weapon {
        (equip.attack, weapon_type_code(equip.weapon_type))
    } else {
        (0, 0)
    };
    let (deco_count, deco_category) = if equip.kind == EquipKind::Deco {
        (equip.deco_count, equip.deco_category())
    } else {
        (0, String::new())
    };
    vec![
        kind_code(equip.kind).to_string(),
        equip.name.clone(),
        equip.rare.to_string(),
        equip.slot1.to_string(),
        equip.slot2.to_string(),
        equip.slot3.to_string(),
        equip.slot_type1.to_string(),
        equip.slot_type2.to_string(),
        equip.slot_type3.to_string(),
        equip.transcending_slot1().to_string(),
        equip.transcending_slot2().to_string(),
        equip.transcending_slot3().to_string(),
        equip.mindef.to_string(),
        equip.maxdef.to_string(),
        equip.transcending_def().to_string(),
        equip.fire.to_string(),
        equip.water.to_string(),
        equip.thunder.to_string(),
        equip.ice.to_string(),
        equip.dragon.to_string(),
        if equip.is_one_set { "1" } else { "0" }.to_string(),
        equip.row_no.to_string(),
        if equip.is_virtual { "1" } else { "0" }.to_string(),
        attack.to_string(),
        weapon_type.to_string(),
        deco_count.to_string(),
        deco_category,
        equip.disp_name().to_string(),
        skills,
    ]
    .join("|")
}

fn equip_ref(equip: &Equipment) -> String {
    if equip.is_virtual {
        equip.disp_name().to_string()
    } else {
        equip.name.clone()
    }
}

fn set_signature(set: &EquipSet) -> String {
    let mut skills = set
        .skills()
        .into_iter()
        .filter(|s| s.level > 0)
        .map(|s| format!("{}:{}", s.name, s.level))
        .collect::<Vec<_>>();
    skills.sort();
    let mut decos = set.decos.iter().map(|d| d.name.clone()).collect::<Vec<_>>();
    decos.sort();
    format!(
        "W={}|H={}|B={}|A={}|Wa={}|L={}|C={}|maxdef={}|mindef={}|fire={}|water={}|thunder={}|ice={}|dragon={}|transc={}|decos={}|skills={}",
        equip_ref(&set.weapon),
        equip_ref(&set.head),
        equip_ref(&set.body),
        equip_ref(&set.arm),
        equip_ref(&set.waist),
        equip_ref(&set.leg),
        equip_ref(&set.charm),
        set.maxdef(),
        set.mindef(),
        set.fire(),
        set.water(),
        set.thunder(),
        set.ice(),
        set.dragon(),
        if set.is_transcending { 1 } else { 0 },
        decos.join(","),
        skills.join(",")
    )
}

fn dump_masters(sim: &Simulator) -> String {
    let mut out = String::new();
    for skill in &sim.masters.skills {
        let mut specific = skill
            .specific_names
            .iter()
            .map(|(key, name)| format!("{}={}", key, name))
            .collect::<Vec<_>>();
        specific.sort();
        out.push_str(&format!(
            "SKILL|{}|{}|{}|{}|{}\n",
            skill.name,
            skill.level,
            skill.category,
            if skill.can_with_artian { 1 } else { 0 },
            specific.join(",")
        ));
    }
    let groups: [(&str, &Vec<Equipment>); 10] = [
        ("weapon", &sim.masters.weapons),
        ("artian", &sim.masters.artians),
        ("head", &sim.masters.heads),
        ("body", &sim.masters.bodys),
        ("arm", &sim.masters.arms),
        ("waist", &sim.masters.waists),
        ("leg", &sim.masters.legs),
        ("charm", &sim.masters.charms),
        ("additional_charm", &sim.masters.additional_charms),
        ("deco", &sim.masters.decos),
    ];
    for (name, list) in groups {
        for (index, equip) in list.iter().enumerate() {
            out.push_str(&format!("EQ|{}|{}|{}\n", name, index, equip_line(equip)));
        }
    }
    for clude in &sim.masters.cludes {
        out.push_str(&format!(
            "CLUDE|{}|{}\n",
            clude.name,
            match clude.kind {
                CludeKind::Exclude => 0,
                CludeKind::Include => 1,
            }
        ));
    }
    for name in &sim.masters.recent_skill_names {
        out.push_str(&format!("RECENT|{}\n", name));
    }
    for condition in &sim.masters.my_conditions {
        out.push_str(&format!(
            "COND|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            condition.id.clone().unwrap_or_default(),
            condition.disp_name.clone().unwrap_or_default(),
            if condition.is_specific_weapon { 1 } else { 0 },
            condition.weapon_name.clone().unwrap_or_default(),
            weapon_type_code(condition.weapon_type),
            option_i32(condition.min_attack),
            option_i32(condition.def),
            option_i32(condition.fire),
            option_i32(condition.water),
            option_i32(condition.thunder),
            option_i32(condition.ice),
            option_i32(condition.dragon),
            condition.skill_csv(),
            if condition.is_transcending { 1 } else { 0 },
            if condition.is_best_charm_search { 1 } else { 0 }
        ));
    }
    for set in &sim.masters.my_sets {
        out.push_str(&format!(
            "MYSET|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}\n",
            set.weapon.name,
            set.head.name,
            set.body.name,
            set.arm.name,
            set.waist.name,
            set.leg.name,
            set.charm.name,
            set.deco_name_csv(),
            set.name,
            if set.is_transcending { 1 } else { 0 }
        ));
    }
    for charm in &sim.masters.additional_charms {
        match &charm.upper {
            Some(upper) => out.push_str(&format!(
                "UPPER|{}|{}|{}|{}\n",
                charm.name,
                upper.name,
                upper.disp_name,
                if upper.is_full_upper { 1 } else { 0 }
            )),
            None => out.push_str(&format!("UPPER|{}|||\n", charm.name)),
        }
    }
    out
}

fn option_i32(value: Option<i32>) -> String {
    value
        .map(|v| v.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn make_condition(sim: &Simulator, scenario: &Value) -> SearchCondition {
    let mut condition = SearchCondition::new();
    condition.is_transcending = bool_field(scenario, "transcending", true);
    condition.has_all_decos = bool_field(scenario, "has_all_decos", false);
    condition.is_best_charm_search = bool_field(scenario, "best_charm", false);
    condition.is_best_artian_search = bool_field(scenario, "best_artian", false);
    if let Some(skills) = scenario.get("skills").and_then(|x| x.as_array()) {
        for entry in skills {
            let name = str_field(entry, "name").unwrap_or_default();
            let level = int_field(entry, "level").unwrap_or(0);
            let mut skill = sim.masters.make_skill(&name, level);
            skill.is_fixed = bool_field(entry, "fixed", false);
            condition.add_skill(&skill);
        }
    }
    if let Some(slot_weapon) = str_field(scenario, "slot_weapon") {
        condition.is_specific_weapon = true;
        condition.weapon_name = Some(slot_weapon);
        condition.weapon_type = WeaponType::None;
    } else if bool_field(scenario, "is_specific_weapon", false) {
        condition.is_specific_weapon = true;
        condition.weapon_name = str_field(scenario, "weapon_name");
        if let Some(weapon_type) = str_field(scenario, "weapon_type") {
            condition.weapon_type = WeaponType::from_str(&weapon_type).unwrap_or(WeaponType::None);
        }
    } else if let Some(weapon_type) = str_field(scenario, "weapon_type") {
        condition.weapon_type = WeaponType::from_str(&weapon_type).unwrap_or(WeaponType::None);
    }
    condition.min_attack = int_field(scenario, "min_attack");
    condition.def = int_field(scenario, "def");
    condition.fire = int_field(scenario, "fire");
    condition.water = int_field(scenario, "water");
    condition.thunder = int_field(scenario, "thunder");
    condition.ice = int_field(scenario, "ice");
    condition.dragon = int_field(scenario, "dragon");
    condition
}

fn first_equip_name(sim: &Simulator, group: &str) -> Option<String> {
    let list = match group {
        "head" => &sim.masters.heads,
        "body" => &sim.masters.bodys,
        "arm" => &sim.masters.arms,
        "waist" => &sim.masters.waists,
        "leg" => &sim.masters.legs,
        "charm" => &sim.masters.charms,
        "additional_charm" => &sim.masters.additional_charms,
        "weapon" => &sim.masters.weapons,
        _ => return None,
    };
    list.first().map(|equip| equip.name.clone())
}

fn apply_first_cludes(sim: &mut Simulator, scenario: &Value) -> bool {
    let mut added = false;
    for (key, include) in [("include_first", true), ("exclude_first", false)] {
        if let Some(groups) = scenario.get(key).and_then(|x| x.as_array()) {
            for group in groups {
                if let Some(name) = group.as_str().and_then(|g| first_equip_name(sim, g)) {
                    let _ = if include {
                        sim.add_include(&name)
                    } else {
                        sim.add_exclude(&name)
                    };
                    added = true;
                }
            }
        }
    }
    added
}

fn run_scenario(sim: &mut Simulator, scenario: &Value, out: &mut String) {
    let id = str_field(scenario, "id").unwrap_or_default();
    let limit = int_field(scenario, "limit").unwrap_or(10);
    let condition = make_condition(sim, scenario);
    let added_cludes = apply_first_cludes(sim, scenario);
    if bool_field(scenario, "extra", false) {
        let cancel = AtomicBool::new(false);
        let skills = match sim.search_extra_skill(&condition, &cancel, None) {
            Ok(skills) => skills,
            Err(error) => {
                write_error(out, &id, &error.to_string());
                cleanup_cludes(sim, added_cludes);
                return;
            }
        };
        out.push_str(&format!("BEGIN|{}\nEXTRA_COUNT|{}\n", id, skills.len()));
        for skill in &skills {
            out.push_str(&format!("EXTRA|{}|{}\n", skill.name, skill.level));
        }
        out.push_str(&format!("END|{}\n", id));
        cleanup_cludes(sim, added_cludes);
        return;
    }
    if bool_field(scenario, "charm_search", false) {
        if let Err(error) = sim.search(condition, limit, None) {
            write_error(out, &id, &error.to_string());
            cleanup_cludes(sim, added_cludes);
            return;
        }
        let cancel = AtomicBool::new(false);
        let sets = match sim.search_charm(&cancel, None) {
            Ok(sets) => sets,
            Err(error) => {
                write_error(out, &id, &error.to_string());
                cleanup_cludes(sim, added_cludes);
                return;
            }
        };
        write_results(&id, &sets, false, out);
        cleanup_cludes(sim, added_cludes);
        return;
    }
    let sets = match sim.search(condition, limit, None) {
        Ok(sets) => sets,
        Err(error) => {
            write_error(out, &id, &error.to_string());
            cleanup_cludes(sim, added_cludes);
            return;
        }
    };
    let sets = match int_field(scenario, "more") {
        Some(more) => match sim.search_more(more, None) {
            Ok(sets) => sets,
            Err(error) => {
                write_error(out, &id, &error.to_string());
                cleanup_cludes(sim, added_cludes);
                return;
            }
        },
        None => sets,
    };
    let all = sim.is_searched_all;
    write_results(&id, &sets, all, out);
    cleanup_cludes(sim, added_cludes);
}

fn write_error(out: &mut String, id: &str, message: &str) {
    out.push_str(&format!("BEGIN|{}\nERROR|{}\nEND|{}\n", id, message, id));
}

fn cleanup_cludes(sim: &mut Simulator, added: bool) {
    if added {
        let _ = sim.delete_all_clude();
    }
}

fn write_results(id: &str, sets: &[EquipSet], all: bool, out: &mut String) {
    let best = sets.first().map(|s| s.maxdef()).unwrap_or(-1);
    out.push_str(&format!(
        "BEGIN|{}\nCOUNT|{}\nALL|{}\nBEST|{}\n",
        id,
        sets.len(),
        if all { 1 } else { 0 },
        best
    ));
    for set in sets {
        out.push_str(&format!("RESULT|{}\n", set_signature(set)));
    }
    out.push_str(&format!("END|{}\n", id));
}

fn apply_mutations(sim: &mut Simulator, out: &mut String) {
    if let (Some(weapon), Some(head), Some(body), Some(charm)) = (
        sim.masters
            .weapons
            .iter()
            .find(|w| {
                w.weapon_type == WeaponType::None && w.slot1 == 1 && w.slot2 == 1 && w.slot3 == 0
            })
            .cloned(),
        sim.masters.heads.first().cloned(),
        sim.masters.bodys.first().cloned(),
        sim.masters.charms.first().cloned(),
    ) {
        let mut set = EquipSet::new();
        set.weapon = weapon;
        set.head = head;
        set.body = body;
        set.arm = sim.masters.arms.first().cloned().unwrap_or_default();
        set.waist = sim.masters.waists.first().cloned().unwrap_or_default();
        set.leg = sim.masters.legs.first().cloned().unwrap_or_default();
        set.charm = charm;
        set.name = "parity-set".to_string();
        let _ = sim.add_my_set(set);
    }
    if let Some(head) = sim.masters.heads.first().map(|e| e.name.clone()) {
        let _ = sim.add_include(&head);
    }
    if let Some(body) = sim.masters.bodys.first().map(|e| e.name.clone()) {
        let _ = sim.add_exclude(&body);
    }
    let first_skill = sim.masters.skills[0].name.clone();
    let second_skill = sim.masters.skills[1].name.clone();
    add_charm(
        sim,
        "parity-charm-1",
        &first_skill,
        2,
        Some(&second_skill),
        1,
    );
    add_charm(sim, "parity-charm-0", &first_skill, 1, None, 0);
    if let Some(deco) = sim.masters.decos.first() {
        let name = deco.name.clone();
        let _ = sim.save_deco_count(&name, 3);
    }
    let mut condition = SearchCondition::new();
    condition.add_skill(&sim.masters.make_skill(&first_skill, 2));
    condition.add_skill(&sim.masters.make_skill(&second_skill, 1));
    condition.weapon_type = WeaponType::None;
    condition.id = Some("parity-condition-1".to_string());
    condition.disp_name = Some("parity condition".to_string());
    let _ = sim.add_my_condition(condition);
    out.push_str("MUTATIONS|done\n");
}

fn add_charm(
    sim: &mut Simulator,
    name: &str,
    first_skill: &str,
    first_level: i32,
    second_skill: Option<&str>,
    second_level: i32,
) {
    let mut charm = Equipment::new(EquipKind::Charm);
    charm.name = name.to_string();
    charm.rare = 8;
    charm.slot1 = 2;
    charm.slot2 = 1;
    charm.slot3 = 0;
    charm.slot_type1 = 0;
    charm.slot_type2 = 1;
    charm.slot_type3 = 0;
    charm
        .skills
        .push(sim.masters.make_skill(first_skill, first_level));
    if let Some(second_skill) = second_skill {
        charm
            .skills
            .push(sim.masters.make_skill(second_skill, second_level));
    }
    let _ = sim.add_charm(charm);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let root = arg_value(&args, "--root").expect("--root");
    let scenarios_path = arg_value(&args, "--scenarios").expect("--scenarios");
    let out_path = arg_value(&args, "--out").expect("--out");
    let text = std::fs::read_to_string(&scenarios_path).expect("scenarios");
    let json: Value = serde_json::from_str(&text).expect("json");
    let mut sim = Simulator::load(Path::new(&root)).expect("load");
    let mut out = String::new();
    out.push_str(&dump_masters(&sim));
    for scenario in json.get("scenarios").and_then(|x| x.as_array()).unwrap() {
        run_scenario(&mut sim, scenario, &mut out);
    }
    apply_mutations(&mut sim, &mut out);
    out.push_str(&dump_masters(&sim));
    std::fs::write(&out_path, out).expect("write out");
}
