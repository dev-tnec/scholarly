use calamine::{Data, DataType, Range, Reader, Xlsx, open_workbook};
use chrono::{Duration, NaiveDate};
use std::path::Path;

use super::{
    ChildRow, DocumentRow, FaqRow, ImportError, Record, RequirementRow, RowReport, StepRow,
    TimelineRow,
};
use crate::domain::scholarship::{DeadlineType, fmt_short};

const SH_SCHOLARSHIPS: &str = "Scholarships";
const SH_BENEFITS: &str = "Keuntungan";
const SH_REQUIREMENTS: &str = "Persyaratan";
const SH_DOCUMENTS: &str = "Dokumen";
const SH_STEPS: &str = "Cara Mendaftar";
const SH_TIMELINE: &str = "Jadwal";
const SH_FAQS: &str = "FAQ";
const SH_CHECKLIST: &str = "Checklist";

/// Parse a hand-typed date cell. Unparseable text is a hard import error.
pub fn parse_date_text(text: &str) -> Option<NaiveDate> {
    let text = text.trim();
    for fmt in ["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%d %b %Y", "%d %B %Y"] {
        if let Ok(d) = NaiveDate::parse_from_str(text, fmt) {
            return Some(d);
        }
    }
    None
}

fn serial_to_date(serial: f64) -> Option<NaiveDate> {
    // Excel epoch, 1899-12-30, including the 1900 leap-year bug convention.
    NaiveDate::from_ymd_opt(1899, 12, 30)?.checked_add_signed(Duration::days(serial.trunc() as i64))
}

fn cell_text(cell: &Data) -> String {
    match cell {
        Data::Empty => String::new(),
        Data::String(s) => s.trim().to_string(),
        Data::Float(f) => {
            if f.fract() == 0.0 {
                format!("{}", *f as i64)
            } else {
                f.to_string()
            }
        }
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => {
            if *b {
                "Ya".into()
            } else {
                "Tidak".into()
            }
        }
        other => other.to_string().trim().to_string(),
    }
}

fn cell_date(cell: &Data) -> Option<NaiveDate> {
    match cell {
        Data::Empty => None,
        Data::String(s) => parse_date_text(s),
        Data::Float(f) => serial_to_date(*f),
        Data::Int(i) => serial_to_date(*i as f64),
        Data::DateTime(_) => cell.as_datetime().map(|dt| dt.date()),
        _ => None,
    }
}

/// Timeline dates may be a real date or a label such as "Feb 2027".
fn cell_date_or_label(cell: &Data) -> (Option<NaiveDate>, String) {
    match cell_date(cell) {
        Some(d) => (Some(d), fmt_short(d)),
        None => (None, cell_text(cell)),
    }
}

struct Sheet {
    header: Vec<String>,
    rows: Vec<Vec<Data>>,
}

impl Sheet {
    fn new(range: &Range<Data>) -> Self {
        let mut rows = range.rows();
        let header = rows
            .next()
            .map(|r| r.iter().map(cell_text).collect())
            .unwrap_or_default();
        Self {
            header,
            rows: rows.map(|r| r.to_vec()).collect(),
        }
    }

    fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }

    fn get(&self, row: &[Data], name: &str) -> String {
        self.col(name)
            .and_then(|i| row.get(i))
            .map(cell_text)
            .unwrap_or_default()
    }

    fn get_cell<'a>(&self, row: &'a [Data], name: &str) -> Option<&'a Data> {
        self.col(name).and_then(|i| row.get(i))
    }
}

fn sheet(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
) -> Result<Sheet, ImportError> {
    let range = wb
        .worksheet_range(name)
        .map_err(|e| ImportError::Workbook(format!("sheet {name}: {e}")))?;
    Ok(Sheet::new(&range))
}

