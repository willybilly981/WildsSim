use std::collections::HashMap;

use crate::error::SimResult;

#[derive(Clone, Debug)]
pub struct CsvRow {
    values: Vec<String>,
}

impl CsvRow {
    pub fn get(&self, index: usize) -> Option<&str> {
        self.values.get(index).map(|s| s.as_str())
    }
}

#[derive(Clone, Debug, Default)]
pub struct CsvTable {
    headers: Vec<String>,
    header_index: HashMap<String, usize>,
    pub rows: Vec<CsvRow>,
}

impl CsvTable {
    pub fn parse(text: &str) -> CsvTable {
        if text.is_empty() {
            return CsvTable::default();
        }
        let text = if let Some(stripped) = text.strip_prefix('#') {
            stripped.to_string()
        } else {
            text.to_string()
        };
        let text = text
            .replace("生産素材1,個数", "生産素材1,生産素材個数1")
            .replace("生産素材2,個数", "生産素材2,生産素材個数2")
            .replace("生産素材3,個数", "生産素材3,生産素材個数3")
            .replace("生産素材4,個数", "生産素材4,生産素材個数4")
            .replace("生産素材A1,個数", "生産素材1,生産素材個数1")
            .replace("生産素材A2,個数", "生産素材2,生産素材個数2")
            .replace("生産素材A3,個数", "生産素材3,生産素材個数3")
            .replace("生産素材A4,個数", "生産素材4,生産素材個数4");
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .comment(Some(b'#'))
            .from_reader(text.as_bytes());
        let headers = match reader.headers() {
            Ok(headers) => headers.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        };
        let mut header_index = HashMap::new();
        for (i, header) in headers.iter().enumerate() {
            header_index.insert(header.clone(), i);
        }
        let mut rows = Vec::new();
        for record in reader.records().flatten() {
            rows.push(CsvRow {
                values: record.iter().map(|s| s.to_string()).collect(),
            });
        }
        CsvTable {
            headers,
            header_index,
            rows,
        }
    }

    pub fn has_column(&self, name: &str) -> bool {
        self.header_index.contains_key(name)
    }

    pub fn get<'a>(&self, row: &'a CsvRow, name: &str) -> SimResult<&'a str> {
        let index = self
            .header_index
            .get(name)
            .copied()
            .ok_or_else(|| crate::error::SimError(format!("列 {} が存在しません", name)))?;
        row.get(index)
            .ok_or_else(|| crate::error::SimError(format!("列 {} の値が存在しません", name)))
    }

    pub fn get_opt<'a>(&self, row: &'a CsvRow, name: &str) -> Option<&'a str> {
        let index = self.header_index.get(name).copied()?;
        row.get(index)
    }

    pub fn contains_value_prefix(&self, row: &CsvRow, name: &str, prefix: &str) -> bool {
        self.get_opt(row, name)
            .map(|v| v.starts_with(prefix))
            .unwrap_or(false)
    }

    pub fn headers(&self) -> &[String] {
        &self.headers
    }
}

pub fn write_csv(headers: &[String], rows: &[Vec<String>]) -> SimResult<String> {
    let mut writer = csv::WriterBuilder::new()
        .terminator(csv::Terminator::CRLF)
        .from_writer(Vec::new());
    writer.write_record(headers)?;
    for row in rows {
        writer.write_record(row)?;
    }
    let bytes = writer
        .into_inner()
        .map_err(|e| crate::error::SimError(e.to_string()))?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}
