use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use eframe::egui;
use simmodel::*;

const SEARCH_WEAPON: &str = "指定しない(全武器から検索する)";
const SLOT_ONLY: &str = "武器はスロットのみ指定";
const CALC_WEAPON: &str = "武器も計算に含める";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Skill,
    Result,
    Clude,
    Charm,
    Artian,
    Deco,
    MySet,
    License,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SubTab {
    Recent,
    Extra,
    Condition,
    Special,
}

#[derive(Clone, Default)]
struct SkillChoice {
    level: i32,
    fixed: bool,
}

#[derive(Clone)]
struct SkillSelectorState {
    name: String,
    level: i32,
    levels: Vec<(i32, String)>,
    group_only: bool,
    series_only: bool,
}

enum JobKind {
    Search,
    More,
    Extra,
    Charm,
}

enum JobMessage {
    Search {
        searcher: Box<Searcher>,
        all: bool,
        skills: Vec<Skill>,
        canceled: bool,
    },
    Extra {
        result: Vec<Skill>,
        canceled: bool,
    },
    Charm {
        result: Vec<EquipSet>,
        canceled: bool,
    },
    MoreFailed {
        searcher: Box<Searcher>,
        message: String,
    },
    Failed {
        message: String,
    },
}

struct Job {
    kind: JobKind,
    rx: Receiver<JobMessage>,
    cancel: Arc<AtomicBool>,
    progress: Arc<AtomicUsize>,
    total: usize,
    indeterminate: bool,
}

enum ConfirmAction {
    ResetCondition,
    DeleteCharm(String),
    DeleteArtian(String),
    DeleteMySet(usize),
    DeleteMyCondition(usize),
}

struct ConfirmDialog {
    title: String,
    message: String,
    action: ConfirmAction,
}

enum Action {
    ApplySkill(String, i32, bool),
    ApplyRecent(String, i32),
    SetCalcMode(usize),
    SetWeaponType(String),
    SetWeapon(String),
    SetSlotWeapon(String),
    ToggleBestCharm(bool),
    ToggleBestArtian(bool),
    StartSearch,
    StartExtraSearch,
    StartSearchMore,
    StartCharmSearch,
    AddMySet,
    SelectResult(usize),
    AddExclude(String),
    AddInclude(String),
    DeleteClude(String),
    DeleteAllClude,
    DeleteAllArmorClude,
    DeleteAllWeaponClude,
    AddMyCondition,
    ApplyMyCondition(usize),
    BeginRenameCondition(usize),
    CommitRenameCondition,
    CancelRenameCondition,
    DeleteMyCondition(usize),
    AddCharm,
    DeleteCharm(String),
    MoveCharm(usize, usize),
    SetCharmSkillName(usize, String),
    SetCharmSkillLevel(usize, i32),
    ClearCharmSkill(usize),
    ClearCharmSlot(usize),
    AddArtian,
    DeleteArtian(String),
    MoveArtian(usize, usize),
    SetArtianSkillName(usize, String),
    ClearArtianSkill(usize),
    SetDecoCount(String, i32),
    SetAllDeco(i32),
    SelectMySet(usize),
    DeleteMySet,
    InputMySetCondition,
    ChangeMySetName,
    MoveMySet(usize, usize),
    SetSubTab(SubTab),
    SetCludeTab(usize),
    Confirm(ConfirmAction),
}

