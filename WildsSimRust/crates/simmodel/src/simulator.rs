use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use rayon::prelude::*;

use crate::charm_appraiser::{has_upper_charm, is_left_upper};
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
    pub is_searched_all: bool,
    pub is_canceling: bool,
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
            is_searched_all: false,
            is_canceling: false,
        })
    }

    pub fn is_best_charm_search(&self) -> bool {
        self.searcher
            .as_ref()
            .map(|searcher| searcher.condition.is_best_charm_search)
            .unwrap_or(false)
    }

    pub fn is_best_artian_search(&self) -> bool {
        self.searcher
            .as_ref()
            .map(|searcher| searcher.condition.is_best_artian_search)
            .unwrap_or(false)
    }

    pub fn current_result_sets(&self) -> Vec<EquipSet> {
        self.searcher
            .as_ref()
            .map(|searcher| searcher.result_sets.clone())
            .unwrap_or_default()
    }

    pub fn search(
        &mut self,
        condition: SearchCondition,
        limit: i32,
        cancel: Option<&AtomicBool>,
    ) -> SimResult<Vec<EquipSet>> {
        self.is_canceling = false;
        let range = build_search_range(&condition, &self.masters);
        let skills = condition.skills.clone();
        let mut searcher = Searcher::new(condition, range)?;
        self.is_searched_all = searcher.exec_search(limit, cancel)?;
        if cancel.map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
            self.is_canceling = true;
        }
        let result = searcher.result_sets.clone();
        self.searcher = Some(searcher);
        self.update_recent_skill(&skills)?;
        Ok(result)
    }

    pub fn search_more(
        &mut self,
        limit: i32,
        cancel: Option<&AtomicBool>,
    ) -> SimResult<Vec<EquipSet>> {
        self.is_canceling = false;
        let searcher = match self.searcher.as_mut() {
            Some(searcher) => searcher,
            None => return Ok(Vec::new()),
        };
        self.is_searched_all = searcher.exec_search(limit, cancel)?;
        if cancel.map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
            self.is_canceling = true;
        }
        Ok(searcher.result_sets.clone())
    }

    pub fn search_extra_skill(
        &mut self,
        condition: &SearchCondition,
        cancel: &AtomicBool,
        progress: Option<&AtomicUsize>,
    ) -> SimResult<Vec<Skill>> {
        self.is_canceling = false;
        let result =
            search_extra_skill_impl(&self.masters, &self.logic, condition, cancel, progress)?;
        if cancel.load(Ordering::Relaxed) {
            self.is_canceling = true;
        }
        Ok(result)
    }

    pub fn search_charm(
        &mut self,
        cancel: &AtomicBool,
        progress: Option<&AtomicUsize>,
    ) -> SimResult<Vec<EquipSet>> {
        self.is_canceling = false;
        let searcher = match self.searcher.as_ref() {
            Some(searcher) => searcher,
            None => return Ok(Vec::new()),
        };
        let condition = searcher.condition.clone();
        let filtered = search_charm_impl(&self.masters, &self.logic, &condition, cancel, progress)?;
        if cancel.load(Ordering::Relaxed) {
            self.is_canceling = true;
        }
        Ok(filtered)
    }

    pub fn add_exclude(&mut self, name: &str) -> SimResult<Option<Clude>> {
        let equip = match self.masters.get_equip_by_name(name) {
            Some(equip) => equip.clone(),
            None => return Ok(None),
        };
        if equip.kind == EquipKind::Weapon && equip.weapon_type == WeaponType::None {
            return Ok(None);
        }
        self.add_clude(&equip.name, CludeKind::Exclude, true)
    }

    pub fn add_include(&mut self, name: &str) -> SimResult<Option<Clude>> {
        let equip = match self.masters.get_equip_by_name(name) {
            Some(equip) => equip.clone(),
            None => return Ok(None),
        };
        if equip.kind == EquipKind::Deco || equip.kind == EquipKind::Weapon {
            return Ok(None);
        }
        let mut to_delete = Vec::new();
        for clude in &self.masters.cludes {
            if clude.kind == CludeKind::Exclude {
                continue;
            }
            match self.masters.get_equip_by_name(&clude.name) {
                None => to_delete.push(clude.name.clone()),
                Some(old) if old.kind == equip.kind => to_delete.push(clude.name.clone()),
                _ => {}
            }
        }
        for name in to_delete {
            self.delete_clude(&name, false)?;
        }
        self.add_clude(&equip.name, CludeKind::Include, true)
    }

    fn add_clude(&mut self, name: &str, kind: CludeKind, save: bool) -> SimResult<Option<Clude>> {
        let mut found = false;
        let mut result = None;
        for clude in &mut self.masters.cludes {
            if clude.name == name {
                clude.kind = kind;
                found = true;
                result = Some(clude.clone());
            }
        }
        if !found {
            let clude = Clude {
                name: name.to_string(),
                kind,
            };
            self.masters.cludes.push(clude.clone());
            result = Some(clude);
        }
        if save {
            self.file.save_clude_csv(&self.masters.cludes)?;
        }
        Ok(result)
    }

    pub fn delete_clude(&mut self, name: &str, save: bool) -> SimResult<()> {
        if let Some(index) = self.masters.cludes.iter().position(|c| c.name == name) {
            self.masters.cludes.remove(index);
        }
        if save {
            self.file.save_clude_csv(&self.masters.cludes)?;
        }
        Ok(())
    }

    pub fn delete_all_clude(&mut self) -> SimResult<()> {
        self.masters.cludes.clear();
        self.file.save_clude_csv(&self.masters.cludes)
    }

    pub fn delete_all_armor_clude(&mut self) -> SimResult<()> {
        let mut weapon_cludes = Vec::new();
        for clude in &self.masters.cludes {
            match self.masters.get_equip_by_name(&clude.name) {
                None => weapon_cludes.push(clude.clone()),
                Some(equip) if equip.kind == EquipKind::Weapon => weapon_cludes.push(clude.clone()),
                _ => {}
            }
        }
        self.masters.cludes = weapon_cludes;
        self.file.save_clude_csv(&self.masters.cludes)
    }

    pub fn delete_all_weapon_clude(&mut self) -> SimResult<()> {
        let mut armor_cludes = Vec::new();
        for clude in &self.masters.cludes {
            match self.masters.get_equip_by_name(&clude.name) {
                None => armor_cludes.push(clude.clone()),
                Some(equip) if equip.kind != EquipKind::Weapon => armor_cludes.push(clude.clone()),
                _ => {}
            }
        }
        self.masters.cludes = armor_cludes;
        self.file.save_clude_csv(&self.masters.cludes)
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
        self.masters.my_sets.push(set.clone());
        self.save_my_set()?;
        Ok(Some(set))
    }

    pub fn delete_my_set_at(&mut self, index: usize) -> SimResult<()> {
        if index >= self.masters.my_sets.len() {
            return Ok(());
        }
        self.masters.my_sets.remove(index);
        self.save_my_set()
    }

    pub fn move_my_set(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.my_sets.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let set = self.masters.my_sets.remove(drop_index);
        self.masters.my_sets.insert(target_index, set);
        self.save_my_set()
    }

    pub fn change_name_of_my_set(&mut self, name: &str, index: usize) -> SimResult<()> {
        if name.is_empty() || index >= self.masters.my_sets.len() {
            return Ok(());
        }
        if self.masters.my_sets[index].name == name {
            return Ok(());
        }
        self.masters.my_sets[index].name = name.to_string();
        self.save_my_set()
    }

    fn save_my_set(&mut self) -> SimResult<()> {
        self.file.save_my_set_csv(&self.masters.my_sets)?;
        self.file
            .save_additional_charm_csv(&self.masters.additional_charms, &self.masters.my_sets)?;
        self.file
            .save_artian_csv(&self.masters.artians, &self.masters.my_sets)?;
        Ok(())
    }

    pub fn take_searcher(&mut self) -> Option<Searcher> {
        self.searcher.take()
    }

    pub fn adopt_searcher(
        &mut self,
        searcher: Searcher,
        all_searched: bool,
        canceled: bool,
    ) -> Vec<EquipSet> {
        self.is_searched_all = all_searched;
        self.is_canceling = canceled;
        let result = searcher.result_sets.clone();
        self.searcher = Some(searcher);
        result
    }

    pub fn set_searcher(
        &mut self,
        searcher: Searcher,
        all_searched: bool,
        skills: &[Skill],
        canceled: bool,
    ) -> SimResult<Vec<EquipSet>> {
        self.update_recent_skill(skills)?;
        Ok(self.adopt_searcher(searcher, all_searched, canceled))
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
        self.masters.recent_skill_names = new_names;
        self.file
            .save_recent_skill_csv(&self.masters.recent_skill_names)
    }

    pub fn add_my_condition(&mut self, condition: SearchCondition) -> SimResult<()> {
        self.masters.my_conditions.push(condition);
        self.file.save_my_condition_csv(&self.masters.my_conditions)
    }

    pub fn delete_my_condition_at(&mut self, index: usize) -> SimResult<()> {
        if index >= self.masters.my_conditions.len() {
            return Ok(());
        }
        self.masters.my_conditions.remove(index);
        self.file.save_my_condition_csv(&self.masters.my_conditions)
    }

    pub fn change_name_of_my_condition(&mut self, name: &str, index: usize) -> SimResult<()> {
        if name.is_empty() || index >= self.masters.my_conditions.len() {
            return Ok(());
        }
        if self.masters.my_conditions[index].disp_name.as_deref() == Some(name) {
            return Ok(());
        }
        self.masters.my_conditions[index].disp_name = Some(name.to_string());
        self.file.save_my_condition_csv(&self.masters.my_conditions)
    }

    pub fn save_deco_count(&mut self, deco_name: &str, count: i32) -> SimResult<()> {
        if count < 0 {
            return Ok(());
        }
        if let Some(deco) = self.masters.decos.iter_mut().find(|d| d.name == deco_name) {
            deco.deco_count = count;
            self.file.save_deco_count_json(&self.masters.decos)?;
        }
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
        self.masters.additional_charms.push(charm);
        self.calc_lower_charm()?;
        self.file
            .save_additional_charm_csv(&self.masters.additional_charms, &self.masters.my_sets)
    }

    pub fn delete_charm(&mut self, name: &str) -> SimResult<()> {
        let index = match self
            .masters
            .additional_charms
            .iter()
            .position(|c| c.name == name)
        {
            Some(index) => index,
            None => return Ok(()),
        };
        self.delete_clude(name, true)?;
        let before = self.masters.my_sets.len();
        self.masters.my_sets.retain(|set| set.charm.name != name);
        let removed_sets = self.masters.my_sets.len() != before;
        self.masters.additional_charms.remove(index);
        self.calc_lower_charm()?;
        if removed_sets {
            self.save_my_set()
        } else {
            self.file
                .save_additional_charm_csv(&self.masters.additional_charms, &self.masters.my_sets)
        }
    }

    pub fn move_charm(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.additional_charms.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let charm = self.masters.additional_charms.remove(drop_index);
        self.masters.additional_charms.insert(target_index, charm);
        self.file
            .save_additional_charm_csv(&self.masters.additional_charms, &self.masters.my_sets)
    }

    pub fn add_artian(&mut self, artian: Equipment) -> SimResult<()> {
        if self.masters.artians.iter().any(|a| a.name == artian.name) {
            return Ok(());
        }
        self.masters.artians.push(artian);
        self.file
            .save_artian_csv(&self.masters.artians, &self.masters.my_sets)
    }

    pub fn delete_artian(&mut self, name: &str) -> SimResult<()> {
        let index = match self.masters.artians.iter().position(|a| a.name == name) {
            Some(index) => index,
            None => return Ok(()),
        };
        self.delete_clude(name, true)?;
        let before = self.masters.my_sets.len();
        self.masters.my_sets.retain(|set| set.weapon.name != name);
        let removed_sets = self.masters.my_sets.len() != before;
        self.masters.artians.remove(index);
        if removed_sets {
            self.save_my_set()
        } else {
            self.file
                .save_artian_csv(&self.masters.artians, &self.masters.my_sets)
        }
    }

    pub fn move_artian(&mut self, drop_index: usize, target_index: usize) -> SimResult<()> {
        let count = self.masters.artians.len();
        if drop_index >= count || target_index >= count || drop_index == target_index {
            return Ok(());
        }
        let artian = self.masters.artians.remove(drop_index);
        self.masters.artians.insert(target_index, artian);
        self.file
            .save_artian_csv(&self.masters.artians, &self.masters.my_sets)
    }

    fn calc_lower_charm(&mut self) -> SimResult<()> {
        calc_lower_charm_impl(&mut self.masters, &self.logic)
    }
}