fn split_list(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Read the processed workbook. Returns the records plus per-row parse errors.
pub fn read(path: &Path) -> Result<(Vec<Record>, Vec<RowReport>), ImportError> {
    let mut wb: Xlsx<_> = open_workbook(path)
        .map_err(|e| ImportError::Workbook(format!("{}: {e}", path.display())))?;

    let published = checklist_all_lolos(&mut wb)?;
    let main = sheet(&mut wb, SH_SCHOLARSHIPS)?;
    let benefits = sheet(&mut wb, SH_BENEFITS)?;
    let requirements = sheet(&mut wb, SH_REQUIREMENTS)?;
    let documents = sheet(&mut wb, SH_DOCUMENTS)?;
    let steps = sheet(&mut wb, SH_STEPS)?;
    let timeline = sheet(&mut wb, SH_TIMELINE)?;
    let faqs = sheet(&mut wb, SH_FAQS)?;

    let mut records = Vec::new();
    let mut parse_errors = Vec::new();

    for row in &main.rows {
        let ext_id = main.get(row, "ID");
        let slug = main.get(row, "Slug");
        if ext_id.is_empty() && slug.is_empty() {
            continue;
        }
        let mut row_errors = Vec::new();

        let open_date = read_date(&main, row, "Tanggal Buka", &mut row_errors);
        let close_date = read_date(&main, row, "Tanggal Tutup", &mut row_errors);
        let checked_at = read_date(&main, row, "Tanggal Cek", &mut row_errors);

        let record = Record {
            ext_id: ext_id.clone(),
            slug,
            name: main.get(row, "Nama Tampilan"),
            official_name: main.get(row, "Nama Resmi"),
            abbreviation: main.get(row, "Singkatan"),
            provider: main.get(row, "Penyelenggara"),
            provider_type: main.get(row, "Tipe Penyelenggara"),
            location: main.get(row, "Lokasi"),
            country: main.get(row, "Negara"),
            levels: split_list(&main.get(row, "Jenjang")),
            funding_type: main.get(row, "Pendanaan"),
            open_date,
            close_date,
            deadline_type: if close_date.is_some() {
                DeadlineType::Fixed
            } else {
                DeadlineType::Unknown
            },
            official_url: main.get(row, "URL Resmi"),
            checked_at,
            quota: main.get(row, "Kuota"),
            duration: main.get(row, "Durasi"),
            language_requirement: main.get(row, "Syarat Bahasa"),
            visa_requirement: main.get(row, "Visa/Paspor"),
            research_proposal: main.get(row, "Proposal Penelitian"),
            title_tag: main.get(row, "Title Tag"),
            meta_description: main.get(row, "Meta Description"),
            notes: main.get(row, "Catatan"),
            published,
            related_names: split_list(&main.get(row, "Beasiswa Terkait")),
            ..Record::default()
        };
        records.push(record);
        if !row_errors.is_empty() {
            parse_errors.push(RowReport {
                id: ext_id,
                name: main.get(row, "Nama Tampilan"),
                errors: row_errors,
                warnings: vec![],
            });
        }
    }

    for record in &mut records {
        record.benefits = benefits
            .rows
            .iter()
            .filter(|r| benefits.get(r, "ID Beasiswa") == record.ext_id)
            .map(|r| ChildRow {
                item: benefits.get(r, "Item"),
                detail: benefits.get(r, "Detail"),
                source_url: benefits.get(r, "URL Sumber"),
            })
            .collect();

        record.requirements = requirements
            .rows
            .iter()
            .filter(|r| requirements.get(r, "ID Beasiswa") == record.ext_id)
            .map(|r| RequirementRow {
                group_name: requirements.get(r, "Kelompok"),
                text: requirements.get(r, "Syarat"),
                source_url: requirements.get(r, "URL Sumber"),
            })
            .collect();

        record.documents = documents
            .rows
            .iter()
            .filter(|r| documents.get(r, "ID Beasiswa") == record.ext_id)
            .map(|r| DocumentRow {
                name: documents.get(r, "Dokumen"),
                required: !documents.get(r, "Wajib").eq_ignore_ascii_case("tidak"),
                source_url: documents.get(r, "URL Sumber"),
            })
            .collect();

        record.steps = steps
            .rows
            .iter()
            .filter(|r| steps.get(r, "ID Beasiswa") == record.ext_id)
            .enumerate()
            .map(|(i, r)| StepRow {
                step_no: steps.get(r, "Urutan").parse().unwrap_or((i + 1) as i64),
                title: steps.get(r, "Judul Langkah"),
                detail: steps.get(r, "Keterangan"),
            })
            .collect();

        record.timeline = timeline
            .rows
            .iter()
            .filter(|r| timeline.get(r, "ID Beasiswa") == record.ext_id)
            .map(|r| {
                let (event_date, event_label) = timeline
                    .get_cell(r, "Tanggal")
                    .map(cell_date_or_label)
                    .unwrap_or((None, String::new()));
                TimelineRow {
                    stage: timeline.get(r, "Tahap"),
                    event_date,
                    event_label,
                    note: timeline.get(r, "Keterangan"),
                }
            })
            .collect();

        record.faqs = faqs
            .rows
            .iter()
            .filter(|r| faqs.get(r, "ID Beasiswa") == record.ext_id)
            .map(|r| FaqRow {
                question: faqs.get(r, "Pertanyaan"),
                answer: faqs.get(r, "Jawaban"),
            })
            .collect();
    }

    Ok((records, parse_errors))
}

fn read_date(
    sheet: &Sheet,
    row: &[Data],
    column: &str,
    errors: &mut Vec<String>,
) -> Option<NaiveDate> {
    let cell = sheet.get_cell(row, column)?;
    let text = cell_text(cell);
    if text.is_empty() {
        return None;
    }
    match cell_date(cell) {
        Some(d) => Some(d),
        None => {
            errors.push(format!("tanggal tidak terbaca di kolom {column}: {text}"));
            None
        }
    }
}

/// The workbook publishes a run only when every checklist row reads Lolos.
fn checklist_all_lolos(
    wb: &mut Xlsx<std::io::BufReader<std::fs::File>>,
) -> Result<bool, ImportError> {
    let range = wb
        .worksheet_range(SH_CHECKLIST)
        .map_err(|e| ImportError::Workbook(format!("sheet {SH_CHECKLIST}: {e}")))?;
    let mut verdicts = Vec::new();
    for row in range.rows() {
        if let Some(cell) = row.get(1) {
            let text = cell_text(cell);
            if !text.is_empty() && text != "Lolos" {
                verdicts.push(text);
            }
        }
    }
    Ok(verdicts.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn date_text_parsing() {
        assert_eq!(
            parse_date_text("2027-01-31"),
            NaiveDate::from_ymd_opt(2027, 1, 31)
        );
        assert_eq!(
            parse_date_text("31/01/2027"),
            NaiveDate::from_ymd_opt(2027, 1, 31)
        );
        assert_eq!(
            parse_date_text("31 Jan 2027"),
            NaiveDate::from_ymd_opt(2027, 1, 31)
        );
        assert_eq!(parse_date_text("31/13/2026"), None);
        assert_eq!(parse_date_text("segera"), None);
    }

    #[test]
    fn excel_serial_dates() {
        assert_eq!(
            serial_to_date(46053.0),
            NaiveDate::from_ymd_opt(2026, 1, 31)
        );
        assert_eq!(serial_to_date(1.0), NaiveDate::from_ymd_opt(1899, 12, 31));
    }

    #[test]
    fn reads_the_real_workbook_when_present() {
        let path = Path::new("../processed/scholarship-master.xlsx");
        if !path.exists() {
            return;
        }
        let (records, _) = read(path).expect("workbook should parse");
        assert_eq!(records.len(), 1);
        let r = &records[0];
        assert_eq!(r.slug, "nl-scholarship");
        assert_eq!(r.levels, ["S1", "S2"]);
        assert_eq!(r.close_date, NaiveDate::from_ymd_opt(2027, 1, 31));
        assert_eq!(r.checked_at, NaiveDate::from_ymd_opt(2026, 11, 25));
        assert_eq!(r.benefits.len(), 6);
        assert_eq!(r.requirements.len(), 5);
        assert_eq!(r.documents.len(), 6);
        assert_eq!(r.steps.len(), 5);
        assert_eq!(r.timeline.len(), 4);
        assert_eq!(r.faqs.len(), 4);
        assert_eq!(r.related_names.len(), 3);
        // The shipped workbook is the template example, so it must be rejected.
        let today = NaiveDate::from_ymd_opt(2026, 11, 26).unwrap();
        let report = crate::import::validate::validate_set(&records, &[], today);
        assert!(!report.ok(), "CONTOH row must be rejected");
        assert!(
            report.rows[0]
                .errors
                .iter()
                .any(|e| e.contains("penanda contoh")),
            "errors: {:?}",
            report.rows[0].errors
        );
    }
}