pub struct App {
    sim: Arc<Mutex<Simulator>>,
    tab: Tab,
    status: String,
    job: Option<Job>,
    confirm: Option<ConfirmDialog>,
    error: Option<String>,
    skill_choices: HashMap<String, SkillChoice>,
    skill_categories: Vec<(String, Vec<String>)>,
    skill_sub: SubTab,
    calc_mode: usize,
    slot_weapons: Vec<String>,
    slot_weapon: String,
    weapon_type: String,
    weapon_choices: Vec<(String, String)>,
    selected_weapon: String,
    min_attack: String,
    def: String,
    fire: String,
    water: String,
    thunder: String,
    ice: String,
    dragon: String,
    limit: String,
    transcending: bool,
    best_charm: bool,
    best_artian: bool,
    rename_index: Option<usize>,
    rename_input: String,
    extra_skills: Vec<Skill>,
    results: Vec<EquipSet>,
    selected_result: Option<usize>,
    remaining: bool,
    no_result: bool,
    last_limit: i32,
    clude_tab: usize,
    clude_filter_name: String,
    clude_only: bool,
    charm_skills: Vec<SkillSelectorState>,
    charm_slot_sizes: [String; 3],
    charm_slot_types: [String; 3],
    artian_type: String,
    artian_name: String,
    artian_skills: Vec<SkillSelectorState>,
    deco_filter_name: String,
    deco_filter_skill: String,
    my_selected: Option<usize>,
    my_name_input: String,
    data_root: PathBuf,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, sim: Simulator, data_root: PathBuf) -> App {
        install_fonts(&cc.egui_ctx);
        let mut skill_choices = HashMap::new();
        let mut skill_categories: Vec<(String, Vec<String>)> = Vec::new();
        for skill in &sim.masters.skills {
            skill_choices.insert(skill.name.clone(), SkillChoice::default());
            match skill_categories.last_mut() {
                Some((category, names)) if *category == skill.category => {
                    names.push(skill.name.clone())
                }
                _ => skill_categories.push((skill.category.clone(), vec![skill.name.clone()])),
            }
        }
        let weapon_type = WEAPON_TYPES[0].as_str().to_string();
        let slot_weapons = sim
            .masters
            .weapons
            .iter()
            .filter(|w| w.weapon_type == WeaponType::None)
            .map(|w| w.name.clone())
            .collect::<Vec<_>>();
        let slot_weapon = slot_weapons.first().cloned().unwrap_or_default();
        let limit = sim.view.default_limit.clone();
        let default_limit = parse_i32(&limit, 100);
        let my_selected = if sim.masters.my_sets.is_empty() {
            None
        } else {
            Some(0)
        };
        let my_name_input = my_selected
            .and_then(|index| sim.masters.my_sets.get(index))
            .map(|set| set.name.clone())
            .unwrap_or_default();
        let artian_type = WEAPON_TYPES[0].as_str().to_string();
        let charm_skill_count = sim.logic.max_charm_skill_count.max(0) as usize;
        let mut app = App {
            sim: Arc::new(Mutex::new(sim)),
            tab: Tab::Skill,
            status: "モンハンワイルズスキルシミュレータ for Windows".to_string(),
            job: None,
            confirm: None,
            error: None,
            skill_choices,
            skill_categories,
            skill_sub: SubTab::Recent,
            calc_mode: 0,
            slot_weapons,
            slot_weapon,
            weapon_type: weapon_type.clone(),
            weapon_choices: Vec::new(),
            selected_weapon: SEARCH_WEAPON.to_string(),
            min_attack: String::new(),
            def: String::new(),
            fire: String::new(),
            water: String::new(),
            thunder: String::new(),
            ice: String::new(),
            dragon: String::new(),
            limit,
            transcending: true,
            best_charm: false,
            best_artian: false,
            rename_index: None,
            rename_input: String::new(),
            extra_skills: Vec::new(),
            results: Vec::new(),
            selected_result: None,
            remaining: false,
            no_result: false,
            last_limit: default_limit,
            clude_tab: 0,
            clude_filter_name: String::new(),
            clude_only: false,
            charm_skills: (0..charm_skill_count).map(|_| normal_selector()).collect(),
            charm_slot_sizes: ["0".to_string(), "0".to_string(), "0".to_string()],
            charm_slot_types: [
                "防具用".to_string(),
                "防具用".to_string(),
                "防具用".to_string(),
            ],
            artian_type,
            artian_name: String::new(),
            artian_skills: vec![artian_selector(true), artian_selector(false)],
            deco_filter_name: String::new(),
            deco_filter_skill: String::new(),
            my_selected,
            my_name_input,
            data_root,
        };
        app.rebuild_weapon_choices(false);
        app
    }

    fn busy(&self) -> bool {
        self.job.is_some()
    }

    fn poll_job(&mut self) {
        if let Some(job) = self.job.take() {
            match job.rx.try_recv() {
                Ok(message) => self.finish_job(job, message),
                Err(TryRecvError::Empty) => {
                    self.job = Some(job);
                }
                Err(TryRecvError::Disconnected) => {
                    self.status = "エラー: 検索スレッドが異常終了しました".to_string();
                }
            }
        }
    }

    fn finish_job(&mut self, job: Job, message: JobMessage) {
        match message {
            JobMessage::Search {
                searcher,
                all,
                skills,
                canceled,
            } => {
                let is_search = matches!(job.kind, JobKind::Search);
                let is_more = matches!(job.kind, JobKind::More);
                let sets = {
                    let mut sim = self.sim.lock().unwrap();
                    let sets = sim.adopt_searcher(*searcher, all, canceled);
                    if is_search {
                        if let Err(error) = sim.update_recent_skill(&skills) {
                            self.status = format!("エラー: {}", error);
                        }
                    }
                    sets
                };
                match Ok::<_, SimError>(sets) {
                    Ok(sets) => {
                        self.results = sets;
                        self.selected_result = if self.results.is_empty() {
                            None
                        } else {
                            Some(0)
                        };
                        self.remaining = canceled || !all;
                        let best_charm = self.sim.lock().unwrap().is_best_charm_search();
                        self.no_result = !self.remaining && self.results.is_empty() && !best_charm;
                        self.status = if canceled {
                            "処理中断：中断時の検索状態を表示します".to_string()
                        } else if is_more {
                            format!("もっと検索完了：{}件", self.results.len())
                        } else {
                            format!("検索完了：{}件", self.results.len())
                        };
                        self.tab = Tab::Result;
                    }
                    Err(error) => {
                        self.status = format!("エラー: {}", error);
                    }
                }
            }
            JobMessage::Extra { result, canceled } => {
                self.extra_skills = result;
                self.skill_sub = SubTab::Extra;
                self.status = if canceled {
                    "処理中断：中断時の検索状態を表示します".to_string()
                } else {
                    "追加スキル検索完了".to_string()
                };
            }
            JobMessage::Charm { result, canceled } => {
                self.results = result;
                self.selected_result = if self.results.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.remaining = false;
                self.no_result = false;
                self.status = if canceled {
                    "処理中断：中断時の検索状態を表示します".to_string()
                } else {
                    "護石検索完了".to_string()
                };
            }
            JobMessage::MoreFailed { searcher, message } => {
                let sets = {
                    let mut sim = self.sim.lock().unwrap();
                    sim.adopt_searcher(*searcher, false, false)
                };
                self.results = sets;
                self.remaining = true;
                self.no_result = false;
                self.status = format!("エラー: {}（検索状態は保持しました）", message);
                self.error = Some(message);
            }
            JobMessage::Failed { message } => {
                self.status = format!("エラー: {}", message);
                self.error = Some(message);
            }
        }
    }

    fn rebuild_weapon_choices(&mut self, preserve: bool) {
        let previous = self.selected_weapon.clone();
        let sim = self.sim.lock().unwrap();
        let mut choices = vec![(SEARCH_WEAPON.to_string(), SEARCH_WEAPON.to_string())];
        for weapon in sim.masters.weapons.iter().chain(sim.masters.artians.iter()) {
            if weapon.weapon_type.as_str() == self.weapon_type {
                choices.push((weapon.name.clone(), weapon.disp_name().to_string()));
            }
        }
        drop(sim);
        self.weapon_choices = choices;
        if preserve && self.weapon_choices.iter().any(|(v, _)| *v == previous) {
            self.selected_weapon = previous;
        } else {
            self.selected_weapon = self.weapon_choices[0].0.clone();
        }
    }

    fn make_condition(&self) -> SearchCondition {
        let mut condition = SearchCondition::new();
        let sim = self.sim.lock().unwrap();
        for (_, names) in &self.skill_categories {
            for name in names {
                let choice = match self.skill_choices.get(name) {
                    Some(choice) => choice,
                    None => continue,
                };
                if choice.level == 0 && !choice.fixed {
                    continue;
                }
                let mut skill = sim.masters.make_skill(name, choice.level);
                skill.is_fixed = choice.fixed;
                condition.skills.push(skill);
            }
        }
        if self.calc_mode == 0 {
            condition.is_specific_weapon = true;
            condition.weapon_name = Some(self.slot_weapon.clone());
            condition.weapon_type = WeaponType::None;
        } else if self.selected_weapon == SEARCH_WEAPON {
            condition.is_specific_weapon = false;
            condition.weapon_type = WeaponType::from_str(&self.weapon_type).unwrap_or_default();
            condition.min_attack = parse_optional(&self.min_attack);
        } else {
            condition.is_specific_weapon = true;
            condition.weapon_type = WeaponType::from_str(&self.weapon_type).unwrap_or_default();
            condition.weapon_name = Some(self.selected_weapon.clone());
            if let Some(artian) = sim
                .masters
                .artians
                .iter()
                .find(|a| a.name == self.selected_weapon)
            {
                condition.weapon_disp_name = Some(artian.disp_name().to_string());
            }
        }
        condition.def = parse_optional(&self.def);
        condition.fire = parse_optional(&self.fire);
        condition.water = parse_optional(&self.water);
        condition.thunder = parse_optional(&self.thunder);
        condition.ice = parse_optional(&self.ice);
        condition.dragon = parse_optional(&self.dragon);
        condition.is_transcending = self.transcending;
        condition.id = Some(uuid::Uuid::new_v4().to_string());
        condition.disp_name = Some("検索条件".to_string());
        condition.is_best_charm_search = self.best_charm;
        condition.is_best_artian_search = self.best_artian;
        condition
    }

    fn start_search(&mut self) {
        if self.busy() {
            return;
        }
        let condition = self.make_condition();
        let skills = condition.skills.clone();
        let limit = parse_i32(
            &self.limit,
            parse_i32(&self.sim.lock().unwrap().view.default_limit, 100),
        );
        self.last_limit = limit;
        let range = {
            let sim = self.sim.lock().unwrap();
            build_search_range(&condition, &sim.masters)
        };
        let mut searcher = match Searcher::new(condition, range) {
            Ok(searcher) => searcher,
            Err(error) => {
                self.status = format!("エラー: {}", error);
                return;
            }
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let flag = Arc::clone(&cancel);
        std::thread::spawn(move || match searcher.exec_search(limit, Some(&flag)) {
            Ok(all) => {
                let canceled = flag.load(Ordering::Relaxed);
                let _ = tx.send(JobMessage::Search {
                    searcher: Box::new(searcher),
                    all,
                    skills,
                    canceled,
                });
            }
            Err(error) => {
                let _ = tx.send(JobMessage::Failed {
                    message: error.to_string(),
                });
            }
        });
        self.job = Some(Job {
            kind: JobKind::Search,
            rx,
            cancel,
            progress,
            total: 0,
            indeterminate: true,
        });
        self.status = "検索開始・・・".to_string();
    }

    fn start_search_more(&mut self) {
        if self.busy() {
            return;
        }
        let searcher = { self.sim.lock().unwrap().take_searcher() };
        let searcher = match searcher {
            Some(searcher) => searcher,
            None => return,
        };
        let limit = self.last_limit;
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let flag = Arc::clone(&cancel);
        std::thread::spawn(move || {
            let mut searcher = searcher;
            match searcher.exec_search(limit, Some(&flag)) {
                Ok(all) => {
                    let canceled = flag.load(Ordering::Relaxed);
                    let _ = tx.send(JobMessage::Search {
                        searcher: Box::new(searcher),
                        all,
                        skills: Vec::new(),
                        canceled,
                    });
                }
                Err(error) => {
                    let _ = tx.send(JobMessage::MoreFailed {
                        searcher: Box::new(searcher),
                        message: error.to_string(),
                    });
                }
            }
        });
        self.job = Some(Job {
            kind: JobKind::More,
            rx,
            cancel,
            progress,
            total: 0,
            indeterminate: true,
        });
        self.status = "もっと検索開始・・・".to_string();
    }

    fn start_extra_search(&mut self) {
        if self.busy() {
            return;
        }
        let condition = self.make_condition();
        let (masters, logic, total) = {
            let sim = self.sim.lock().unwrap();
            (
                sim.masters.clone(),
                sim.logic.clone(),
                sim.masters.skills.len(),
            )
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let flag = Arc::clone(&cancel);
        let progress_flag = Arc::clone(&progress);
        std::thread::spawn(move || {
            match simmodel::simulator::search_extra_skill_impl(
                &masters,
                &logic,
                &condition,
                &flag,
                Some(&progress_flag),
            ) {
                Ok(result) => {
                    let canceled = flag.load(Ordering::Relaxed);
                    let _ = tx.send(JobMessage::Extra { result, canceled });
                }
                Err(error) => {
                    let _ = tx.send(JobMessage::Failed {
                        message: error.to_string(),
                    });
                }
            }
        });
        self.job = Some(Job {
            kind: JobKind::Extra,
            rx,
            cancel,
            progress,
            total,
            indeterminate: false,
        });
        self.status = "追加スキル検索開始・・・".to_string();
    }

    fn start_charm_search(&mut self) {
        if self.busy() {
            return;
        }
        let (condition, masters, logic, total) = {
            let sim = self.sim.lock().unwrap();
            let condition = match sim.current_condition() {
                Some(condition) => condition,
                None => {
                    self.status = "通常の検索を行う前に護石検索が実行されました。".to_string();
                    return;
                }
            };
            let total = condition
                .make_related_charms(
                    &sim.masters.shining_charm_combos,
                    &sim.masters.shining_charm_groups,
                )
                .len();
            (condition, sim.masters.clone(), sim.logic.clone(), total)
        };
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = Arc::new(AtomicUsize::new(0));
        let (tx, rx) = mpsc::channel();
        let flag = Arc::clone(&cancel);
        let progress_flag = Arc::clone(&progress);
        std::thread::spawn(move || {
            match simmodel::simulator::search_charm_impl(
                &masters,
                &logic,
                &condition,
                &flag,
                Some(&progress_flag),
            ) {
                Ok(result) => {
                    let canceled = flag.load(Ordering::Relaxed);
                    let _ = tx.send(JobMessage::Charm { result, canceled });
                }
                Err(error) => {
                    let _ = tx.send(JobMessage::Failed {
                        message: error.to_string(),
                    });
                }
            }
        });
        self.job = Some(Job {
            kind: JobKind::Charm,
            rx,
            cancel,
            progress,
            total,
            indeterminate: false,
        });
        self.status = "護石検索開始・・・".to_string();
    }

    fn clear_choices(&mut self) {
        for choice in self.skill_choices.values_mut() {
            choice.level = 0;
            choice.fixed = false;
        }
    }

    fn apply_skill(&mut self, name: &str, level: i32, fixed: bool) {
        let valid = {
            let sim = self.sim.lock().unwrap();
            valid_level(&sim.masters, name, level)
        };
        if let Some(choice) = self.skill_choices.get_mut(name) {
            choice.level = valid;
            choice.fixed = fixed;
        }
    }

    fn apply_my_set_condition(&mut self, index: usize) {
        let set = {
            let sim = self.sim.lock().unwrap();
            match sim.masters.my_sets.get(index) {
                Some(set) => set.clone(),
                None => return,
            }
        };
        self.clear_choices();
        for skill in set.skills() {
            self.apply_skill(&skill.name, skill.level, false);
        }
        if set.weapon.weapon_type == WeaponType::None {
            self.calc_mode = 0;
            self.slot_weapon = set.weapon.name.clone();
            self.min_attack.clear();
        } else {
            self.calc_mode = 1;
            self.weapon_type = set.weapon.weapon_type.as_str().to_string();
            self.rebuild_weapon_choices(false);
            self.selected_weapon = set.weapon.name.clone();
            self.min_attack.clear();
        }
        self.transcending = set.is_transcending;
        self.tab = Tab::Skill;
        self.status = format!("マイセット反映完了：{}", set.name);
    }

    fn apply_my_condition(&mut self, index: usize) {
        let condition = {
            let sim = self.sim.lock().unwrap();
            match sim.masters.my_conditions.get(index) {
                Some(condition) => condition.clone(),
                None => return,
            }
        };
        self.clear_choices();
        for skill in &condition.skills {
            self.apply_skill(&skill.name, skill.level, skill.is_fixed);
        }
        if condition.is_specific_weapon {
            let mut weapon_name = condition.weapon_name.clone().unwrap_or_default();
            let mut weapon_type = condition.weapon_type;
            let exists = {
                let sim = self.sim.lock().unwrap();
                sim.masters
                    .all_equipments()
                    .iter()
                    .any(|e| e.name == weapon_name)
            };
            if weapon_name.is_empty() || !exists {
                weapon_type = WeaponType::None;
                weapon_name = "スロットのみ_0-0-0".to_string();
            }
            if weapon_type == WeaponType::None {
                self.calc_mode = 0;
                self.slot_weapon = weapon_name;
                self.min_attack.clear();
            } else {
                self.calc_mode = 1;
                self.weapon_type = weapon_type.as_str().to_string();
                self.rebuild_weapon_choices(false);
                self.selected_weapon = weapon_name;
                self.min_attack.clear();
            }
        } else {
            self.calc_mode = 1;
            self.weapon_type = condition.weapon_type.as_str().to_string();
            self.rebuild_weapon_choices(false);
            self.selected_weapon = SEARCH_WEAPON.to_string();
            self.min_attack = condition
                .min_attack
                .map(|v| v.to_string())
                .unwrap_or_default();
        }
        self.def = condition.def.map(|v| v.to_string()).unwrap_or_default();
        self.fire = condition.fire.map(|v| v.to_string()).unwrap_or_default();
        self.water = condition.water.map(|v| v.to_string()).unwrap_or_default();
        self.thunder = condition.thunder.map(|v| v.to_string()).unwrap_or_default();
        self.ice = condition.ice.map(|v| v.to_string()).unwrap_or_default();
        self.dragon = condition.dragon.map(|v| v.to_string()).unwrap_or_default();
        self.transcending = condition.is_transcending;
        self.status = format!(
            "マイ検索条件反映完了：{}",
            condition.disp_name.unwrap_or_default()
        );
    }

    fn add_my_condition(&mut self) {
        let condition = self.make_condition();
        let mut index = 1;
        let name = loop {
            let candidate = format!("検索条件{}", index);
            let exists = {
                let sim = self.sim.lock().unwrap();
                sim.masters
                    .my_conditions
                    .iter()
                    .any(|c| c.disp_name.as_deref() == Some(candidate.as_str()))
            };
            if !exists {
                break candidate;
            }
            index += 1;
        };
        let mut condition = condition;
        condition.disp_name = Some(name.clone());
        let result = { self.sim.lock().unwrap().add_my_condition(condition) };
        match result {
            Ok(()) => self.status = format!("マイ検索条件登録完了：{}", name),
            Err(error) => self.status = format!("エラー: {}", error),
        }
    }

    fn add_my_set(&mut self) {
        let set = match self
            .selected_result
            .and_then(|index| self.results.get(index))
        {
            Some(set) => set.clone(),
            None => return,
        };
        let result = { self.sim.lock().unwrap().add_my_set(set.clone()) };
        match result {
            Ok(Some(added)) => {
                self.status = format!("マイセット登録完了：{}", added.name);
                let sim = self.sim.lock().unwrap();
                self.my_selected = if sim.masters.my_sets.is_empty() {
                    None
                } else {
                    Some(sim.masters.my_sets.len() - 1)
                };
                self.my_name_input = added.name;
            }
            Ok(None) => self.status = "マイセット登録に失敗しました".to_string(),
            Err(error) => self.status = format!("エラー: {}", error),
        }
    }

    fn apply_action(&mut self, action: Action) {
        match action {
            Action::ApplySkill(name, level, fixed) => self.apply_skill(&name, level, fixed),
            Action::ApplyRecent(name, level) => self.apply_skill(&name, level, false),
            Action::SetCalcMode(mode) => {
                self.calc_mode = mode;
            }
            Action::SetWeaponType(weapon_type) => {
                self.weapon_type = weapon_type;
                self.rebuild_weapon_choices(false);
            }
            Action::SetWeapon(weapon) => self.selected_weapon = weapon,
            Action::SetSlotWeapon(weapon) => self.slot_weapon = weapon,
            Action::ToggleBestCharm(value) => self.best_charm = value,
            Action::ToggleBestArtian(value) => {
                self.best_artian = value;
                if value {
                    self.calc_mode = 1;
                    self.selected_weapon = SEARCH_WEAPON.to_string();
                }
            }
            Action::StartSearch => self.start_search(),
            Action::StartExtraSearch => self.start_extra_search(),
            Action::StartSearchMore => self.start_search_more(),
            Action::StartCharmSearch => self.start_charm_search(),
            Action::AddMySet => self.add_my_set(),
            Action::SelectResult(index) => {
                self.selected_result = Some(index);
            }
            Action::AddExclude(name) => {
                let (result, disp) = {
                    let mut sim = self.sim.lock().unwrap();
                    let disp = sim
                        .masters
                        .get_equip_by_name(&name)
                        .map(|equip| equip.disp_name().to_string())
                        .unwrap_or_else(|| name.clone());
                    (sim.add_exclude(&name), disp)
                };
                self.status = match result {
                    Ok(Some(_)) => format!("除外登録完了：{}", disp),
                    Ok(None) => format!("{}は除外できません", disp),
                    Err(error) => format!("エラー: {}", error),
                };
                let _ = result;
            }
            Action::AddInclude(name) => {
                let (result, disp) = {
                    let mut sim = self.sim.lock().unwrap();
                    let disp = sim
                        .masters
                        .get_equip_by_name(&name)
                        .map(|equip| equip.disp_name().to_string())
                        .unwrap_or_else(|| name.clone());
                    (sim.add_include(&name), disp)
                };
                self.status = match result {
                    Ok(Some(_)) => format!("固定登録完了：{}", disp),
                    Ok(None) => format!("{}は固定できません", disp),
                    Err(error) => format!("エラー: {}", error),
                };
                let _ = result;
            }
            Action::DeleteClude(name) => {
                let result = { self.sim.lock().unwrap().delete_clude(&name, true) };
                self.status = match result {
                    Ok(()) => "除外・固定解除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
            }
            Action::DeleteAllClude => {
                let result = { self.sim.lock().unwrap().delete_all_clude() };
                self.status = match result {
                    Ok(()) => "固定・除外の全解除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
            }
            Action::DeleteAllArmorClude => {
                let result = { self.sim.lock().unwrap().delete_all_armor_clude() };
                self.status = match result {
                    Ok(()) => "固定・除外の全解除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
            }
            Action::DeleteAllWeaponClude => {
                let result = { self.sim.lock().unwrap().delete_all_weapon_clude() };
                self.status = match result {
                    Ok(()) => "固定・除外の全解除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
            }
            Action::AddMyCondition => self.add_my_condition(),
            Action::ApplyMyCondition(index) => self.apply_my_condition(index),
            Action::BeginRenameCondition(index) => {
                let name = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters
                        .my_conditions
                        .get(index)
                        .and_then(|c| c.disp_name.clone())
                        .unwrap_or_default()
                };
                self.rename_index = Some(index);
                self.rename_input = name;
            }
            Action::CommitRenameCondition => {
                if let Some(index) = self.rename_index.take() {
                    let name = self.rename_input.clone();
                    let result = {
                        self.sim
                            .lock()
                            .unwrap()
                            .change_name_of_my_condition(&name, index)
                    };
                    self.status = match result {
                        Ok(()) => "マイ検索条件名前変更完了".to_string(),
                        Err(error) => format!("エラー: {}", error),
                    };
                }
            }
            Action::CancelRenameCondition => {
                self.rename_index = None;
            }
            Action::DeleteMyCondition(index) => {
                self.confirm = Some(ConfirmDialog {
                    title: "マイ検索条件削除".to_string(),
                    message: "マイ検索条件を削除します。\nよろしいですか？".to_string(),
                    action: ConfirmAction::DeleteMyCondition(index),
                });
            }
            Action::AddCharm => self.add_charm(),
            Action::DeleteCharm(name) => self.delete_charm_prompt(&name),
            Action::MoveCharm(from, to) => {
                let result = { self.sim.lock().unwrap().move_charm(from, to) };
                if let Err(error) = result {
                    self.status = format!("エラー: {}", error);
                }
            }
            Action::SetCharmSkillName(index, name) => {
                if let Some(selector) = self.charm_skills.get_mut(index) {
                    set_selector_name(selector, &name, false, &self.sim);
                }
            }
            Action::SetCharmSkillLevel(index, level) => {
                if let Some(selector) = self.charm_skills.get_mut(index) {
                    selector.level = level;
                }
            }
            Action::ClearCharmSkill(index) => {
                if let Some(selector) = self.charm_skills.get_mut(index) {
                    selector.name.clear();
                    selector.level = 0;
                    selector.levels = vec![(0, String::new())];
                }
            }
            Action::ClearCharmSlot(index) => {
                if index < 3 {
                    self.charm_slot_sizes[index] = "0".to_string();
                    self.charm_slot_types[index] = "防具用".to_string();
                }
            }
            Action::AddArtian => {
                self.add_artian();
                self.rebuild_weapon_choices(true);
            }
            Action::DeleteArtian(name) => self.delete_artian_prompt(&name),
            Action::MoveArtian(from, to) => {
                let result = { self.sim.lock().unwrap().move_artian(from, to) };
                if let Err(error) = result {
                    self.status = format!("エラー: {}", error);
                }
            }
            Action::SetArtianSkillName(index, name) => {
                if let Some(selector) = self.artian_skills.get_mut(index) {
                    set_selector_name(selector, &name, true, &self.sim);
                }
            }
            Action::ClearArtianSkill(index) => {
                if let Some(selector) = self.artian_skills.get_mut(index) {
                    selector.name.clear();
                    selector.level = 0;
                    selector.levels = vec![(0, String::new())];
                }
            }
            Action::SetDecoCount(name, count) => {
                let result = { self.sim.lock().unwrap().save_deco_count(&name, count) };
                if let Err(error) = result {
                    self.status = format!("エラー: {}", error);
                }
            }
            Action::SetAllDeco(count) => {
                self.deco_filter_name.clear();
                self.deco_filter_skill.clear();
                let names = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters
                        .decos
                        .iter()
                        .map(|d| d.name.clone())
                        .collect::<Vec<_>>()
                };
                let mut sim = self.sim.lock().unwrap();
                for name in names {
                    if let Err(error) = sim.save_deco_count(&name, count) {
                        self.status = format!("エラー: {}", error);
                        break;
                    }
                }
            }
            Action::SelectMySet(index) => {
                self.my_selected = Some(index);
                let sim = self.sim.lock().unwrap();
                self.my_name_input = sim
                    .masters
                    .my_sets
                    .get(index)
                    .map(|set| set.name.clone())
                    .unwrap_or_default();
            }
            Action::DeleteMySet => {
                if let Some(index) = self.my_selected {
                    let name = {
                        let sim = self.sim.lock().unwrap();
                        sim.masters
                            .my_sets
                            .get(index)
                            .map(|set| set.name.clone())
                            .unwrap_or_default()
                    };
                    self.confirm = Some(ConfirmDialog {
                        title: "マイセット削除".to_string(),
                        message: format!("マイセット「{}」を削除します。\nよろしいですか？", name),
                        action: ConfirmAction::DeleteMySet(index),
                    });
                }
            }
            Action::InputMySetCondition => {
                if let Some(index) = self.my_selected {
                    self.apply_my_set_condition(index);
                }
            }
            Action::ChangeMySetName => {
                if let Some(index) = self.my_selected {
                    let name = self.my_name_input.clone();
                    let result = { self.sim.lock().unwrap().change_name_of_my_set(&name, index) };
                    self.status = match result {
                        Ok(()) => format!("マイセット名前変更完了：{}", name),
                        Err(error) => format!("エラー: {}", error),
                    };
                }
            }
            Action::MoveMySet(from, to) => {
                let result = { self.sim.lock().unwrap().move_my_set(from, to) };
                if let Err(error) = result {
                    self.status = format!("エラー: {}", error);
                } else if self.my_selected == Some(from) {
                    self.my_selected = Some(to);
                }
            }
            Action::SetSubTab(tab) => self.skill_sub = tab,
            Action::SetCludeTab(index) => self.clude_tab = index,
            Action::Confirm(action) => {
                self.confirm = Some(ConfirmDialog {
                    title: self.confirm_title(&action),
                    message: self.confirm_message(&action),
                    action,
                })
            }
        }
    }

    fn confirm_title(&self, action: &ConfirmAction) -> String {
        match action {
            ConfirmAction::ResetCondition => "検索条件リセット".to_string(),
            ConfirmAction::DeleteCharm(_) => "護石削除".to_string(),
            ConfirmAction::DeleteArtian(_) => "アーティア武器削除".to_string(),
            ConfirmAction::DeleteMySet(_) => "マイセット削除".to_string(),
            ConfirmAction::DeleteMyCondition(_) => "マイ検索条件削除".to_string(),
        }
    }

    fn confirm_message(&self, action: &ConfirmAction) -> String {
        match action {
            ConfirmAction::ResetCondition => {
                "選択中の検索条件を全てリセットします。\nよろしいですか？".to_string()
            }
            ConfirmAction::DeleteCharm(name) => {
                let used = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters
                        .my_sets
                        .iter()
                        .any(|set| set.charm.name == *name)
                };
                if used {
                    "この護石を利用しているマイセットが存在します。\n削除すると該当のマイセットも同時に削除されます。\nよろしいですか？".to_string()
                } else {
                    "護石を削除します。\nよろしいですか？".to_string()
                }
            }
            ConfirmAction::DeleteArtian(name) => {
                let used = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters
                        .my_sets
                        .iter()
                        .any(|set| set.weapon.name == *name)
                };
                if used {
                    "このアーティア武器を利用しているマイセットが存在します。\n削除すると該当のマイセットも同時に削除されます。\nよろしいですか？".to_string()
                } else {
                    "アーティア武器を削除します。\nよろしいですか？".to_string()
                }
            }
            ConfirmAction::DeleteMySet(_) => {
                "マイセットを削除します。\nよろしいですか？".to_string()
            }
            ConfirmAction::DeleteMyCondition(_) => {
                "マイ検索条件を削除します。\nよろしいですか？".to_string()
            }
        }
    }

    fn delete_charm_prompt(&mut self, name: &str) {
        self.confirm = Some(ConfirmDialog {
            title: "護石削除".to_string(),
            message: self.confirm_message(&ConfirmAction::DeleteCharm(name.to_string())),
            action: ConfirmAction::DeleteCharm(name.to_string()),
        });
    }

    fn delete_artian_prompt(&mut self, name: &str) {
        self.confirm = Some(ConfirmDialog {
            title: "アーティア武器削除".to_string(),
            message: self.confirm_message(&ConfirmAction::DeleteArtian(name.to_string())),
            action: ConfirmAction::DeleteArtian(name.to_string()),
        });
    }

    fn execute_confirm(&mut self, action: ConfirmAction) {
        match action {
            ConfirmAction::ResetCondition => {
                self.clear_choices();
                self.calc_mode = 0;
                self.weapon_type = WEAPON_TYPES[0].as_str().to_string();
                self.rebuild_weapon_choices(false);
                self.min_attack.clear();
                self.def.clear();
                self.fire.clear();
                self.water.clear();
                self.thunder.clear();
                self.ice.clear();
                self.dragon.clear();
            }
            ConfirmAction::DeleteCharm(name) => {
                let result = { self.sim.lock().unwrap().delete_charm(&name) };
                self.status = match result {
                    Ok(()) => "護石削除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
                self.my_selected = {
                    let sim = self.sim.lock().unwrap();
                    if sim.masters.my_sets.is_empty() {
                        None
                    } else {
                        Some(0)
                    }
                };
            }
            ConfirmAction::DeleteArtian(name) => {
                let result = { self.sim.lock().unwrap().delete_artian(&name) };
                self.status = match result {
                    Ok(()) => "アーティア削除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
                self.my_selected = {
                    let sim = self.sim.lock().unwrap();
                    if sim.masters.my_sets.is_empty() {
                        None
                    } else {
                        Some(0)
                    }
                };
                self.rebuild_weapon_choices(true);
            }
            ConfirmAction::DeleteMySet(index) => {
                let result = { self.sim.lock().unwrap().delete_my_set_at(index) };
                self.status = match result {
                    Ok(()) => "マイセット削除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
                self.my_selected = {
                    let sim = self.sim.lock().unwrap();
                    if sim.masters.my_sets.is_empty() {
                        None
                    } else {
                        Some(0)
                    }
                };
            }
            ConfirmAction::DeleteMyCondition(index) => {
                let result = { self.sim.lock().unwrap().delete_my_condition_at(index) };
                self.status = match result {
                    Ok(()) => "マイ検索条件削除完了".to_string(),
                    Err(error) => format!("エラー: {}", error),
                };
            }
        }
        if let Some(index) = self.my_selected {
            let sim = self.sim.lock().unwrap();
            if let Some(set) = sim.masters.my_sets.get(index) {
                self.my_name_input = set.name.clone();
            }
        }
    }

    fn add_charm(&mut self) {
        let mut charm = Equipment::new(EquipKind::Charm);
        charm.name = uuid::Uuid::new_v4().to_string();
        for selector in &self.charm_skills {
            if selector.name.is_empty() || selector.level <= 0 {
                continue;
            }
            let exists = {
                let sim = self.sim.lock().unwrap();
                sim.masters.is_skill_name(&selector.name)
            };
            if exists {
                let skill = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters.make_skill(&selector.name, selector.level)
                };
                charm.skills.push(skill);
            }
        }
        charm.slot1 = parse_i32(&self.charm_slot_sizes[0], 0);
        charm.slot2 = parse_i32(&self.charm_slot_sizes[1], 0);
        charm.slot3 = parse_i32(&self.charm_slot_sizes[2], 0);
        charm.slot_type1 = if self.charm_slot_types[0] == "武器用" {
            1
        } else {
            0
        };
        charm.slot_type2 = if self.charm_slot_types[1] == "武器用" {
            1
        } else {
            0
        };
        charm.slot_type3 = if self.charm_slot_types[2] == "武器用" {
            1
        } else {
            0
        };
        charm.set_charm_disp_name();
        let disp = charm.disp_name().to_string();
        let result = { self.sim.lock().unwrap().add_charm(charm) };
        self.status = match result {
            Ok(()) => format!("護石追加完了：{}", disp),
            Err(error) => format!("エラー: {}", error),
        };
    }

    fn add_artian(&mut self) {
        let mut artian = Equipment::new_weapon();
        artian.init_artian();
        artian.name = uuid::Uuid::new_v4().to_string();
        artian.weapon_type = WeaponType::from_str(&self.artian_type).unwrap_or_default();
        let disp_name = if self.artian_name.trim().is_empty() {
            let sim = self.sim.lock().unwrap();
            let mut index = 1;
            loop {
                let candidate = format!("アーティア{}{}", artian.weapon_type, index);
                if !sim
                    .masters
                    .artians
                    .iter()
                    .any(|a| a.disp_name() == candidate)
                {
                    break candidate;
                }
                index += 1;
            }
        } else {
            self.artian_name.clone()
        };
        artian.disp_name = Some(disp_name.clone());
        for selector in &self.artian_skills {
            if selector.name.is_empty() {
                continue;
            }
            let exists = {
                let sim = self.sim.lock().unwrap();
                sim.masters.skills.iter().any(|s| s.name == selector.name)
            };
            if exists {
                let skill = {
                    let sim = self.sim.lock().unwrap();
                    sim.masters.make_skill(&selector.name, selector.level)
                };
                artian.skills.push(skill);
            }
        }
        let result = { self.sim.lock().unwrap().add_artian(artian) };
        self.status = match result {
            Ok(()) => format!("アーティア追加完了：{}", disp_name),
            Err(error) => format!("エラー: {}", error),
        };
    }

    fn draw_tabs(&mut self, ui: &mut egui::Ui) {
        let tabs = [
            (Tab::Skill, "スキル選択"),
            (Tab::Result, "検索結果"),
            (Tab::Clude, "除外・固定設定"),
            (Tab::Charm, "護石設定"),
            (Tab::Artian, "アーティア設定"),
            (Tab::Deco, "装飾品設定"),
            (Tab::MySet, "マイセット"),
            (Tab::License, "ライセンス・謝辞"),
        ];
        ui.horizontal(|ui| {
            for (tab, name) in tabs {
                if ui
                    .selectable_label(self.tab == tab, egui::RichText::new(name).size(16.0))
                    .clicked()
                {
                    self.tab = tab;
                }
            }
        });
    }

    fn draw_status(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.label(&self.status);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let fraction = self
                    .job
                    .as_ref()
                    .map(|job| {
                        if job.total == 0 {
                            0.0
                        } else {
                            (job.progress.load(Ordering::Relaxed) as f32 / job.total as f32)
                                .clamp(0.0, 1.0)
                        }
                    })
                    .unwrap_or(0.0);
                if let Some(job) = &self.job {
                    if job.indeterminate {
                        ui.spinner();
                    } else {
                        ui.add(
                            egui::ProgressBar::new(fraction)
                                .desired_width(180.0)
                                .show_percentage(),
                        );
                    }
                    if ui.button("処理中断").clicked() {
                        job.cancel.store(true, Ordering::Relaxed);
                    }
                } else {
                    ui.add_space(220.0);
                }
            });
        });
    }

    fn draw_skill_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        egui::Panel::right("skill_side")
            .resizable(false)
            .default_size(300.0)
            .show(ui, |ui| {
                self.draw_skill_side(ui, &mut actions);
            });
        egui::Panel::bottom("skill_controls").show(ui, |ui| {
            self.draw_skill_controls(ui, &mut actions);
        });
        egui::ScrollArea::vertical()
            .id_salt("skill_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let sim = self.sim.lock().unwrap();
                let masters = &sim.masters;
                for (category, names) in &self.skill_categories {
                    egui::CollapsingHeader::new(category)
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for name in names {
                                    if !masters.skills.iter().any(|s| s.name == *name) {
                                        continue;
                                    }
                                    let choice =
                                        self.skill_choices.entry(name.clone()).or_default();
                                    let selected = choice.level != 0 || choice.fixed;
                                    if selected {
                                        ui.colored_label(egui::Color32::from_rgb(255, 140, 0), "●");
                                    } else {
                                        ui.label(" ");
                                    }
                                    let levels = skill_level_items(masters, name);
                                    let current = levels
                                        .iter()
                                        .find(|(level, _)| *level == choice.level)
                                        .map(|(_, label)| label.clone())
                                        .unwrap_or_else(|| format!("{}Lv{}", name, choice.level));
                                    egui::ComboBox::from_id_salt(("skilllevel", name))
                                        .width(210.0)
                                        .selected_text(current)
                                        .show_ui(ui, |ui| {
                                            for (level, label) in &levels {
                                                if ui
                                                    .selectable_label(choice.level == *level, label)
                                                    .clicked()
                                                {
                                                    actions.push(Action::ApplySkill(
                                                        name.clone(),
                                                        *level,
                                                        choice.fixed,
                                                    ));
                                                }
                                            }
                                        });
                                    let fix_text = if choice.fixed { "固定" } else { "以上" };
                                    egui::ComboBox::from_id_salt(("skillfix", name))
                                        .width(55.0)
                                        .selected_text(fix_text)
                                        .show_ui(ui, |ui| {
                                            if ui.selectable_label(!choice.fixed, "以上").clicked()
                                            {
                                                actions.push(Action::ApplySkill(
                                                    name.clone(),
                                                    choice.level,
                                                    false,
                                                ));
                                            }
                                            if ui.selectable_label(choice.fixed, "固定").clicked()
                                            {
                                                actions.push(Action::ApplySkill(
                                                    name.clone(),
                                                    choice.level,
                                                    true,
                                                ));
                                            }
                                        });
                                }
                            });
                        });
                }
            });
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_skill_controls(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.separator();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("武器設定");
                    egui::ComboBox::from_id_salt("calcweapon")
                        .width(210.0)
                        .selected_text(if self.calc_mode == 0 {
                            SLOT_ONLY
                        } else {
                            CALC_WEAPON
                        })
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_label(self.calc_mode == 0, SLOT_ONLY)
                                .clicked()
                            {
                                actions.push(Action::SetCalcMode(0));
                            }
                            if ui
                                .selectable_label(self.calc_mode == 1, CALC_WEAPON)
                                .clicked()
                            {
                                actions.push(Action::SetCalcMode(1));
                            }
                        });
                });
                if self.calc_mode == 0 {
                    ui.horizontal(|ui| {
                        ui.label("武器選択");
                        egui::ComboBox::from_id_salt("slotweapon")
                            .width(210.0)
                            .selected_text(&self.slot_weapon)
                            .show_ui(ui, |ui| {
                                for weapon in &self.slot_weapons {
                                    if ui
                                        .selectable_label(&self.slot_weapon == weapon, weapon)
                                        .clicked()
                                    {
                                        actions.push(Action::SetSlotWeapon(weapon.clone()));
                                    }
                                }
                            });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.label("武器種選択");
                        egui::ComboBox::from_id_salt("weapontype")
                            .width(210.0)
                            .selected_text(&self.weapon_type)
                            .show_ui(ui, |ui| {
                                for weapon_type in WeaponType::names_without_none() {
                                    if ui
                                        .selectable_label(
                                            self.weapon_type == weapon_type,
                                            weapon_type,
                                        )
                                        .clicked()
                                    {
                                        actions
                                            .push(Action::SetWeaponType(weapon_type.to_string()));
                                    }
                                }
                            });
                    });
                    ui.horizontal(|ui| {
                        ui.label("武器指定");
                        let display = self
                            .weapon_choices
                            .iter()
                            .find(|(value, _)| *value == self.selected_weapon)
                            .map(|(_, disp)| disp.clone())
                            .unwrap_or_else(|| self.selected_weapon.clone());
                        egui::ComboBox::from_id_salt("weapon")
                            .width(210.0)
                            .selected_text(display)
                            .show_ui(ui, |ui| {
                                for (value, disp) in &self.weapon_choices {
                                    if ui
                                        .selectable_label(self.selected_weapon == *value, disp)
                                        .clicked()
                                    {
                                        actions.push(Action::SetWeapon(value.clone()));
                                    }
                                }
                            });
                    });
                    if self.selected_weapon == SEARCH_WEAPON {
                        ui.horizontal(|ui| {
                            ui.label("最低攻撃力");
                            ui.text_edit_singleline(&mut self.min_attack);
                        });
                    }
                }
            });
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label("防御力");
                    ui.add_sized([50.0, 20.0], egui::TextEdit::singleline(&mut self.def));
                });
                ui.horizontal(|ui| {
                    for (label, value) in [
                        ("火", &mut self.fire),
                        ("水", &mut self.water),
                        ("雷", &mut self.thunder),
                        ("氷", &mut self.ice),
                        ("龍", &mut self.dragon),
                    ] {
                        ui.label(label);
                        ui.add_sized([35.0, 20.0], egui::TextEdit::singleline(value));
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("頑張り度");
                    ui.add_sized([100.0, 20.0], egui::TextEdit::singleline(&mut self.limit));
                });
                ui.checkbox(&mut self.transcending, "限界突破強化を使用する");
            });
            ui.vertical(|ui| {
                if ui.button("検索する").clicked() {
                    actions.push(Action::StartSearch);
                }
                if ui.button("追加スキル検索する").clicked() {
                    actions.push(Action::StartExtraSearch);
                }
                if ui.button("検索条件をリセット").clicked() {
                    actions.push(Action::Confirm(ConfirmAction::ResetCondition));
                }
            });
        });
    }

    fn draw_skill_side(&mut self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.horizontal_wrapped(|ui| {
            for (tab, name) in [
                (SubTab::Recent, "最近使ったスキル"),
                (SubTab::Extra, "追加スキル検索結果"),
                (SubTab::Condition, "マイ検索条件"),
                (SubTab::Special, "特殊検索オプション"),
            ] {
                if ui.selectable_label(self.skill_sub == tab, name).clicked() {
                    actions.push(Action::SetSubTab(tab));
                }
            }
        });
        ui.separator();
        egui::ScrollArea::vertical()
            .id_salt("skill_side_scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| match self.skill_sub {
                SubTab::Recent => {
                    let names = {
                        let sim = self.sim.lock().unwrap();
                        sim.masters.recent_skill_names.clone()
                    };
                    let sim = self.sim.lock().unwrap();
                    for name in names {
                        let master = match sim.masters.skills.iter().find(|s| s.name == name) {
                            Some(master) => master,
                            None => continue,
                        };
                        ui.label(&master.name);
                        ui.horizontal_wrapped(|ui| {
                            for level in 1..=master.level {
                                if master.is_hide_level(Some(level)) {
                                    continue;
                                }
                                if ui.small_button(level.to_string()).clicked() {
                                    actions.push(Action::ApplyRecent(name.clone(), level));
                                }
                            }
                        });
                    }
                }
                SubTab::Extra => {
                    let mut grouped: Vec<(String, Vec<i32>)> = Vec::new();
                    for skill in &self.extra_skills {
                        match grouped.last_mut() {
                            Some((name, levels)) if *name == skill.name => {
                                levels.push(skill.level)
                            }
                            _ => grouped.push((skill.name.clone(), vec![skill.level])),
                        }
                    }
                    let sim = self.sim.lock().unwrap();
                    for (name, levels) in grouped {
                        ui.label(&name);
                        ui.horizontal_wrapped(|ui| {
                            for level in levels {
                                let hidden = sim
                                    .masters
                                    .skills
                                    .iter()
                                    .find(|s| s.name == name)
                                    .map(|s| s.is_hide_level(Some(level)))
                                    .unwrap_or(false);
                                if hidden {
                                    continue;
                                }
                                if ui.small_button(level.to_string()).clicked() {
                                    actions.push(Action::ApplyRecent(name.clone(), level));
                                }
                            }
                        });
                    }
                }
                SubTab::Condition => {
                    if ui.button("現在の検索条件を保存する").clicked() {
                        actions.push(Action::AddMyCondition);
                    }
                    let conditions = {
                        let sim = self.sim.lock().unwrap();
                        sim.masters.my_conditions.clone()
                    };
                    for (index, condition) in conditions.iter().enumerate() {
                        if self.rename_index == Some(index) {
                            ui.horizontal(|ui| {
                                ui.add_sized(
                                    [120.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.rename_input),
                                );
                                if ui.button("確定").clicked() {
                                    actions.push(Action::CommitRenameCondition);
                                }
                                if ui.button("キャンセル").clicked() {
                                    actions.push(Action::CancelRenameCondition);
                                }
                            });
                        } else {
                            ui.label(condition.disp_name.clone().unwrap_or_default());
                        }
                        ui.horizontal(|ui| {
                            if ui.button("適用").clicked() {
                                actions.push(Action::ApplyMyCondition(index));
                            }
                            if ui.button("名前変更").clicked() {
                                actions.push(Action::BeginRenameCondition(index));
                            }
                            if ui.button("削除").clicked() {
                                actions.push(Action::DeleteMyCondition(index));
                            }
                        });
                        egui::CollapsingHeader::new("詳細")
                            .default_open(false)
                            .show(ui, |ui| {
                                ui.label(condition.description());
                            });
                        ui.separator();
                    }
                }
                SubTab::Special => {
                    let mut charm = self.best_charm;
                    if ui
                        .checkbox(&mut charm, "理論値護石検索(光おま)")
                        .changed()
                    {
                        actions.push(Action::ToggleBestCharm(charm));
                    }
                    ui.label(
                        "出現し得る護石を全て所持している仮定で検索を行います。\nこの機能で検索結果に出る理論値護石は、除外・固定・マイセット登録ができません。",
                    );
                    ui.separator();
                    let mut artian = self.best_artian;
                    if ui.checkbox(&mut artian, "理論値アーティア検索").changed() {
                        actions.push(Action::ToggleBestArtian(artian));
                    }
                    ui.label(
                        "出現し得るアーティア武器を全て所持している仮定で検索を行います。\n通常の検索条件を優先するため、「武器設定」が「武器を計算に含める」、「武器指定」が「指定しない(全武器から検索する)」の場合にのみ有効です。",
                    );
                }
            });
    }

    fn draw_result_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        egui::Panel::right("result_detail")
            .resizable(false)
            .default_size(250.0)
            .show(ui, |ui| {
                if ui.button("この装備をマイセットに登録する").clicked() {
                    actions.push(Action::AddMySet);
                }
                let set = self
                    .selected_result
                    .and_then(|index| self.results.get(index))
                    .cloned();
                if let Some(set) = set {
                    for equip in set.equipments() {
                        ui.horizontal(|ui| {
                            ui.add_sized(
                                [40.0, 18.0],
                                egui::Label::new(equip.kind.str_with_colon()),
                            );
                            ui.add_sized(
                                [95.0, 18.0],
                                egui::Label::new(equip.disp_name()).truncate(),
                            );
                            if ui.small_button("除外").clicked() {
                                actions.push(Action::AddExclude(equip.name.clone()));
                            }
                            let can_include =
                                equip.kind != EquipKind::Deco && equip.kind != EquipKind::Weapon;
                            if ui
                                .add_enabled(can_include, egui::Button::new("固定").small())
                                .clicked()
                            {
                                actions.push(Action::AddInclude(equip.name.clone()));
                            }
                        });
                    }
                    ui.separator();
                    let description = set.description();
                    egui::ScrollArea::vertical()
                        .id_salt("result_description")
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(description).wrap());
                        });
                }
            });
        egui::Panel::bottom("result_buttons").show(ui, |ui| {
            ui.horizontal(|ui| {
                if self.remaining && ui.button("もっと検索する").clicked() {
                    actions.push(Action::StartSearchMore);
                }
                if self.no_result && ui.button("護石検索").clicked() {
                    actions.push(Action::StartCharmSearch);
                }
                if self.remaining {
                    ui.label("結果が多い、または中断を行ったため検索を打ち切りました。");
                } else if self.no_result {
                    ui.label("検索結果は0件でした。どんな護石があれば実現できるか検索できます。");
                }
            });
        });
        let row_height = 20.0;
        egui::ScrollArea::horizontal()
            .id_salt("result_h")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.add_sized([55.0, row_height], egui::Label::new("防御"));
                        for header in [
                            "武器",
                            "頭",
                            "胴",
                            "腕",
                            "腰",
                            "足",
                            "護石",
                            "火",
                            "水",
                            "雷",
                            "氷",
                            "龍",
                            "装飾品",
                            "発動スキル",
                        ] {
                            ui.add_sized([125.0, row_height], egui::Label::new(header));
                        }
                    });
                    egui::ScrollArea::vertical()
                        .id_salt("result_v")
                        .auto_shrink([false, false])
                        .show_rows(ui, row_height, self.results.len(), |ui, range| {
                            for index in range {
                                let set = &self.results[index];
                                let selected = self.selected_result == Some(index);
                                let response = ui.horizontal(|ui| {
                                    if ui
                                        .add_sized(
                                            [55.0, row_height],
                                            egui::Button::selectable(
                                                selected,
                                                set.maxdef().to_string(),
                                            ),
                                        )
                                        .clicked()
                                    {
                                        actions.push(Action::SelectResult(index));
                                    }
                                    for text in result_columns(set) {
                                        ui.add_sized(
                                            [125.0, row_height],
                                            egui::Label::new(text).truncate(),
                                        );
                                    }
                                });
                                if response.response.interact(egui::Sense::click()).clicked() {
                                    actions.push(Action::SelectResult(index));
                                }
                            }
                        });
                });
            });
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_clude_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("フィルタ");
                ui.horizontal(|ui| {
                    ui.label("名前：");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.clude_filter_name),
                    );
                });
                ui.checkbox(
                    &mut self.clude_only,
                    "除外・固定が設定されている装備のみに絞る",
                );
                if ui.button("クリア").clicked() {
                    self.clude_filter_name.clear();
                    self.clude_only = false;
                }
            });
            ui.vertical(|ui| {
                ui.label("特殊操作(フィルタは解除されます)");
                if ui.button("除外・固定を全て解除").clicked() {
                    actions.push(Action::DeleteAllClude);
                }
                if ui.button("防具の除外・固定を全て解除").clicked() {
                    actions.push(Action::DeleteAllArmorClude);
                }
                if ui.button("武器の除外・固定を全て解除").clicked() {
                    actions.push(Action::DeleteAllWeaponClude);
                }
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            if ui.selectable_label(self.clude_tab == 0, "防具").clicked() {
                actions.push(Action::SetCludeTab(0));
            }
            if ui.selectable_label(self.clude_tab == 1, "武器").clicked() {
                actions.push(Action::SetCludeTab(1));
            }
        });
        let sim = self.sim.lock().unwrap();
        let cludes: HashMap<String, CludeKind> = sim
            .masters
            .cludes
            .iter()
            .map(|clude| (clude.name.clone(), clude.kind))
            .collect();
        if self.clude_tab == 0 {
            let rows = build_armor_rows(
                &sim.masters,
                &self.clude_filter_name,
                self.clude_only,
                &cludes,
            );
            egui::ScrollArea::both()
                .id_salt("clude_rows")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for header in ["頭", "胴", "腕", "腰", "足", "護石", "装飾品"] {
                            ui.add_sized(
                                [145.0, 16.0],
                                egui::Label::new(egui::RichText::new(header).strong()),
                            );
                        }
                    });
                    for row in &rows {
                        ui.horizontal(|ui| {
                            for cell in &row.cells {
                                clude_cell(ui, cell, &cludes, &mut actions);
                            }
                        });
                    }
                });
        } else {
            let rows = build_weapon_rows(
                &sim.masters,
                &self.clude_filter_name,
                self.clude_only,
                &cludes,
            );
            egui::ScrollArea::both()
                .id_salt("clude_rows")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for weapon_type in WEAPON_TYPES {
                            ui.add_sized(
                                [145.0, 16.0],
                                egui::Label::new(
                                    egui::RichText::new(weapon_type.as_str()).strong(),
                                ),
                            );
                        }
                    });
                    for row in &rows {
                        ui.horizontal(|ui| {
                            for cell in &row.cells {
                                clude_cell(ui, cell, &cludes, &mut actions);
                            }
                        });
                    }
                });
        }
        drop(sim);
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_charm_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        egui::Panel::left("charm_form")
            .resizable(false)
            .default_size(280.0)
            .show(ui, |ui| {
                for index in 0..self.charm_skills.len() {
                    draw_skill_selector(
                        ui,
                        &mut self.charm_skills[index],
                        &self.sim,
                        &mut actions,
                        index,
                        true,
                    );
                }
                for index in 0..3 {
                    ui.horizontal(|ui| {
                        ui.label(format!("スロット{}", index + 1));
                        egui::ComboBox::from_id_salt(("charmsize", index))
                            .width(40.0)
                            .selected_text(&self.charm_slot_sizes[index])
                            .show_ui(ui, |ui| {
                                let max = self.sim.lock().unwrap().logic.max_slot_size;
                                for size in 0..=max {
                                    let text = size.to_string();
                                    if ui
                                        .selectable_label(
                                            self.charm_slot_sizes[index] == text,
                                            &text,
                                        )
                                        .clicked()
                                    {
                                        self.charm_slot_sizes[index] = text.clone();
                                    }
                                }
                            });
                        egui::ComboBox::from_id_salt(("charmtype", index))
                            .width(70.0)
                            .selected_text(&self.charm_slot_types[index])
                            .show_ui(ui, |ui| {
                                for slot_type in ["防具用", "武器用"] {
                                    if ui
                                        .selectable_label(
                                            self.charm_slot_types[index] == slot_type,
                                            slot_type,
                                        )
                                        .clicked()
                                    {
                                        self.charm_slot_types[index] = slot_type.to_string();
                                    }
                                }
                            });
                        if ui.small_button("クリア").clicked() {
                            actions.push(Action::ClearCharmSlot(index));
                        }
                    });
                }
                if ui.button("登録する").clicked() {
                    actions.push(Action::AddCharm);
                }
            });
        let sim = self.sim.lock().unwrap();
        let charms = sim.masters.additional_charms.clone();
        egui::ScrollArea::vertical()
            .id_salt("charm_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("charmgrid").striped(true).show(ui, |ui| {
                    ui.strong("詳細");
                    ui.strong("操作");
                    ui.strong("");
                    ui.end_row();
                    for (index, charm) in charms.iter().enumerate() {
                        ui.label(charm.disp_name());
                        ui.horizontal(|ui| {
                            if ui.small_button("削除").clicked() {
                                actions.push(Action::DeleteCharm(charm.name.clone()));
                            }
                            if index > 0 && ui.small_button("上へ").clicked() {
                                actions.push(Action::MoveCharm(index, index - 1));
                            }
                            if index + 1 < charms.len() && ui.small_button("下へ").clicked() {
                                actions.push(Action::MoveCharm(index, index + 1));
                            }
                        });
                        ui.label(charm_upper_text(charm));
                        ui.end_row();
                    }
                });
            });
        drop(sim);
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_artian_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        egui::Panel::left("artian_form")
            .resizable(false)
            .default_size(280.0)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label("武器種：");
                    egui::ComboBox::from_id_salt("artiantype")
                        .width(120.0)
                        .selected_text(&self.artian_type)
                        .show_ui(ui, |ui| {
                            for weapon_type in WeaponType::names_without_none() {
                                if ui
                                    .selectable_label(self.artian_type == weapon_type, weapon_type)
                                    .clicked()
                                {
                                    self.artian_type = weapon_type.to_string();
                                }
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("名前：");
                    ui.add_sized(
                        [120.0, 20.0],
                        egui::TextEdit::singleline(&mut self.artian_name),
                    );
                });
                for index in 0..self.artian_skills.len() {
                    draw_skill_selector(
                        ui,
                        &mut self.artian_skills[index],
                        &self.sim,
                        &mut actions,
                        index,
                        false,
                    );
                }
                if ui.button("登録する").clicked() {
                    actions.push(Action::AddArtian);
                }
            });
        let sim = self.sim.lock().unwrap();
        let artians = sim.masters.artians.clone();
        egui::ScrollArea::vertical()
            .id_salt("artian_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("artiangrid").striped(true).show(ui, |ui| {
                    ui.strong("武器種");
                    ui.strong("名前");
                    ui.strong("スキル");
                    ui.strong("操作");
                    ui.end_row();
                    for (index, artian) in artians.iter().enumerate() {
                        ui.label(artian.weapon_type.to_string());
                        ui.label(artian.disp_name());
                        ui.label(
                            artian
                                .skills
                                .iter()
                                .map(|s| s.name.clone())
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                        ui.horizontal(|ui| {
                            if ui.small_button("削除").clicked() {
                                actions.push(Action::DeleteArtian(artian.name.clone()));
                            }
                            if index > 0 && ui.small_button("上へ").clicked() {
                                actions.push(Action::MoveArtian(index, index - 1));
                            }
                            if index + 1 < artians.len() && ui.small_button("下へ").clicked() {
                                actions.push(Action::MoveArtian(index, index + 1));
                            }
                        });
                        ui.end_row();
                    }
                });
            });
        drop(sim);
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_deco_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.label("フィルタ");
                ui.horizontal(|ui| {
                    ui.label("装飾品名：");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.deco_filter_name),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("スキル名：");
                    ui.add_sized(
                        [150.0, 20.0],
                        egui::TextEdit::singleline(&mut self.deco_filter_skill),
                    );
                });
                if ui.button("クリア").clicked() {
                    self.deco_filter_name.clear();
                    self.deco_filter_skill.clear();
                }
            });
            ui.vertical(|ui| {
                ui.label("特殊操作(フィルタは解除されます)");
                if ui.button("全装飾品を0に").clicked() {
                    actions.push(Action::SetAllDeco(0));
                }
                if ui.button("全装飾品を7に").clicked() {
                    actions.push(Action::SetAllDeco(7));
                }
            });
        });
        ui.separator();
        let sim = self.sim.lock().unwrap();
        let mut groups: Vec<(String, Vec<Equipment>)> = Vec::new();
        for deco in &sim.masters.decos {
            if !self.deco_filter_name.is_empty()
                && !deco.disp_name().contains(&self.deco_filter_name)
            {
                continue;
            }
            if !self.deco_filter_skill.is_empty()
                && !deco
                    .skills
                    .iter()
                    .any(|s| s.name.contains(&self.deco_filter_skill))
            {
                continue;
            }
            let category = deco.deco_category();
            match groups.last_mut() {
                Some((name, list)) if *name == category => list.push(deco.clone()),
                _ => groups.push((category, vec![deco.clone()])),
            }
        }
        egui::ScrollArea::vertical()
            .id_salt("deco_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for (category, decos) in groups {
                    egui::CollapsingHeader::new(category)
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                for deco in decos {
                                    ui.label(deco.disp_name());
                                    egui::ComboBox::from_id_salt(("decocount", deco.name.clone()))
                                        .width(40.0)
                                        .selected_text(deco.deco_count.to_string())
                                        .show_ui(ui, |ui| {
                                            for count in 0..=7 {
                                                if ui
                                                    .selectable_label(
                                                        deco.deco_count == count,
                                                        count.to_string(),
                                                    )
                                                    .clicked()
                                                {
                                                    actions.push(Action::SetDecoCount(
                                                        deco.name.clone(),
                                                        count,
                                                    ));
                                                }
                                            }
                                        });
                                }
                            });
                        });
                }
            });
        drop(sim);
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_myset_tab(&mut self, ui: &mut egui::Ui) {
        let mut actions = Vec::new();
        egui::Panel::right("myset_detail")
            .resizable(false)
            .default_size(250.0)
            .show(ui, |ui| {
                if ui.button("この装備をマイセットから削除する").clicked() {
                    actions.push(Action::DeleteMySet);
                }
                if ui.button("この装備のスキルを検索条件に入力する").clicked() {
                    actions.push(Action::InputMySetCondition);
                }
                ui.horizontal(|ui| {
                    ui.label("名前：");
                    ui.add_sized(
                        [100.0, 20.0],
                        egui::TextEdit::singleline(&mut self.my_name_input),
                    );
                    if ui.button("名前変更").clicked() {
                        actions.push(Action::ChangeMySetName);
                    }
                });
                if let Some(index) = self.my_selected {
                    ui.horizontal(|ui| {
                        if index > 0 && ui.small_button("上へ").clicked() {
                            actions.push(Action::MoveMySet(index, index - 1));
                        }
                        let count = self.sim.lock().unwrap().masters.my_sets.len();
                        if index + 1 < count && ui.small_button("下へ").clicked() {
                            actions.push(Action::MoveMySet(index, index + 1));
                        }
                    });
                }
                let set = self.my_selected.and_then(|index| {
                    let sim = self.sim.lock().unwrap();
                    sim.masters.my_sets.get(index).cloned()
                });
                if let Some(set) = set {
                    for equip in set.equipments() {
                        ui.horizontal(|ui| {
                            ui.label(equip.kind.str_with_colon());
                            ui.add_sized(
                                [95.0, 18.0],
                                egui::Label::new(equip.disp_name()).truncate(),
                            );
                            if ui.small_button("除外").clicked() {
                                actions.push(Action::AddExclude(equip.name.clone()));
                            }
                            let can_include =
                                equip.kind != EquipKind::Deco && equip.kind != EquipKind::Weapon;
                            if ui
                                .add_enabled(can_include, egui::Button::new("固定").small())
                                .clicked()
                            {
                                actions.push(Action::AddInclude(equip.name.clone()));
                            }
                        });
                    }
                    ui.separator();
                    let description = set.description();
                    egui::ScrollArea::vertical()
                        .id_salt("myset_description")
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(description).wrap());
                        });
                }
            });
        let sim = self.sim.lock().unwrap();
        let sets = sim.masters.my_sets.clone();
        egui::ScrollArea::vertical()
            .id_salt("myset_list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Grid::new("mysetgrid").striped(true).show(ui, |ui| {
                    for header in [
                        "防御",
                        "武器",
                        "名前",
                        "発動スキル",
                        "火",
                        "水",
                        "雷",
                        "氷",
                        "龍",
                        "頭",
                        "胴",
                        "腕",
                        "腰",
                        "足",
                        "護石",
                        "装飾品",
                    ] {
                        ui.strong(header);
                    }
                    ui.end_row();
                    for (index, set) in sets.iter().enumerate() {
                        ui.label(set.maxdef().to_string());
                        ui.label(set.weapon.detail_disp_name());
                        if ui
                            .selectable_label(self.my_selected == Some(index), &set.name)
                            .clicked()
                        {
                            actions.push(Action::SelectMySet(index));
                        }
                        ui.label(set.skills_disp());
                        ui.label(set.fire().to_string());
                        ui.label(set.water().to_string());
                        ui.label(set.thunder().to_string());
                        ui.label(set.ice().to_string());
                        ui.label(set.dragon().to_string());
                        ui.label(set.head.detail_disp_name());
                        ui.label(set.body.detail_disp_name());
                        ui.label(set.arm.detail_disp_name());
                        ui.label(set.waist.detail_disp_name());
                        ui.label(set.leg.detail_disp_name());
                        ui.label(set.charm.disp_name());
                        ui.label(set.deco_name_csv());
                        ui.end_row();
                    }
                });
            });
        drop(sim);
        for action in actions {
            self.apply_action(action);
        }
    }

    fn draw_license_tab(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .id_salt("license")
            .show(ui, |ui| {
                ui.heading("WildsSim Rust port");
                ui.label("オリジナル: https://github.com/EXXXI/WildsSim (MIT License)");
                ui.separator();
                ui.columns(2, |columns| {
                    columns[0].label(LICENSE_TEXT);
                    columns[1].label(LICENSE_EXPLANATION);
                });
                ui.separator();
                ui.label(format!("データフォルダ: {}", self.data_root.display()));
            });
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll_job();
        if self.busy() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
        egui::Panel::top("tabs").show(ui, |ui| self.draw_tabs(ui));
        egui::Panel::bottom("status").show(ui, |ui| self.draw_status(ui));
        egui::CentralPanel::default().show(ui, |ui| match self.tab {
            Tab::Skill => self.draw_skill_tab(ui),
            Tab::Result => self.draw_result_tab(ui),
            Tab::Clude => self.draw_clude_tab(ui),
            Tab::Charm => self.draw_charm_tab(ui),
            Tab::Artian => self.draw_artian_tab(ui),
            Tab::Deco => self.draw_deco_tab(ui),
            Tab::MySet => self.draw_myset_tab(ui),
            Tab::License => self.draw_license_tab(ui),
        });
        self.draw_confirm(&ctx);
        self.draw_error(&ctx);
    }
}

