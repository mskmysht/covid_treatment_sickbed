use std::{fs::File, io::Write, num::ParseIntError, path::Path};

use calamine::{open_workbook_auto, DataType, Range, Reader};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Record {
    prefecture: Prefecture,
    phase: Phase,
    inpatient_count: PatientCount,
    dedicated_bed_count: ResourceCount,
}

#[derive(Debug, Serialize)]
struct Prefecture {
    code: String,
    name: String,
}

#[derive(Debug, Serialize)]
struct Phase {
    current: u8,
    maximum: u8,
    mode: PhaseMode,
}

#[derive(Debug, Serialize)]
enum PhaseMode {
    /// 一般フェーズ
    Normal,
    /// 緊急フェーズ
    Emergency,
}

#[derive(Debug, Serialize)]
struct PatientCount {
    /// 患者総数（入院者数）
    total: u32,
    /// 専用リソース（確保病床）使用者
    dedicated: u32,
    /// 臨時の専用リソース（臨時・待機病床）使用者
    extra: u32,
}

#[derive(Debug, Serialize)]
struct ResourceCount {
    /// 即応病床
    available_or_assigned: u32,
    /// 確保病床
    guaranteed: u32,
    /// 臨時医療施設と入院待機施設の確保病床
    extra_guaranteed: u32,
}

const START_ROW: u32 = 8; // 9行目
const END_ROW: u32 = 54; // 55行目
fn collect_records(data: &Range<DataType>) -> Vec<Record> {
    (START_ROW..=END_ROW)
        .map(|row| read_record(data, row))
        .collect()
}

const PREFECTURE_INFO: u32 = 0;
const PHASE_INFO: u32 = 5;
const INPATIENT_TOTAL: u32 = 2;
const INPATIENT_DEDICATED: u32 = 3;
const INPATIENT_EXTRA: u32 = 4;
const AVAILABLE_OR_ASSIGNED: u32 = 6;
const GUARANTEED: u32 = 7;
const EXTRA_GUARANTEED: u32 = 8;
fn read_record(data: &Range<DataType>, row: u32) -> Record {
    let prefecture = {
        let s = data
            .get_value((row, PREFECTURE_INFO))
            .expect(&format!("Out of range of prefecture in {row} th row."))
            .to_string();
        let mut s = s.split(' ');
        let code = s.next().unwrap().trim().to_string();
        let name = s.next().unwrap().trim().to_string();
        Prefecture { code, name }
    };
    let phase = parse_phase(
        &data
            .get_value((row, PHASE_INFO))
            .expect(&format!("Out of range of phase info in {row} th row."))
            .to_string(),
    )
    .unwrap();
    let total = get_number(data, row, INPATIENT_TOTAL).expect(&format!(
        "Out of range or non-integer value of inpatient in {row} th row."
    ));
    let dedicated = get_number(data, row, INPATIENT_DEDICATED).expect(&format!(
        "Out of range or non-integer value of dedicated in {row} th row."
    ));
    let extra = get_number(data, row, INPATIENT_EXTRA).expect(&format!(
        "Out of range or non-integer value of extra in {row} th row."
    ));
    let available_or_assigned = get_number(data, row, AVAILABLE_OR_ASSIGNED).expect(&format!(
        "Out of range or non-integer value of available bed in {row} th row."
    ));
    let guaranteed = get_number(data, row, GUARANTEED).expect(&format!(
        "Out of range or non-integer value of guaranteed bed in {row} th row."
    ));
    let extra_guaranteed = get_number(data, row, EXTRA_GUARANTEED).expect(&format!(
        "Out of range or non-integer value of extra guaranteed bed in {row} th row."
    ));

    Record {
        prefecture,
        phase,
        inpatient_count: PatientCount {
            total,
            dedicated,
            extra,
        },
        dedicated_bed_count: ResourceCount {
            available_or_assigned,
            guaranteed,
            extra_guaranteed,
        },
    }
}

fn get_number(data: &Range<DataType>, row: u32, column: u32) -> Option<u32> {
    match data.get_value((row, column))? {
        DataType::Int(i) => Some(*i as u32),
        DataType::Float(f) => Some(*f as u32),
        _ => None,
    }
}

fn parse_phase(s: &str) -> Result<Phase, MyError> {
    let mut s = s.split('／');
    let c = s.next().unwrap().trim();
    let m = s.next().unwrap().trim();

    if let Some(c) = util::to_half_digits(c) {
        Ok(Phase {
            current: c.parse()?,
            maximum: util::to_half_digits(m).unwrap().parse()?,
            mode: PhaseMode::Normal,
        })
    } else {
        Ok(Phase {
            current: parse_roman_numerals(c)?,
            maximum: parse_roman_numerals(m)?,
            mode: PhaseMode::Emergency,
        })
    }
}