fn calc_lower_charm_impl(masters: &mut Masters, logic: &LogicConfig) -> SimResult<()> {
    if !logic.use_calc_upper_charm {
        return Ok(());
    }
    for charm in &mut masters.additional_charms {
        charm.upper = None;
    }
    let snapshot = masters.additional_charms.clone();
    let compute = || {
        snapshot
            .par_iter()
            .map(|charm| -> SimResult<Option<UpperInfo>> {
                let upper = match has_upper_charm(masters, charm, true)? {
                    Some(upper) => upper,
                    None => return Ok(None),
                };
                let is_upper = is_left_upper(masters, charm, &upper, true)?;
                Ok(Some(UpperInfo {
                    name: upper.name.clone(),
                    disp_name: upper.disp_name().to_string(),
                    is_full_upper: !is_upper,
                }))
            })
            .collect::<SimResult<Vec<Option<UpperInfo>>>>()
    };
    let uppers = run_parallel(logic, compute)?;
    for (charm, upper) in masters.additional_charms.iter_mut().zip(uppers) {
        charm.upper = upper;
    }
    Ok(())
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

    file.save_additional_charm_csv(&masters.additional_charms, &masters.my_sets)?;
    file.save_artian_csv(&masters.artians, &masters.my_sets)?;

    let mut masters = masters;
    calc_lower_charm_impl(&mut masters, logic)?;
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
                    let mut ex_condition = condition.copy_for_search();
                    let ex_skill = masters.make_skill(&skill.name, level);
                    let is_new_skill = ex_condition.add_skill(&ex_skill);
                    if is_new_skill {
                        let mut searcher = Searcher::new(ex_condition, range.clone())?;
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
    let run = || {
        target_charms
            .par_iter()
            .map(|target_charm| -> SimResult<Option<EquipSet>> {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(None);
                }
                let ex_condition = condition.copy_for_search();
                let mut range = build_search_range(&ex_condition, masters);
                range.charms = vec![target_charm.clone()];
                range.cludes = masters.cludes.clone();
                let mut ex_searcher = Searcher::new(ex_condition, range)?;
                ex_searcher.exec_search(1, None)?;
                if let Some(progress) = progress {
                    progress.fetch_add(1, Ordering::Relaxed);
                }
                Ok(ex_searcher.result_sets.first().cloned())
            })
            .collect::<SimResult<Vec<Option<EquipSet>>>>()
    };
    let results = run_parallel(logic, run)?;
    let result_sets: Vec<EquipSet> = results.into_iter().flatten().collect();
    let mut filtered = Vec::new();
    for (left_index, left) in result_sets.iter().enumerate() {
        let mut has_lower = false;
        for (right_index, right) in result_sets.iter().enumerate() {
            if left_index == right_index {
                continue;
            }
            if is_left_upper(masters, &left.charm, &right.charm, false)? {
                has_lower = true;
                break;
            }
        }
        if !has_lower {
            filtered.push(left.clone());
        }
    }
    Ok(filtered)
}