impl App {
    fn draw_confirm(&mut self, ctx: &egui::Context) {
        let mut execute = None;
        let mut close = false;
        if let Some(confirm) = &self.confirm {
            egui::Window::new(&confirm.title)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(&confirm.message);
                    ui.horizontal(|ui| {
                        if ui.button("はい").clicked() {
                            execute = Some(match &confirm.action {
                                ConfirmAction::ResetCondition => ConfirmAction::ResetCondition,
                                ConfirmAction::DeleteCharm(name) => {
                                    ConfirmAction::DeleteCharm(name.clone())
                                }
                                ConfirmAction::DeleteArtian(name) => {
                                    ConfirmAction::DeleteArtian(name.clone())
                                }
                                ConfirmAction::DeleteMySet(index) => {
                                    ConfirmAction::DeleteMySet(*index)
                                }
                                ConfirmAction::DeleteMyCondition(index) => {
                                    ConfirmAction::DeleteMyCondition(*index)
                                }
                            });
                        }
                        if ui.button("いいえ").clicked() {
                            close = true;
                        }
                    });
                });
        }
        if let Some(action) = execute {
            self.confirm = None;
            self.execute_confirm(action);
        } else if close {
            self.confirm = None;
        }
    }

    fn draw_error(&mut self, ctx: &egui::Context) {
        let mut close = false;
        if let Some(message) = &self.error {
            egui::Window::new("エラー")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("OK").clicked() {
                        close = true;
                    }
                });
        }
        if close {
            self.error = None;
        }
    }
}

