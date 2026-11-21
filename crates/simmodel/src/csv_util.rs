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
    header_index: HashMap<String, usize>,
    pub rows: Vec<CsvRow>,
}

impl CsvTable {
    // Parse errors are propagated: silently skipping a broken row (e.g. an
    // unclosed quote) would make the next save truncate the file.
    pub fn parse(text: &str) -> SimResult<CsvTable> {
        if text.is_empty() {
            return Ok(CsvTable::default());
        }
        let text = text.strip_prefix('#').unwrap_or(text);
        let mut reader = csv::ReaderBuilder::new()
            .has_headers(true)
            .flexible(true)
            .comment(Some(b'#'))
            .from_reader(text.as_bytes());
        let mut header_index = HashMap::new();
        {
            let headers = reader.headers()?;
            for (i, header) in headers.iter().enumerate() {
                header_index.insert(header.to_string(), i);
            }
        }
        let mut rows = Vec::new();
        for record in reader.records() {
            let record = record?;
            rows.push(CsvRow {
                values: record.iter().map(|s| s.to_string()).collect(),
            });
        }
        Ok(CsvTable { header_index, rows })
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