#[derive(thiserror::Error, Debug)]
enum MyError {
    #[error("Invalid char {0}")]
    ParseRomanError(char),
    #[error("Parse int error")]
    ParseIntError(#[from] ParseIntError),
}

/// support for 1-9 range
fn parse_roman_numerals(s: &str) -> Result<u8, MyError> {
    let mut n = 0;
    let mut cs = s.chars().peekable();
    loop {
        let Some(c) = cs.next() else { break; };
        let k = match c {
            'I' | '\u{2160}' => match cs.peek() {
                Some('V') | Some('\u{3264}') => {
                    cs.next();
                    4
                }
                Some('X') | Some('\u{2169}') => {
                    cs.next();
                    9
                }
                _ => 1,
            },
            'V' => 5,
            '\u{2161}'..='\u{2168}' => (u32::try_from(c).unwrap() - 0x2160) as u8 + 1,
            _ => return Err(MyError::ParseRomanError(c)),
        };
        n += k;
    }
    Ok(n)
}

#[argopt::cmd]
fn main(report_file: String, save_to: String) {
    let path = Path::new(&report_file);
    if !path.exists() {
        eprintln!("[error] {report_file}: File is not found.");
        return;
    }

    let save_dir = Path::new(&save_to);
    if !save_dir.exists() {
        eprintln!("[error] {save_to}: Directory not found.");
        return;
    }

    let out_path = save_dir.join(path.with_extension("json").file_name().unwrap());
    if out_path.exists() {
        println!(
            "[warn] Skipped {}: File already exists.",
            out_path.display()
        );
        return;
    }

    let mut wb = open_workbook_auto(path).expect("Cannot open file.");
    let (sheet_name, ws) = &wb.worksheets()[0];
    println!("Extracting {sheet_name} sheet in {}...", path.display());
    let records = collect_records(ws);

    let mut file = File::create(out_path).unwrap();
    file.write_all(serde_json::to_string_pretty(&records).unwrap().as_bytes())
        .unwrap();

    println!("Done.");
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use calamine::{open_workbook, Reader, Xlsx};

    use crate::{collect_records, read_record, PhaseMode, START_ROW};

    const TEST_FILE: &str = "test/001019538.xlsx";

    #[test]
    fn test_correct_records() {
        let path = Path::new(TEST_FILE);
        let mut wb: Xlsx<_> = open_workbook(path).expect("Cannot open file.");
        let ws = &wb.worksheets()[0].1;
        let rs = collect_records(ws);
        for (i, r) in rs.iter().enumerate() {
            assert_eq!(r.prefecture.code.parse::<usize>().unwrap(), i + 1);
        }
    }

    #[test]
    fn test_read_record_normal() {
        let path = Path::new(TEST_FILE);
        let mut wb: Xlsx<_> = open_workbook(path).expect("Cannot open file.");
        let ws = &wb.worksheets()[0].1;

        let r = read_record(ws, START_ROW + 12);
        assert_eq!(r.prefecture.code, "13");
        assert_eq!(r.prefecture.name, "東京都");
        assert_eq!(matches!(r.phase.mode, PhaseMode::Normal), true);
        assert_eq!(r.phase.current, 2);
        assert_eq!(r.phase.maximum, 2);
        assert_eq!(r.inpatient_count.total, 3066);
        assert_eq!(r.inpatient_count.dedicated, 2924);
        assert_eq!(r.inpatient_count.extra, 225);
        assert_eq!(r.dedicated_bed_count.available_or_assigned, 5005);
        assert_eq!(r.dedicated_bed_count.guaranteed, 7496);
        assert_eq!(r.dedicated_bed_count.extra_guaranteed, 579);
    }

    #[test]
    fn test_read_record_emergency() {
        let path = Path::new(TEST_FILE);
        let mut wb: Xlsx<_> = open_workbook(path).expect("Cannot open file.");
        let ws = &wb.worksheets()[0].1;

        let r = read_record(ws, START_ROW + 5);
        assert_eq!(r.prefecture.code, "06");
        assert_eq!(r.prefecture.name, "山形県");
        assert_eq!(matches!(r.phase.mode, PhaseMode::Emergency), true);
        assert_eq!(r.phase.current, 1);
        assert_eq!(r.phase.maximum, 2);
        assert_eq!(r.inpatient_count.total, 457);
        assert_eq!(r.inpatient_count.dedicated, 151);
        assert_eq!(r.inpatient_count.extra, 0);
        assert_eq!(r.dedicated_bed_count.available_or_assigned, 284);
        assert_eq!(r.dedicated_bed_count.guaranteed, 284);
        assert_eq!(r.dedicated_bed_count.extra_guaranteed, 0);
    }
}