struct CellSpec {
    name: Option<String>,
    disp: String,
    kind: Option<EquipKind>,
}

struct RowSpec {
    cells: Vec<CellSpec>,
}

fn make_cell(equip: Option<&Equipment>) -> CellSpec {
    match equip {
        Some(equip) => CellSpec {
            name: Some(equip.name.clone()),
            disp: equip.disp_name().to_string(),
            kind: Some(equip.kind),
        },
        None => CellSpec {
            name: None,
            disp: String::new(),
            kind: None,
        },
    }
}

fn filtered_equips<'a>(
    equips: &'a [Equipment],
    filter: &str,
    only: bool,
    cludes: &HashMap<String, CludeKind>,
) -> Vec<&'a Equipment> {
    equips
        .iter()
        .filter(|equip| {
            (filter.is_empty() || equip.disp_name().contains(filter))
                && (!only || cludes.contains_key(&equip.name))
        })
        .collect()
}

fn build_armor_rows(
    masters: &Masters,
    filter: &str,
    only: bool,
    cludes: &HashMap<String, CludeKind>,
) -> Vec<RowSpec> {
    let armors = [
        filtered_equips(&masters.heads, filter, only, cludes),
        filtered_equips(&masters.bodys, filter, only, cludes),
        filtered_equips(&masters.arms, filter, only, cludes),
        filtered_equips(&masters.waists, filter, only, cludes),
        filtered_equips(&masters.legs, filter, only, cludes),
    ];
    let mut charm_list: Vec<&Equipment> = masters.charms.iter().collect();
    charm_list.extend(masters.additional_charms.iter());
    let charm_list = charm_list
        .into_iter()
        .filter(|equip| {
            (filter.is_empty() || equip.disp_name().contains(filter))
                && (!only || cludes.contains_key(&equip.name))
        })
        .collect::<Vec<_>>();
    let deco_list = filtered_equips(&masters.decos, filter, only, cludes);
    let mut row_nos: Vec<i32> = Vec::new();
    for list in &armors {
        for equip in list {
            if !row_nos.contains(&equip.row_no) {
                row_nos.push(equip.row_no);
            }
        }
    }
    row_nos.sort();
    let mut rows: Vec<RowSpec> = Vec::new();
    for row_no in row_nos {
        let lists: Vec<Vec<&Equipment>> = armors
            .iter()
            .map(|list| {
                list.iter()
                    .filter(|e| e.row_no == row_no)
                    .copied()
                    .collect()
            })
            .collect();
        let max = lists.iter().map(|list| list.len()).max().unwrap_or(0);
        for i in 0..max {
            let mut cells = Vec::new();
            for list in &lists {
                cells.push(make_cell(list.get(i).copied()));
            }
            cells.push(make_cell(charm_list.get(rows.len()).copied()));
            cells.push(make_cell(deco_list.get(rows.len()).copied()));
            rows.push(RowSpec { cells });
        }
    }
    while rows.len() < charm_list.len().max(deco_list.len()) {
        let index = rows.len();
        let mut cells = vec![
            make_cell(None),
            make_cell(None),
            make_cell(None),
            make_cell(None),
            make_cell(None),
        ];
        cells.push(make_cell(charm_list.get(index).copied()));
        cells.push(make_cell(deco_list.get(index).copied()));
        rows.push(RowSpec { cells });
    }
    rows
}

