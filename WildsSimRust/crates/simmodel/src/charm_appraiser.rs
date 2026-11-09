use crate::error::SimResult;
use crate::model::*;
use crate::searcher::Searcher;

fn generic_type_name(slot_type: i32) -> &'static str {
    match slot_type {
        0 => "防",
        1 => "攻",
        2 => "両",
        _ => "",
    }
}

fn generic_size_name(size: i32) -> &'static str {
    match size {
        1 => "１",
        2 => "２",
        3 => "３",
        4 => "４",
        _ => "",
    }
}

fn generic_deco_name(slot_type: i32, size: i32) -> String {
    format!(
        "_temP_{}汎珠【{}】",
        generic_type_name(slot_type),
        generic_size_name(size)
    )
}

pub fn make_generic_decos() -> Vec<Equipment> {
    let mut decos = Vec::new();
    for slot_type in 0..=2 {
        for size in 1..=4 {
            let name = generic_deco_name(slot_type, size);
            let mut deco = Equipment::new_deco();
            deco.name = name.clone();
            deco.slot_type1 = if slot_type == 2 { 3 } else { slot_type };
            deco.slot1 = size;
            let mut skill = Skill::default();
            skill.name = name;
            skill.level = 1;
            skill.max_level = 1;
            skill.category = "未分類".to_string();
            deco.skills.push(skill);
            decos.push(deco);
        }
    }
    decos
}

pub fn is_left_upper(
    masters: &Masters,
    left: &Equipment,
    right: &Equipment,
    use_decos: bool,
) -> SimResult<bool> {
    let mut searcher = make_charm_searcher(masters, right, std::slice::from_ref(left), use_decos)?;
    searcher.exec_search(1, None)?;
    Ok(!searcher.result_sets.is_empty())
}

pub fn has_upper_charm(
    masters: &Masters,
    charm: &Equipment,
    use_decos: bool,
) -> SimResult<Option<Equipment>> {
    let charm_range: Vec<Equipment> = masters
        .additional_charms
        .iter()
        .filter(|candidate| candidate.name != charm.name)
        .cloned()
        .collect();
    let mut searcher = make_charm_searcher(masters, charm, &charm_range, use_decos)?;
    searcher.exec_search(1, None)?;
    Ok(searcher.result_sets.first().map(|set| set.charm.clone()))
}

fn make_charm_searcher(
    masters: &Masters,
    base_charm: &Equipment,
    charm_range: &[Equipment],
    use_decos: bool,
) -> SimResult<Searcher> {
    let generic_decos = make_generic_decos();
    let mut condition = SearchCondition::new();
    condition.skills = base_charm.skills.clone();
    condition.has_all_decos = true;
    for slot_type in 0..=2 {
        for size in 1..=4 {
            let mut count = 0;
            if base_charm.slot1 == size && base_charm.slot_type1 == slot_type {
                count += 1;
            }
            if base_charm.slot2 == size && base_charm.slot_type2 == slot_type {
                count += 1;
            }
            if base_charm.slot3 == size && base_charm.slot_type3 == slot_type {
                count += 1;
            }
            if count > 0 {
                let mut skill = Skill::default();
                skill.name = generic_deco_name(slot_type, size);
                skill.level = count;
                skill.max_level = count;
                condition.skills.push(skill);
            }
        }
    }
    let mut range = SearchRange::new_empty();
    range.charms = charm_range.to_vec();
    if use_decos {
        range.decos.extend(generic_decos);
        range.decos.extend(masters.decos.iter().cloned());
    } else {
        range.decos = generic_decos;
    }
    Searcher::new(condition, range)
}
