use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};

use microlp::{
    ComparisonOp, Error as LpError, LinearExpr, OptimizationDirection, Problem, Solution, Variable,
};

use crate::error::{SimError, SimResult};
use crate::model::*;

const MAX_DECO_COUNT: i32 = 7;

pub struct Searcher {
    pub condition: SearchCondition,
    pub result_sets: Vec<EquipSet>,
    all_equips: Vec<Equipment>,
    cludes: Vec<Clude>,
    variables: HashMap<String, Variable>,
    problem: Option<Problem>,
    solution: Option<Solution>,
    pending_exclusion: Option<(LinearExpr, f64)>,
    all_searched: bool,
}

impl Searcher {
    pub fn new(condition: SearchCondition, range: SearchRange) -> SimResult<Searcher> {
        let mut all_equips: Vec<Equipment> = Vec::new();
        all_equips.extend(range.weapons.iter().cloned());
        all_equips.extend(range.heads.iter().cloned());
        all_equips.extend(range.bodys.iter().cloned());
        all_equips.extend(range.arms.iter().cloned());
        all_equips.extend(range.waists.iter().cloned());
        all_equips.extend(range.legs.iter().cloned());
        all_equips.extend(range.charms.iter().cloned());
        all_equips.extend(range.decos.iter().cloned());

        let mut seen_skills = HashSet::new();
        for skill in &condition.skills {
            if !seen_skills.insert(skill.name.clone()) {
                return Err(SimError(format!(
                    "スキル {} が重複しています。",
                    skill.name
                )));
            }
        }

        let mut searcher = Searcher {
            condition,
            result_sets: Vec::new(),
            all_equips,
            cludes: range.cludes,
            variables: HashMap::new(),
            problem: None,
            solution: None,
            pending_exclusion: None,
            all_searched: false,
        };
        searcher.build()?;
        Ok(searcher)
    }

    pub fn exec_search(&mut self, limit: i32, cancel: Option<&AtomicBool>) -> SimResult<bool> {
        // Match the C# behavior: a limit of zero or less adds no results.
        if limit <= 0 {
            return Ok(false);
        }
        let target = self.result_sets.len() as i64 + limit as i64;
        let solution = match self.solution.take() {
            Some(solution) => solution,
            None => {
                let problem = match self.problem.take() {
                    Some(problem) => problem,
                    None => return Ok(true),
                };
                match solve_problem(problem)? {
                    Some(solution) => solution,
                    None => {
                        self.all_searched = true;
                        return Ok(true);
                    }
                }
            }
        };

        let mut current = Some(solution);
        let mut all_searched = false;
        loop {
            if let Some((expr, rhs)) = self.pending_exclusion.take() {
                let previous = current.take().unwrap();
                match previous.add_constraint(expr, ComparisonOp::Le, rhs) {
                    Ok(outcome) => match outcome.into_solution() {
                        Ok(next) => current = Some(next),
                        Err(_) => {
                            return Err(SimError(
                                "検索が制限時間内に完了しませんでした。".to_string(),
                            ))
                        }
                    },
                    Err(LpError::Infeasible) => {
                        all_searched = true;
                        break;
                    }
                    Err(error) => {
                        return Err(SimError(format!("検索に失敗しました。: {}", error)));
                    }
                }
            }

            let solution = current.as_ref().unwrap();
            let set = match self.make_set(solution) {
                Some(set) => set,
                None => {
                    all_searched = true;
                    break;
                }
            };
            self.remove_duplicate_set(&set);
            self.result_sets.push(set.clone());

            let equips = set.existing_equips_without_decos();
            let mut expr = LinearExpr::empty();
            for equip in &equips {
                if let Some(var) = self.variables.get(&equip.name) {
                    expr.add(*var, 1.0);
                }
            }
            let rhs = (equips.len() as f64) - 1.0;
            self.pending_exclusion = Some((expr, rhs));

            if cancel.map(|c| c.load(Ordering::Relaxed)).unwrap_or(false) {
                break;
            }
            if self.result_sets.len() as i64 >= target {
                break;
            }
        }
        self.solution = current;
        self.all_searched = all_searched;
        Ok(all_searched)
    }

    pub fn is_all_searched(&self) -> bool {
        self.all_searched
    }