fn build_weapon_rows(
    masters: &Masters,
    filter: &str,
    only: bool,
    cludes: &HashMap<String, CludeKind>,
) -> Vec<RowSpec> {
    let mut lists: Vec<Vec<&Equipment>> = Vec::new();
    for weapon_type in WEAPON_TYPES {
        let list = masters
            .weapons
            .iter()
            .chain(masters.artians.iter())
            .filter(|weapon| weapon.weapon_type == weapon_type)
            .filter(|equip| {
                (filter.is_empty() || equip.disp_name().contains(filter))
                    && (!only || cludes.contains_key(&equip.name))
            })
            .collect::<Vec<_>>();
        lists.push(list);
    }
    let mut row_nos: Vec<i32> = Vec::new();
    for list in &lists {
        for equip in list {
            if !row_nos.contains(&equip.row_no) {
                row_nos.push(equip.row_no);
            }
        }
    }
    row_nos.sort();
    let mut rows = Vec::new();
    for row_no in row_nos {
        let lists: Vec<Vec<&Equipment>> = lists
            .iter()
            .map(|list| {
                list.iter()
                    .filter(|e| e.row_no == row_no)
                    .copied()
                    .collect()
            })
            .collect();
        let max = lists.iter().map(|list| list.len()).max().unwrap_or(0);
        for i in 0..max {
            let cells = lists
                .iter()
                .map(|list| make_cell(list.get(i).copied()))
                .collect();
            rows.push(RowSpec { cells });
        }
    }
    rows
}

