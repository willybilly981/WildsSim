use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::charm_appraiser::{is_left_upper, upper_info_for};
use crate::config::{LogicConfig, ViewConfig};
use crate::error::{SimError, SimResult};
use crate::file_operation::*;
use crate::model::*;
use crate::searcher::Searcher;

pub struct Simulator {
    pub masters: Masters,
    pub logic: LogicConfig,
    pub view: ViewConfig,
    pub file: FileOperation,
    searcher: Option<Searcher>,
}

impl Simulator {
    pub fn load(root: impl Into<PathBuf>) -> SimResult<Simulator> {
        let root = root.into();
        let logic = LogicConfig::load(&root.join("conf").join("LogicConfig.csv"))?;
        let view = ViewConfig::load(&root.join("conf").join("viewConfig.csv"))?;
        let file = FileOperation::new(root, logic.clone());
        let masters = load_masters(&file, &logic)?;
        Ok(Simulator {
            masters,
            logic,
            view,
            file,
            searcher: None,
        })
    }

    pub fn is_best_charm_search(&self) -> bool {
        self.searcher
            .as_ref()
            .map(|searcher| searcher.condition.is_best_charm_search)
            .unwrap_or(false)
    }

    pub fn add_exclude(&mut self, name: &str) -> SimResult<Option<Clude>> {
        let equip = match self.masters.get_equip_by_name(name) {
            Some(equip) => equip.clone(),
            None => return Ok(None),
        };
        if equip.kind == EquipKind::Weapon && equip.weapon_type == WeaponType::None {
            return Ok(None);
        }
        let mut cludes = self.masters.cludes.clone();
        for clude in &mut cludes {
            if clude.name == equip.name {
                clude.kind = CludeKind::Exclude;
            }
        }
        if !cludes.iter().any(|clude| clude.name == equip.name) {
            cludes.push(Clude {
                name: equip.name.clone(),
                kind: CludeKind::Exclude,
            });
        }
        let result = cludes
            .iter()
            .find(|clude| clude.name == equip.name)
            .cloned();
        self.commit_cludes(cludes)?;
        Ok(result)
    }

    pub fn add_include(&mut self, name: &str) -> SimResult<Option<Clude>> {
        let equip = match self.masters.get_equip_by_name(name) {
            Some(equip) => equip.clone(),
            None => return Ok(None),
        };
        if equip.kind == EquipKind::Deco || equip.kind == EquipKind::Weapon {
            return Ok(None);
        }
        // Only one include per part, and stale includes are dropped, all in one save.
        let mut cludes: Vec<Clude> = self
            .masters
            .cludes
            .iter()
            .filter(|clude| {
                clude.kind == CludeKind::Exclude
                    || self
                        .masters
                        .get_equip_by_name(&clude.name)
                        .map(|old| old.kind != equip.kind)
                        .unwrap_or(false)
            })
            .cloned()
            .collect();
        for clude in &mut cludes {
            if clude.name == equip.name {
                clude.kind = CludeKind::Include;
            }
        }
        if !cludes.iter().any(|clude| clude.name == equip.name) {
            cludes.push(Clude {
                name: equip.name.clone(),
                kind: CludeKind::Include,
            });
        }
        let result = cludes
            .iter()
            .find(|clude| clude.name == equip.name)
            .cloned();
        self.file.save_clude_csv(&cludes)?;
        self.masters.cludes = cludes;
        Ok(result)
    }

    fn commit_cludes(&mut self, cludes: Vec<Clude>) -> SimResult<()> {
        self.file.save_clude_csv(&cludes)?;
        self.masters.cludes = cludes;
        Ok(())
    }

    pub fn delete_clude(&mut self, name: &str, save: bool) -> SimResult<()> {
        if !save {
            self.masters.cludes.retain(|clude| clude.name != name);
            return Ok(());
        }
        let cludes: Vec<Clude> = self
            .masters
            .cludes
            .iter()
            .filter(|clude| clude.name != name)
            .cloned()
            .collect();
        self.commit_cludes(cludes)
    }

    pub fn delete_all_clude(&mut self) -> SimResult<()> {
        self.commit_cludes(Vec::new())
    }