    fn build(&mut self) -> SimResult<()> {
        let mut problem = Problem::new(OptimizationDirection::Maximize);
        for equip in &self.all_equips {
            if self.variables.contains_key(&equip.name) {
                return Err(SimError(format!(
                    "装備名 {} が重複しています。",
                    equip.name
                )));
            }
            let score = self.score(equip);
            let max = if equip.kind == EquipKind::Deco {
                if self.condition.has_all_decos {
                    MAX_DECO_COUNT
                } else {
                    equip.deco_count
                }
            } else {
                1
            };
            let var = problem.add_integer_var(score as f64, (0, max.max(0)));
            self.variables.insert(equip.name.clone(), var);
        }

        let mut weapon = LinearExpr::empty();
        let mut weapon_terms = false;
        let mut head = LinearExpr::empty();
        let mut head_terms = false;
        let mut body = LinearExpr::empty();
        let mut body_terms = false;
        let mut arm = LinearExpr::empty();
        let mut arm_terms = false;
        let mut waist = LinearExpr::empty();
        let mut waist_terms = false;
        let mut leg = LinearExpr::empty();
        let mut leg_terms = false;
        let mut charm = LinearExpr::empty();
        let mut charm_terms = false;
        let mut weapon_slots = [
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
        ];
        let mut weapon_slot_terms = [false; 4];
        let mut armor_slots = [
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
        ];
        let mut armor_slot_terms = [false; 4];
        let mut all_slots = [
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
            LinearExpr::empty(),
        ];
        let mut all_slot_terms = [false; 4];
        let mut def = LinearExpr::empty();
        let mut def_terms = false;
        let mut fire = LinearExpr::empty();
        let mut fire_terms = false;
        let mut water = LinearExpr::empty();
        let mut water_terms = false;
        let mut thunder = LinearExpr::empty();
        let mut thunder_terms = false;
        let mut ice = LinearExpr::empty();
        let mut ice_terms = false;
        let mut dragon = LinearExpr::empty();
        let mut dragon_terms = false;
        let mut attack = LinearExpr::empty();
        let mut attack_terms = false;

        let mut skill_exprs: HashMap<String, LinearExpr> = HashMap::new();
        let mut skill_terms: HashMap<String, bool> = HashMap::new();
        for skill in &self.condition.skills {
            skill_exprs.insert(skill.name.clone(), LinearExpr::empty());
            skill_terms.insert(skill.name.clone(), false);
        }
        let mut one_set_exprs: HashMap<i32, LinearExpr> = HashMap::new();

        for equip in &self.all_equips {
            let var = *self.variables.get(&equip.name).unwrap();
            let is_deco = equip.kind == EquipKind::Deco;
            match equip.kind {
                EquipKind::Weapon => {
                    weapon.add(var, 1.0);
                    weapon_terms = true;
                }
                EquipKind::Head => {
                    head.add(var, 1.0);
                    head_terms = true;
                }
                EquipKind::Body => {
                    body.add(var, 1.0);
                    body_terms = true;
                }
                EquipKind::Arm => {
                    arm.add(var, 1.0);
                    arm_terms = true;
                }
                EquipKind::Waist => {
                    waist.add(var, 1.0);
                    waist_terms = true;
                }
                EquipKind::Leg => {
                    leg.add(var, 1.0);
                    leg_terms = true;
                }
                EquipKind::Charm => {
                    charm.add(var, 1.0);
                    charm_terms = true;
                }
                _ => {}
            }

            let mut weapon_slot_cond = self.slot_calc(equip, true, false);
            let mut armor_slot_cond = self.slot_calc(equip, false, true);
            let mut all_slot_cond = self.slot_calc(equip, true, true);
            if is_deco {
                for i in 0..4 {
                    if equip.slot_type1 == 2 {
                        weapon_slot_cond[i] = 0;
                        armor_slot_cond[i] = 0;
                    } else {
                        weapon_slot_cond[i] = -weapon_slot_cond[i];
                        armor_slot_cond[i] = -armor_slot_cond[i];
                    }
                    all_slot_cond[i] = -all_slot_cond[i];
                }
            }
            for i in 0..4 {
                if weapon_slot_cond[i] != 0 {
                    weapon_slots[i].add(var, weapon_slot_cond[i] as f64);
                    weapon_slot_terms[i] = true;
                }
                if armor_slot_cond[i] != 0 {
                    armor_slots[i].add(var, armor_slot_cond[i] as f64);
                    armor_slot_terms[i] = true;
                }
                if all_slot_cond[i] != 0 {
                    all_slots[i].add(var, all_slot_cond[i] as f64);
                    all_slot_terms[i] = true;
                }
            }

            let def_value = if self.condition.is_transcending {
                equip.transcending_def()
            } else {
                equip.maxdef
            };
            if def_value != 0 {
                def.add(var, def_value as f64);
                def_terms = true;
            }
            if equip.fire != 0 {
                fire.add(var, equip.fire as f64);
                fire_terms = true;
            }
            if equip.water != 0 {
                water.add(var, equip.water as f64);
                water_terms = true;
            }
            if equip.thunder != 0 {
                thunder.add(var, equip.thunder as f64);
                thunder_terms = true;
            }
            if equip.ice != 0 {
                ice.add(var, equip.ice as f64);
                ice_terms = true;
            }
            if equip.dragon != 0 {
                dragon.add(var, equip.dragon as f64);
                dragon_terms = true;
            }
            if equip.kind == EquipKind::Weapon && equip.attack != 0 {
                attack.add(var, equip.attack as f64);
                attack_terms = true;
            }

            for cond_skill in &self.condition.skills {
                let mut level = 0;
                for equip_skill in &equip.skills {
                    if equip_skill.name == cond_skill.name {
                        level = equip_skill.level;
                    }
                }
                if level != 0 {
                    if let Some(expr) = skill_exprs.get_mut(&cond_skill.name) {
                        expr.add(var, level as f64);
                        skill_terms.insert(cond_skill.name.clone(), true);
                    }
                }
            }

            if equip.is_one_set {
                one_set_exprs
                    .entry(equip.row_no)
                    .or_insert_with(LinearExpr::empty)
                    .add(
                        var,
                        if equip.kind == EquipKind::Head {
                            4.0
                        } else {
                            -1.0
                        },
                    );
            }
        }

        let mut impossible = false;
        if self.condition.is_specific_weapon {
            if weapon_terms {
                problem.add_constraint(weapon, ComparisonOp::Eq, 1.0);
            } else {
                impossible = true;
            }
        } else if weapon_terms {
            problem.add_constraint(weapon, ComparisonOp::Le, 1.0);
        }
        for (expr, has_terms) in [
            (head, head_terms),
            (body, body_terms),
            (arm, arm_terms),
            (waist, waist_terms),
            (leg, leg_terms),
            (charm, charm_terms),
        ] {
            if has_terms {
                problem.add_constraint(expr, ComparisonOp::Le, 1.0);
            }
        }
        for i in 0..4 {
            if weapon_slot_terms[i] {
                problem.add_constraint(weapon_slots[i].clone(), ComparisonOp::Ge, 0.0);
            }
            if armor_slot_terms[i] {
                problem.add_constraint(armor_slots[i].clone(), ComparisonOp::Ge, 0.0);
            }
            if all_slot_terms[i] {
                problem.add_constraint(all_slots[i].clone(), ComparisonOp::Ge, 0.0);
            }
        }
        let required_def = self.condition.def.unwrap_or(0);
        if def_terms {
            problem.add_constraint(def, ComparisonOp::Ge, required_def as f64);
        } else if required_def > 0 {
            impossible = true;
        }
        for (expr, has_terms, value) in [
            (fire, fire_terms, self.condition.fire),
            (water, water_terms, self.condition.water),
            (thunder, thunder_terms, self.condition.thunder),
            (ice, ice_terms, self.condition.ice),
            (dragon, dragon_terms, self.condition.dragon),
        ] {
            if let Some(value) = value {
                if has_terms {
                    problem.add_constraint(expr, ComparisonOp::Ge, value as f64);
                } else if value > 0 {
                    impossible = true;
                }
            }
        }
        let required_attack = self.condition.min_attack.unwrap_or(0);
        if attack_terms {
            problem.add_constraint(attack, ComparisonOp::Ge, required_attack as f64);
        } else if required_attack > 0 {
            impossible = true;
        }

        for skill in &self.condition.skills {
            let has_terms = *skill_terms.get(&skill.name).unwrap_or(&false);
            let expr = skill_exprs
                .remove(&skill.name)
                .unwrap_or_else(LinearExpr::empty);
            let lower = skill.level as f64;
            let mut upper = f64::INFINITY;
            if skill.is_fixed {
                upper = lower;
                if DISPLAY_RESTRICT_CATEGORIES.contains(&skill.category.as_str()) {
                    upper = f64::INFINITY;
                    if let Some(key) = skill
                        .specific_names
                        .keys()
                        .filter(|key| **key > skill.level)
                        .min()
                    {
                        upper = (*key - 1) as f64;
                    }
                }
            }
            if !has_terms {
                if lower > 0.0 || (upper.is_finite() && upper < 0.0) {
                    impossible = true;
                }
                continue;
            }
            if lower > upper {
                impossible = true;
            }
            problem.add_constraint(expr.clone(), ComparisonOp::Ge, lower);
            if upper.is_finite() {
                problem.add_constraint(expr, ComparisonOp::Le, upper);
            }
        }

        let all_names: HashSet<&str> = self
            .all_equips
            .iter()
            .map(|equip| equip.name.as_str())
            .collect();
        for clude in &self.cludes {
            if !all_names.contains(clude.name.as_str()) {
                continue;
            }
            if let Some(var) = self.variables.get(&clude.name) {
                let fix = if clude.kind == CludeKind::Include {
                    1.0
                } else {
                    0.0
                };
                let mut expr = LinearExpr::empty();
                expr.add(*var, 1.0);
                problem.add_constraint(expr, ComparisonOp::Eq, fix);
            }
        }

        for expr in one_set_exprs.values() {
            problem.add_constraint(expr.clone(), ComparisonOp::Eq, 0.0);
        }

        if impossible {
            let dummy = problem.add_var(0.0, (0.0, 0.0));
            let mut expr = LinearExpr::empty();
            expr.add(dummy, 1.0);
            problem.add_constraint(expr, ComparisonOp::Ge, 1.0);
        }

        self.problem = Some(problem);
        Ok(())
    }