fn clude_cell(
    ui: &mut egui::Ui,
    cell: &CellSpec,
    cludes: &HashMap<String, CludeKind>,
    actions: &mut Vec<Action>,
) {
    ui.vertical(|ui| {
        ui.add_sized([145.0, 16.0], egui::Label::new(&cell.disp).truncate());
        if let Some(name) = &cell.name {
            let kind = cludes.get(name).copied();
            let mut exclude = kind == Some(CludeKind::Exclude);
            let mut include = kind == Some(CludeKind::Include);
            if ui.checkbox(&mut exclude, "除外").changed() {
                if exclude {
                    actions.push(Action::AddExclude(name.clone()));
                } else {
                    actions.push(Action::DeleteClude(name.clone()));
                }
            }
            let can_include = !matches!(cell.kind, Some(EquipKind::Deco) | Some(EquipKind::Weapon));
            if ui
                .add_enabled(can_include, egui::Checkbox::new(&mut include, "固定"))
                .changed()
            {
                if include {
                    actions.push(Action::AddInclude(name.clone()));
                } else {
                    actions.push(Action::DeleteClude(name.clone()));
                }
            }
        } else {
            ui.label("");
            ui.label("");
        }
    });
}

fn result_columns(set: &EquipSet) -> Vec<String> {
    vec![
        set.weapon.detail_disp_name().to_string(),
        set.head.detail_disp_name().to_string(),
        set.body.detail_disp_name().to_string(),
        set.arm.detail_disp_name().to_string(),
        set.waist.detail_disp_name().to_string(),
        set.leg.detail_disp_name().to_string(),
        set.charm.disp_name().to_string(),
        set.fire().to_string(),
        set.water().to_string(),
        set.thunder().to_string(),
        set.ice().to_string(),
        set.dragon().to_string(),
        set.deco_name_csv(),
        set.skills_disp(),
    ]
}