    pub fn delete_all_armor_clude(&mut self) -> SimResult<()> {
        let cludes: Vec<Clude> = self
            .masters
            .cludes
            .iter()
            .filter(|clude| {
                self.masters
                    .get_equip_by_name(&clude.name)
                    .map(|equip| equip.kind == EquipKind::Weapon)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        self.commit_cludes(cludes)
    }

    pub fn delete_all_weapon_clude(&mut self) -> SimResult<()> {
        let cludes: Vec<Clude> = self
            .masters
            .cludes
            .iter()
            .filter(|clude| {
                self.masters
                    .get_equip_by_name(&clude.name)
                    .map(|equip| equip.kind != EquipKind::Weapon)
                    .unwrap_or(true)
            })
            .cloned()
            .collect();
        self.commit_cludes(cludes)
    }

    pub fn add_my_set(&mut self, mut set: EquipSet) -> SimResult<Option<EquipSet>> {
        if !set.charm.name.is_empty()
            && !self
                .masters
                .charms
                .iter()
                .chain(self.masters.additional_charms.iter())
                .any(|c| c.name == set.charm.name)
        {
            return Ok(None);
        }
        if !set.weapon.name.is_empty()
            && !self
                .masters
                .weapons
                .iter()
                .chain(self.masters.artians.iter())
                .any(|w| w.name == set.weapon.name)
        {
            return Ok(None);
        }
        if set.name.is_empty() {
            set.name = self.logic.default_my_set_name.clone();
        }
        let mut my_sets = self.masters.my_sets.clone();
        my_sets.push(set.clone());
        self.commit_my_sets(my_sets)?;
        Ok(Some(set))
    }

    pub fn delete_my_set_at(&mut self, index: usize) -> SimResult<()> {
        if index >= self.masters.my_sets.len() {
            return Ok(());
        }
        let mut my_sets = self.masters.my_sets.clone();
        my_sets.remove(index);
        self.commit_my_sets(my_sets)
    }

    pub fn move_my_set(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.my_sets.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let mut my_sets = self.masters.my_sets.clone();
        let set = my_sets.remove(drop_index);
        my_sets.insert(target_index, set);
        self.commit_my_sets(my_sets)
    }

    pub fn change_name_of_my_set(&mut self, name: &str, index: usize) -> SimResult<()> {
        if name.is_empty() || index >= self.masters.my_sets.len() {
            return Ok(());
        }
        if self.masters.my_sets[index].name == name {
            return Ok(());
        }
        let mut my_sets = self.masters.my_sets.clone();
        my_sets[index].name = name.to_string();
        self.commit_my_sets(my_sets)
    }

    // Save succeeds before memory changes, so a failed save never leaves the
    // app showing data the files do not have. The three files are still
    // written in order, so a mid-failure can leave charm/artian flags stale,
    // which the next save repairs.
    fn commit_my_sets(&mut self, my_sets: Vec<EquipSet>) -> SimResult<()> {
        self.file.save_my_set_csv(&my_sets)?;
        self.file
            .save_additional_charm_csv(&self.masters.additional_charms, &my_sets)?;
        self.file.save_artian_csv(&self.masters.artians, &my_sets)?;
        self.masters.my_sets = my_sets;
        Ok(())
    }

    pub fn take_searcher(&mut self) -> Option<Searcher> {
        self.searcher.take()
    }

    pub fn adopt_searcher(&mut self, searcher: Searcher) -> Vec<EquipSet> {
        let result = searcher.result_sets.clone();
        self.searcher = Some(searcher);
        result
    }

    pub fn current_condition(&self) -> Option<SearchCondition> {
        self.searcher
            .as_ref()
            .map(|searcher| searcher.condition.clone())
    }

    pub fn update_recent_skill(&mut self, skills: &[Skill]) -> SimResult<()> {
        let mut new_names: Vec<String> = Vec::new();
        for skill in skills {
            new_names.push(skill.name.clone());
        }
        for old_name in &self.masters.recent_skill_names {
            if self.logic.max_recent_skill_count <= new_names.len() as i32 {
                break;
            }
            if !new_names.iter().any(|name| name == old_name) {
                new_names.push(old_name.clone());
            }
        }
        self.file.save_recent_skill_csv(&new_names)?;
        self.masters.recent_skill_names = new_names;
        Ok(())
    }

    pub fn add_my_condition(&mut self, condition: SearchCondition) -> SimResult<()> {
        let mut conditions = self.masters.my_conditions.clone();
        conditions.push(condition);
        self.file.save_my_condition_csv(&conditions)?;
        self.masters.my_conditions = conditions;
        Ok(())
    }

    pub fn delete_my_condition_at(&mut self, index: usize) -> SimResult<()> {
        if index >= self.masters.my_conditions.len() {
            return Ok(());
        }
        let mut conditions = self.masters.my_conditions.clone();
        conditions.remove(index);
        self.file.save_my_condition_csv(&conditions)?;
        self.masters.my_conditions = conditions;
        Ok(())
    }

    pub fn change_name_of_my_condition(&mut self, name: &str, index: usize) -> SimResult<()> {
        if name.is_empty() || index >= self.masters.my_conditions.len() {
            return Ok(());
        }
        if self.masters.my_conditions[index].disp_name.as_deref() == Some(name) {
            return Ok(());
        }
        let mut conditions = self.masters.my_conditions.clone();
        conditions[index].disp_name = Some(name.to_string());
        self.file.save_my_condition_csv(&conditions)?;
        self.masters.my_conditions = conditions;
        Ok(())
    }

    pub fn save_deco_count(&mut self, deco_name: &str, count: i32) -> SimResult<()> {
        if count < 0 || !self.masters.decos.iter().any(|d| d.name == deco_name) {
            return Ok(());
        }
        let mut decos = self.masters.decos.clone();
        for deco in &mut decos {
            if deco.name == deco_name {
                deco.deco_count = count;
            }
        }
        self.file.save_deco_count_json(&decos)?;
        self.masters.decos = decos;
        Ok(())
    }

    pub fn set_all_deco_count(&mut self, count: i32) -> SimResult<()> {
        if count < 0 {
            return Ok(());
        }
        let mut decos = self.masters.decos.clone();
        for deco in &mut decos {
            deco.deco_count = count;
        }
        self.file.save_deco_count_json(&decos)?;
        self.masters.decos = decos;
        Ok(())
    }

    pub fn add_charm(&mut self, charm: Equipment) -> SimResult<()> {
        if self
            .masters
            .additional_charms
            .iter()
            .any(|c| c.name == charm.name)
        {
            return Ok(());
        }
        if let Some(name) = duplicate_skill_name(&charm) {
            return Err(SimError(format!("スキル {} が重複しています。", name)));
        }
        let mut charms = self.masters.additional_charms.clone();
        if self.logic.use_calc_upper_charm {
            let decos = &self.masters.decos;
            // The added charm's own upper is searched against the old list, and
            // each existing charm only needs a pair check against the addition,
            // not a re-search over the whole list.
            let added_upper = upper_info_for(decos, &charm, &charms, true)?;
            let mut added = charm;
            added.upper = added_upper;
            for old in charms.iter_mut() {
                if !is_left_upper(decos, &added, old, true)? {
                    continue;
                }
                let mutual = is_left_upper(decos, old, &added, true)?;
                let strict = !mutual;
                let replace = match &old.upper {
                    None => true,
                    Some(_) => strict,
                };
                if replace {
                    old.upper = Some(UpperInfo {
                        name: added.name.clone(),
                        disp_name: added.disp_name().to_string(),
                        is_full_upper: strict,
                    });
                }
            }
            charms.push(added);
        } else {
            charms.push(charm);
        }
        self.file
            .save_additional_charm_csv(&charms, &self.masters.my_sets)?;
        self.masters.additional_charms = charms;
        Ok(())
    }

    pub fn delete_charm(&mut self, name: &str) -> SimResult<()> {
        if !self
            .masters
            .additional_charms
            .iter()
            .any(|c| c.name == name)
        {
            return Ok(());
        }
        let mut charms: Vec<Equipment> = self
            .masters
            .additional_charms
            .iter()
            .filter(|c| c.name != name)
            .cloned()
            .collect();
        let my_sets: Vec<EquipSet> = self
            .masters
            .my_sets
            .iter()
            .filter(|set| set.charm.name != name)
            .cloned()
            .collect();
        let removed_sets = my_sets.len() != self.masters.my_sets.len();
        if self.logic.use_calc_upper_charm {
            let decos = &self.masters.decos;
            // Only charms that named the deleted one need a fresh upper search.
            for index in 0..charms.len() {
                if charms[index].upper.as_ref().map(|u| u.name.as_str()) != Some(name) {
                    continue;
                }
                let (before, rest) = charms.split_at_mut(index);
                let (charm, after) = rest.split_first_mut().unwrap();
                let candidates: Vec<Equipment> =
                    before.iter().chain(after.iter()).cloned().collect();
                charm.upper = upper_info_for(decos, charm, &candidates, true)?;
            }
        }
        self.file.save_additional_charm_csv(&charms, &my_sets)?;
        if removed_sets {
            self.file.save_my_set_csv(&my_sets)?;
            self.file.save_artian_csv(&self.masters.artians, &my_sets)?;
        }
        self.masters.additional_charms = charms;
        self.masters.my_sets = my_sets;
        self.delete_clude(name, true)
    }

    pub fn move_charm(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.additional_charms.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let mut charms = self.masters.additional_charms.clone();
        let charm = charms.remove(drop_index);
        charms.insert(target_index, charm);
        self.file
            .save_additional_charm_csv(&charms, &self.masters.my_sets)?;
        self.masters.additional_charms = charms;
        Ok(())
    }

    pub fn add_artian(&mut self, artian: Equipment) -> SimResult<()> {
        if self.masters.artians.iter().any(|a| a.name == artian.name) {
            return Ok(());
        }
        if let Some(name) = duplicate_skill_name(&artian) {
            return Err(SimError(format!("スキル {} が重複しています。", name)));
        }
        let mut artians = self.masters.artians.clone();
        artians.push(artian);
        self.file.save_artian_csv(&artians, &self.masters.my_sets)?;
        self.masters.artians = artians;
        Ok(())
    }

    pub fn delete_artian(&mut self, name: &str) -> SimResult<()> {
        if !self.masters.artians.iter().any(|a| a.name == name) {
            return Ok(());
        }
        let artians: Vec<Equipment> = self
            .masters
            .artians
            .iter()
            .filter(|a| a.name != name)
            .cloned()
            .collect();
        let my_sets: Vec<EquipSet> = self
            .masters
            .my_sets
            .iter()
            .filter(|set| set.weapon.name != name)
            .cloned()
            .collect();
        let removed_sets = my_sets.len() != self.masters.my_sets.len();
        self.file.save_artian_csv(&artians, &my_sets)?;
        if removed_sets {
            self.file.save_my_set_csv(&my_sets)?;
            self.file
                .save_additional_charm_csv(&self.masters.additional_charms, &my_sets)?;
        }
        self.masters.artians = artians;
        self.masters.my_sets = my_sets;
        self.delete_clude(name, true)
    }

    pub fn move_artian(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.artians.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let mut artians = self.masters.artians.clone();
        let artian = artians.remove(drop_index);
        artians.insert(target_index, artian);
        self.file.save_artian_csv(&artians, &self.masters.my_sets)?;
        self.masters.artians = artians;
        Ok(())
    }
}

fn duplicate_skill_name(equip: &Equipment) -> Option<&str> {
    let mut seen = HashSet::new();
    equip
        .skills
        .iter()
        .find(|skill| !seen.insert(skill.name.as_str()))
        .map(|skill| skill.name.as_str())
}

fn calc_charm_uppers(
    decos: &[Equipment],
    charms: &[Equipment],
    logic: &LogicConfig,
) -> SimResult<Vec<Option<UpperInfo>>> {
    if !logic.use_calc_upper_charm {
        return Ok(vec![None; charms.len()]);
    }
    let compute = || {
        charms
            .par_iter()
            .map(|charm| upper_info_for(decos, charm, charms, true))
            .collect::<SimResult<Vec<Option<UpperInfo>>>>()
    };
    run_parallel(logic, compute)
}

fn run_parallel<T: Send>(
    logic: &LogicConfig,
    run: impl FnOnce() -> SimResult<T> + Send,
) -> SimResult<T> {
    match logic.max_degree_of_parallelism {
        n if n > 0 => {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(n as usize)
                .build()
                .map_err(|error| SimError(error.to_string()))?;
            pool.install(run)
        }
        _ => run(),
    }
}

fn load_masters(file: &FileOperation, logic: &LogicConfig) -> SimResult<Masters> {
    let mut masters = Masters::default();
    masters.def_upgrades = file.load_def_upgrade_csv()?;
    let raw_skills = file.load_skill_csv()?;
    let raw_lookup = SkillLookup::new(&raw_skills);
    let heads = file.load_equip_csv(
        HEAD_CSV,
        EquipKind::Head,
        Some(&masters.def_upgrades),
        &raw_lookup,
    )?;
    let bodys = file.load_equip_csv(
        BODY_CSV,
        EquipKind::Body,
        Some(&masters.def_upgrades),
        &raw_lookup,
    )?;
    let arms = file.load_equip_csv(
        ARM_CSV,
        EquipKind::Arm,
        Some(&masters.def_upgrades),
        &raw_lookup,
    )?;
    let waists = file.load_equip_csv(
        WAIST_CSV,
        EquipKind::Waist,
        Some(&masters.def_upgrades),
        &raw_lookup,
    )?;
    let legs = file.load_equip_csv(
        LEG_CSV,
        EquipKind::Leg,
        Some(&masters.def_upgrades),
        &raw_lookup,
    )?;
    let charms = file.load_equip_csv(CHARM_CSV, EquipKind::Charm, None, &raw_lookup)?;
    let decos = file.load_deco_csv(&raw_lookup)?;
    let weapons = file.load_weapon_csv(&raw_lookup)?;

    let mut used_names: HashMap<&str, ()> = HashMap::new();
    for equip in weapons
        .iter()
        .chain(heads.iter())
        .chain(bodys.iter())
        .chain(arms.iter())
        .chain(waists.iter())
        .chain(legs.iter())
        .chain(charms.iter())
        .chain(decos.iter())
    {
        for skill in &equip.skills {
            used_names.insert(skill.name.as_str(), ());
        }
    }
    let filtered_skills: Vec<Skill> = raw_skills
        .iter()
        .filter(|skill| used_names.contains_key(skill.name.as_str()))
        .cloned()
        .collect();
    masters.set_skills(filtered_skills);
    let lookup = masters.skill_lookup().clone();

    masters.heads = heads;
    masters.bodys = bodys;
    masters.arms = arms;
    masters.waists = waists;
    masters.legs = legs;
    masters.charms = charms;
    masters.decos = decos;
    masters.weapons = weapons;

    masters.shining_charm_combos = file.load_shining_charm_combo_csv()?;
    masters.shining_charm_groups = file.load_shining_charm_group_csv(&lookup)?;

    file.make_save_folder()?;
    masters.additional_charms = file.load_additional_charm_csv(&lookup)?;
    masters.artians = file.load_artian_csv(&lookup)?;
    masters.cludes = file.load_clude_csv()?;
    masters.recent_skill_names = file.load_recent_skill_csv()?;
    masters.my_conditions = file.load_my_condition_csv(&lookup)?;

    let mut all_equips: Vec<Equipment> = Vec::new();
    all_equips.extend(masters.weapons.iter().cloned());
    all_equips.extend(masters.artians.iter().cloned());
    all_equips.extend(masters.heads.iter().cloned());
    all_equips.extend(masters.bodys.iter().cloned());
    all_equips.extend(masters.arms.iter().cloned());
    all_equips.extend(masters.waists.iter().cloned());
    all_equips.extend(masters.legs.iter().cloned());
    all_equips.extend(masters.charms.iter().cloned());
    all_equips.extend(masters.additional_charms.iter().cloned());
    all_equips.extend(masters.decos.iter().cloned());
    masters.my_sets = file.load_my_set_csv(&all_equips)?;

    for condition in &mut masters.my_conditions {
        if let Some(weapon_name) = &condition.weapon_name {
            if let Some(artian) = masters.artians.iter().find(|a| a.name == *weapon_name) {
                condition.weapon_disp_name = Some(artian.disp_name().to_string());
            }
        }
    }

    let uppers = calc_charm_uppers(&masters.decos, &masters.additional_charms, logic)?;
    for (charm, upper) in masters.additional_charms.iter_mut().zip(uppers) {
        charm.upper = upper;
    }
    Ok(masters)
}

pub fn search_extra_skill_impl(
    masters: &Masters,
    logic: &LogicConfig,
    condition: &SearchCondition,
    cancel: &AtomicBool,
    progress: Option<&AtomicUsize>,
) -> SimResult<Vec<Skill>> {
    let range = build_search_range(condition, masters);
    let skills = &masters.skills;
    let run = || {
        skills
            .par_iter()
            .map(|skill| -> SimResult<Vec<Skill>> {
                let mut result: Vec<Skill> = Vec::new();
                if cancel.load(Ordering::Relaxed) {
                    return Ok(result);
                }
                for level in 1..=skill.level {
                    // Levels that never activate (e.g. the gaps in a group or
                    // series skill) cannot be applied; solving for them is
                    // wasted work.
                    if skill.is_hide_level(Some(level)) {
                        continue;
                    }
                    let mut ex_condition = condition.copy_for_search();
                    let ex_skill = masters.make_skill(&skill.name, level);
                    let is_new_skill = ex_condition.add_skill(&ex_skill);
                    if is_new_skill {
                        // Virtual charms/artians must be rebuilt per iteration:
                        // they are generated from the skills under test.
                        let range = if ex_condition.is_best_charm_search
                            || ex_condition.is_best_artian_search
                        {
                            build_search_range(&ex_condition, masters)
                        } else {
                            range.clone()
                        };
                        let mut searcher = Searcher::new(ex_condition, range)?;
                        searcher.exec_search(1, None)?;
                        if searcher.result_sets.is_empty() {
                            break;
                        }
                        result.push(ex_skill);
                    }
                }
                if let Some(progress) = progress {
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                Ok(result)
            })
            .collect::<SimResult<Vec<Vec<Skill>>>>()
    };
    let results = run_parallel(logic, run)?;
    let mut ex_skills: Vec<Skill> = Vec::new();
    for result in results {
        ex_skills.extend(result);
    }
    let mut sorted = Vec::new();
    for skill in &masters.skills {
        for result in &ex_skills {
            if skill.name == result.name {
                sorted.push(result.clone());
            }
        }
    }
    Ok(sorted)
}

pub fn search_charm_impl(
    masters: &Masters,
    logic: &LogicConfig,
    condition: &SearchCondition,
    cancel: &AtomicBool,
    progress: Option<&AtomicUsize>,
) -> SimResult<Vec<EquipSet>> {
    let target_charms =
        condition.make_related_charms(&masters.shining_charm_combos, &masters.shining_charm_groups);
    let base_range = build_search_range(condition, masters);
    let compute = || -> SimResult<Vec<EquipSet>> {
        let candidates: Vec<Option<EquipSet>> = target_charms
            .par_iter()
            .map(|target_charm| -> SimResult<Option<EquipSet>> {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                let ex_condition = condition.copy_for_search();
                let mut range = base_range.clone();
                range.charms = vec![target_charm.clone()];
                let mut ex_searcher = Searcher::new(ex_condition, range)?;
                ex_searcher.exec_search(1, None)?;
                if let Some(progress) = progress {
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                Ok(ex_searcher.result_sets.first().cloned())
            })
            .collect::<SimResult<Vec<Option<EquipSet>>>>()?;
        let result_sets: Vec<EquipSet> = candidates.into_iter().flatten().collect();
        if result_sets.len() < 2 {
            return Ok(result_sets);
        }
        let decos = &masters.decos;
        result_sets
            .par_iter()
            .enumerate()
            .map(|(left_index, left)| -> SimResult<Option<EquipSet>> {
                // On cancel every entry is kept, returning the unfiltered list.
                if cancel.load(Ordering::Relaxed) {
                    return Ok(Some(left.clone()));
                }
                for (right_index, right) in result_sets.iter().enumerate() {
                    if left_index == right_index {
                        continue;
                    }
                    if is_left_upper(decos, &left.charm, &right.charm, false)? {
                        // Equivalent charms satisfy each other both ways; the
                        // earlier candidate wins so they do not remove each
                        // other.
                        if left_index > right_index
                            || !is_left_upper(decos, &right.charm, &left.charm, false)?
                        {
                            return Ok(None);
                        }
                    }
                }
                Ok(Some(left.clone()))
            })
            .collect::<SimResult<Vec<Option<EquipSet>>>>()
            .map(|filtered| filtered.into_iter().flatten().collect())
    };
    run_parallel(logic, compute)
}