    fn score(&self, equip: &Equipment) -> i32 {
        let (slot1, slot2, slot3) = if equip.kind != EquipKind::Deco {
            (equip.slot1, equip.slot2, equip.slot3)
        } else {
            (-equip.slot1, 0, 0)
        };
        let mut score = if self.condition.is_transcending {
            equip.transcending_def()
        } else {
            equip.maxdef
        };
        score *= 20;
        score += slot1.signum() + slot2.signum() + slot3.signum();
        score *= 80;
        score += slot1 + slot2 + slot3;
        score
    }

    fn slot_calc(&self, equip: &Equipment, count_weapon: bool, count_armor: bool) -> [i32; 4] {
        let (slot1, slot2, slot3) = if self.condition.is_transcending {
            (
                equip.transcending_slot1(),
                equip.transcending_slot2(),
                equip.transcending_slot3(),
            )
        } else {
            (equip.slot1, equip.slot2, equip.slot3)
        };
        let mut cond = [0i32; 4];
        if (count_weapon && equip.slot_type1 != 0) || (count_armor && equip.slot_type1 != 1) {
            for i in 0..slot1 {
                cond[i as usize] += 1;
            }
        }
        if (count_weapon && equip.slot_type2 != 0) || (count_armor && equip.slot_type2 != 1) {
            for i in 0..slot2 {
                cond[i as usize] += 1;
            }
        }
        if (count_weapon && equip.slot_type3 != 0) || (count_armor && equip.slot_type3 != 1) {
            for i in 0..slot3 {
                cond[i as usize] += 1;
            }
        }
        cond
    }