fn skill_level_items(masters: &Masters, name: &str) -> Vec<(i32, String)> {
    let master = match masters.skills.iter().find(|skill| skill.name == name) {
        Some(master) => master,
        None => return vec![(0, String::new())],
    };
    let mut items = vec![(0, master.name.clone())];
    for level in 1..=master.level {
        if master.is_hide_level(Some(level)) {
            continue;
        }
        let label = master
            .specific_names
            .get(&level)
            .map(|specific| format!("{}({}Lv{})", specific, master.name, level))
            .unwrap_or_else(|| format!("{}Lv{}", master.name, level));
        items.push((level, label));
    }
    items
}

fn valid_level(masters: &Masters, name: &str, level: i32) -> i32 {
    let items = skill_level_items(masters, name);
    let mut level = level;
    while level > 0 && !items.iter().any(|(value, _)| *value == level) {
        level -= 1;
    }
    level
}

fn normal_selector() -> SkillSelectorState {
    SkillSelectorState {
        name: String::new(),
        level: 0,
        levels: vec![(0, String::new())],
        group_only: false,
        series_only: false,
    }
}

fn artian_selector(group_only: bool) -> SkillSelectorState {
    SkillSelectorState {
        name: String::new(),
        level: 0,
        levels: vec![(0, String::new())],
        group_only,
        series_only: !group_only,
    }
}

