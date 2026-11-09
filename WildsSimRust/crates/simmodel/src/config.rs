use std::path::Path;

use crate::error::{SimError, SimResult};
use crate::parse_util::{parse_bool, parse_i32};

#[derive(Clone, Debug)]
pub struct LogicConfig {
    pub max_slot_size: i32,
    pub max_recent_skill_count: i32,
    pub max_charm_skill_count: i32,
    pub max_degree_of_parallelism: i32,
    pub allow_unavailable_equipments: bool,
    pub use_calc_upper_charm: bool,
    pub artian_skill_count: i32,
    pub default_my_set_name: String,
}

impl Default for LogicConfig {
    fn default() -> Self {
        LogicConfig {
            max_slot_size: 4,
            max_recent_skill_count: 20,
            max_charm_skill_count: 3,
            max_degree_of_parallelism: -1,
            allow_unavailable_equipments: false,
            use_calc_upper_charm: true,
            artian_skill_count: 2,
            default_my_set_name: "マイセット".to_string(),
        }
    }
}

impl LogicConfig {
    pub fn load(conf_csv: &Path) -> SimResult<LogicConfig> {
        let mut config = LogicConfig::default();
        let text = match read_text(conf_csv) {
            Ok(text) => text,
            Err(error) => {
                return Err(SimError(format!(
                    "設定ファイル {} の読み込みに失敗しました。: {}",
                    conf_csv.display(),
                    error
                )))
            }
        };
        let table = crate::csv_util::CsvTable::parse(&text);
        if let Some(row) = table.rows.first() {
            config.max_slot_size =
                get_i32(&table, row, "スロットの最大の大きさ", config.max_slot_size);
            config.max_recent_skill_count = get_i32(
                &table,
                row,
                "最近使ったスキルの記憶容量",
                config.max_recent_skill_count,
            );
            config.max_charm_skill_count = get_i32(
                &table,
                row,
                "追加護石のスキル最大個数",
                config.max_charm_skill_count,
            );
            config.max_degree_of_parallelism = get_i32(
                &table,
                row,
                "最大並列処理数",
                config.max_degree_of_parallelism,
            );
            config.allow_unavailable_equipments = get_bool(
                &table,
                row,
                "入手不可装備の利用有無",
                config.allow_unavailable_equipments,
            );
            config.use_calc_upper_charm = get_bool(
                &table,
                row,
                "下位互換護石の検出有無",
                config.use_calc_upper_charm,
            );
            config.artian_skill_count = get_i32(
                &table,
                row,
                "アーティア武器のスキル数",
                config.artian_skill_count,
            );
            config.default_my_set_name = get_string(
                &table,
                row,
                "マイセットのデフォルト名",
                &config.default_my_set_name,
            );
        }
        Ok(config)
    }
}

#[derive(Clone, Debug)]
pub struct ViewConfig {
    pub default_limit: String,
    pub no_skill_name: String,
    pub use_saved_column_indexes: bool,
}

impl Default for ViewConfig {
    fn default() -> Self {
        ViewConfig {
            default_limit: "100".to_string(),
            no_skill_name: "スキル選択".to_string(),
            use_saved_column_indexes: false,
        }
    }
}

impl ViewConfig {
    pub fn load(conf_csv: &Path) -> SimResult<ViewConfig> {
        let mut config = ViewConfig::default();
        let text = match read_text(conf_csv) {
            Ok(text) => text,
            Err(error) => {
                return Err(SimError(format!(
                    "設定ファイル {} の読み込みに失敗しました。: {}",
                    conf_csv.display(),
                    error
                )))
            }
        };
        let table = crate::csv_util::CsvTable::parse(&text);
        if let Some(row) = table.rows.first() {
            config.default_limit =
                get_string(&table, row, "デフォルトの頑張り度", &config.default_limit);
            config.no_skill_name =
                get_string(&table, row, "スキル未選択時の表示", &config.no_skill_name);
            config.use_saved_column_indexes =
                get_string(&table, row, "グリッドの列順保存有無", "有") == "有";
        }
        Ok(config)
    }
}

fn read_text(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let bytes = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        &bytes[3..]
    } else {
        &bytes[..]
    };
    Ok(String::from_utf8_lossy(bytes).into_owned())
}

fn get_string(
    table: &crate::csv_util::CsvTable,
    row: &crate::csv_util::CsvRow,
    name: &str,
    default: &str,
) -> String {
    table.get(row, name).ok().unwrap_or(default).to_string()
}

fn get_i32(
    table: &crate::csv_util::CsvTable,
    row: &crate::csv_util::CsvRow,
    name: &str,
    default: i32,
) -> i32 {
    match table.get(row, name).ok() {
        Some(value) => parse_i32(value, default),
        None => default,
    }
}

fn get_bool(
    table: &crate::csv_util::CsvTable,
    row: &crate::csv_util::CsvRow,
    name: &str,
    default: bool,
) -> bool {
    match table.get(row, name).ok() {
        Some(value) => parse_bool(value, default),
        None => default,
    }
}