    fn make_set(&self, solution: &Solution) -> Option<EquipSet> {
        let mut set = EquipSet::new();
        if self.condition.is_transcending {
            set.is_transcending = true;
        }
        let mut has_data = false;
        for equip in &self.all_equips {
            let var = match self.variables.get(&equip.name) {
                Some(var) => *var,
                None => continue,
            };
            let value = solution.var_value(var).round() as i32;
            if value > 0 {
                if equip.name.trim().is_empty() {
                    continue;
                }
                has_data = true;
                match equip.kind {
                    EquipKind::Weapon => set.weapon = equip.clone(),
                    EquipKind::Head => set.head = equip.clone(),
                    EquipKind::Body => set.body = equip.clone(),
                    EquipKind::Arm => set.arm = equip.clone(),
                    EquipKind::Waist => set.waist = equip.clone(),
                    EquipKind::Leg => set.leg = equip.clone(),
                    EquipKind::Deco => {
                        for _ in 0..value {
                            set.decos.push(equip.clone());
                        }
                    }
                    EquipKind::Charm => set.charm = equip.clone(),
                    EquipKind::Error => {}
                }
            }
        }
        if !has_data {
            return None;
        }
        set.sort_decos();
        Some(set)
    }

    fn remove_duplicate_set(&mut self, new_set: &EquipSet) {
        self.result_sets.retain(|set| {
            !(is_duplicate_equip_name(&new_set.weapon.name, &set.weapon.name)
                && is_duplicate_equip_name(&new_set.head.name, &set.head.name)
                && is_duplicate_equip_name(&new_set.body.name, &set.body.name)
                && is_duplicate_equip_name(&new_set.arm.name, &set.arm.name)
                && is_duplicate_equip_name(&new_set.waist.name, &set.waist.name)
                && is_duplicate_equip_name(&new_set.leg.name, &set.leg.name)
                && is_duplicate_equip_name(&new_set.charm.name, &set.charm.name))
        });
    }
}

fn is_duplicate_equip_name(new_name: &str, old_name: &str) -> bool {
    new_name.trim().is_empty() || new_name == old_name
}

fn solve_problem(problem: Problem) -> SimResult<Option<Solution>> {
    match problem.solve() {
        Ok(outcome) => match outcome.into_solution() {
            Ok(solution) => Ok(Some(solution)),
            Err(_) => Err(SimError(
                "検索が制限時間内に完了しませんでした。".to_string(),
            )),
        },
        Err(LpError::Infeasible) => Ok(None),
        Err(error) => Err(SimError(format!("検索に失敗しました。: {}", error))),
    }
}