fn set_selector_name(
    selector: &mut SkillSelectorState,
    name: &str,
    artian: bool,
    sim: &Arc<Mutex<Simulator>>,
) {
    selector.name = name.to_string();
    let sim = sim.lock().unwrap();
    let master = sim.masters.skills.iter().find(|s| s.name == name);
    let master = match master {
        Some(master) => master,
        None => {
            selector.level = 0;
            selector.levels = vec![(0, String::new())];
            return;
        }
    };
    if selector.group_only && (master.category != "グループスキル" || !master.can_with_artian)
    {
        selector.level = 0;
        selector.levels = vec![(0, String::new())];
        return;
    }
    if selector.series_only && (master.category != "シリーズスキル" || !master.can_with_artian)
    {
        selector.level = 0;
        selector.levels = vec![(0, String::new())];
        return;
    }
    if artian {
        selector.levels = vec![(1, "1".to_string())];
        selector.level = 1;
    } else {
        selector.levels = (0..=master.level)
            .rev()
            .map(|level| (level, level.to_string()))
            .collect();
        selector.level = master.level;
    }
}

fn draw_skill_selector(
    ui: &mut egui::Ui,
    selector: &mut SkillSelectorState,
    sim: &Arc<Mutex<Simulator>>,
    actions: &mut Vec<Action>,
    index: usize,
    charm: bool,
) {
    ui.horizontal(|ui| {
        let sim_guard = sim.lock().unwrap();
        let names = sim_guard
            .masters
            .skills
            .iter()
            .filter(|skill| {
                (!selector.group_only
                    || (skill.category == "グループスキル" && skill.can_with_artian))
                    && (!selector.series_only
                        || (skill.category == "シリーズスキル" && skill.can_with_artian))
            })
            .map(|skill| skill.name.clone())
            .collect::<Vec<_>>();
        drop(sim_guard);
        egui::ComboBox::from_id_salt(("selectorname", charm, index))
            .width(115.0)
            .selected_text(if selector.name.is_empty() {
                if selector.group_only {
                    "グループスキル"
                } else if selector.series_only {
                    "シリーズスキル"
                } else {
                    "スキル選択"
                }
            } else {
                &selector.name
            })
            .show_ui(ui, |ui| {
                for name in names {
                    if ui.selectable_label(selector.name == name, &name).clicked() {
                        let value = name.clone();
                        if charm {
                            actions.push(Action::SetCharmSkillName(index, value));
                        } else {
                            actions.push(Action::SetArtianSkillName(index, value));
                        }
                    }
                }
            });
        if charm {
            egui::ComboBox::from_id_salt(("selectorlevel", index))
                .width(45.0)
                .selected_text(selector.level.to_string())
                .show_ui(ui, |ui| {
                    for (level, label) in &selector.levels {
                        if ui
                            .selectable_label(selector.level == *level, label)
                            .clicked()
                        {
                            actions.push(Action::SetCharmSkillLevel(index, *level));
                        }
                    }
                });
        } else {
            ui.label(selector.level.to_string());
        }
        if ui.small_button("クリア").clicked() {
            if charm {
                actions.push(Action::ClearCharmSkill(index));
            } else {
                actions.push(Action::ClearArtianSkill(index));
            }
        }
    });
}

fn charm_upper_text(charm: &Equipment) -> String {
    match &charm.upper {
        Some(upper) => {
            if upper.is_full_upper {
                format!("上位互換の護石が存在します：{}", upper.disp_name)
            } else {
                format!("同値の護石が存在します：{}", upper.disp_name)
            }
        }
        None => String::new(),
    }
}

fn parse_i32(value: &str, default: i32) -> i32 {
    value.trim().parse::<i32>().unwrap_or(default)
}

fn parse_optional(value: &str) -> Option<i32> {
    value.trim().parse::<i32>().ok()
}

fn install_fonts(ctx: &egui::Context) {
    let candidates = [
        "C:\\Windows\\Fonts\\meiryo.ttc",
        "C:\\Windows\\Fonts\\msgothic.ttc",
        "C:\\Windows\\Fonts\\YuGothM.ttc",
        "C:\\Windows\\Fonts\\NotoSansJP-VF.ttf",
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "jp".to_string(),
                Arc::new(egui::FontData::from_owned(bytes)),
            );
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                family.insert(0, "jp".to_string());
            }
            if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                family.push("jp".to_string());
            }
            ctx.set_fonts(fonts);
            break;
        }
    }
}

const LICENSE_TEXT: &str = "■バージョン\n20260904.0\n\n■このシミュのライセンス\nMIT License\n\n■使わせていただいたOSS\n・Google OR-Tools (original)\nhttps://github.com/google/or-tools\n\n・CSV (original)\nhttps://github.com/stevehansen/csv/\n\n・Prism.Wpf (original)\nhttps://github.com/PrismLibrary/Prism\n\n・ReactiveProperty (original)\nhttps://github.com/runceel/ReactiveProperty\n\n・NLog (original)\nhttps://nlog-project.org/\n\n・DotNetKit.Wpf.AutoCompleteComboBox (original)\nhttps://github.com/vain0x/DotNetKit.Wpf.AutoCompleteComboBox/\n\n■Rust port\n・microlp\nhttps://github.com/Specy/microlp\n\n・egui / eframe\nhttps://github.com/emilk/egui\n\n・csv\nhttps://github.com/BurntSushi/rust-csv\n\n・serde_json\nhttps://github.com/serde-rs/json\n\n・rayon\nhttps://github.com/rayon-rs/rayon\n\n・uuid\nhttps://github.com/uuid-rs/uuid\n\n■スペシャルサンクス\n・5chモンハン板シミュスレの方々\nhttps://github.com/13-480/lp-doc\n\n・先人のシミュ作成者様\n特に頑シミュ様に大きく影響を受けています";

const LICENSE_EXPLANATION: &str = "■ライセンスって何？\n普通にスキルシミュとして使う分には気にしないでOK\n\n■もっと詳しく\nこういう使い方までならOKだよ、ってのを定める取り決め\n今回のは大体こんな感じ\n・基本は好きに使ってOK\n・このシミュのせいで何か起きても開発者は責任取らんよ\n・改変や再配布するときだけはよく調べてルールに従ってね";
