use chrono::NaiveDate;
use std::collections::HashSet;

use super::{ImportReport, Record, RowReport};

pub const TITLE_MAX: usize = 60;
pub const META_MIN: usize = 120;
pub const META_MAX: usize = 155;
pub const FRESH_DAYS: i64 = 90;
pub const REVIEW_WINDOW_DAYS: i64 = 14;
pub const DEADLINE_WARNING_DAYS: i64 = 7;

/// PRD section 5 slug constraint, reused by validation.
pub fn slug_ok(slug: &str) -> bool {
    !slug.is_empty()
        && slug
            .bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

pub fn title_ok(title: &str) -> bool {
    !title.is_empty() && title.chars().count() <= TITLE_MAX
}

pub fn meta_ok(meta: &str) -> bool {
    let len = meta.chars().count();
    (META_MIN..=META_MAX).contains(&len)
}

pub fn has_sample_marker(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("[contoh data]") || lower.contains("contoh-")
}

fn is_https(url: &str) -> bool {
    url.starts_with("https://")
}

pub fn validate_set(
    records: &[Record],
    redirect_paths: &[String],
    today: NaiveDate,
) -> ImportReport {
    let mut seen_slugs: HashSet<&str> = HashSet::new();
    let redirects: HashSet<&str> = redirect_paths.iter().map(|s| s.as_str()).collect();

    let mut rows = Vec::with_capacity(records.len());
    for record in records {
        let mut row = RowReport {
            id: record.ext_id.clone(),
            name: record.name.clone(),
            errors: Vec::new(),
            warnings: Vec::new(),
        };
        validate_record(record, today, &mut row);

        if !record.slug.is_empty() {
            if !seen_slugs.insert(record.slug.as_str()) {
                row.errors
                    .push(format!("slug duplikat di workbook: {}", record.slug));
            }
            if redirects.contains(record.slug.as_str()) {
                row.errors.push(format!(
                    "slug bertabrakan dengan sumber redirect: {}",
                    record.slug
                ));
            }
        }

        if record.related_names.len() < 3 {
            row.errors.push(format!(
                "beasiswa terkait kurang dari 3: {}",
                record.related_names.len()
            ));
        }
        if record.related_missing > 0 {
            row.errors.push(format!(
                "{} tautan beasiswa terkait menunjuk beasiswa yang hilang atau belum diterbitkan",
                record.related_missing
            ));
        }
        for name in &record.related_names {
            let found = records
                .iter()
                .any(|r| r.slug == *name || r.name == *name || r.official_name == *name);
            if !found {
                row.errors
                    .push(format!("beasiswa terkait tidak ditemukan: {name}"));
            }
        }
        rows.push(row);
    }
    ImportReport { rows }
}

pub fn validate_record(record: &Record, today: NaiveDate, row: &mut RowReport) {
    let required = [
        ("Nama Tampilan", &record.name),
        ("Nama Resmi", &record.official_name),
        ("Slug", &record.slug),
        ("Penyelenggara", &record.provider),
        ("Tipe Penyelenggara", &record.provider_type),
        ("Lokasi", &record.location),
        ("Negara", &record.country),
        ("Pendanaan", &record.funding_type),
        ("URL Resmi", &record.official_url),
        ("Title Tag", &record.title_tag),
        ("Meta Description", &record.meta_description),
        ("Catatan", &record.notes),
    ];
    for (label, value) in required {
        if value.trim().is_empty() {
            row.errors.push(format!("field wajib kosong: {label}"));
        }
    }
    if record.levels.is_empty() {
        row.errors.push("field wajib kosong: Jenjang".into());
    }
    let Some(checked_at) = record.checked_at else {
        row.errors.push("field wajib kosong: Tanggal Cek".into());
        return;
    };

    if !slug_ok(&record.slug) && !record.slug.is_empty() {
        row.errors.push(format!(
            "slug tidak valid (harus ^[a-z0-9-]+$): {}",
            record.slug
        ));
    }
    if !is_https(&record.official_url) && !record.official_url.is_empty() {
        row.errors.push("URL Resmi bukan https".into());
    }
    if checked_at > today {
        row.errors.push("Tanggal Cek berada di masa depan".into());
    }
    if record.open_date.is_none()
        && record.deadline_type != crate::domain::scholarship::DeadlineType::Rolling
    {
        // Providers often publish only the close date, so this stays a warning.
        row.warnings.push("Tanggal Buka kosong".into());
    }
    if record.deadline_type == crate::domain::scholarship::DeadlineType::Fixed
        && record.close_date.is_none()
    {
        row.errors
            .push("Tanggal Tutup kosong untuk deadline tetap".into());
    }
    if let (Some(open), Some(close)) = (record.open_date, record.close_date)
        && close < open
    {
        row.errors.push("Tanggal Tutup sebelum Tanggal Buka".into());
    }

    let mut texts: Vec<&str> = record.text_fields();
    for b in &record.benefits {
        texts.push(&b.item);
        texts.push(&b.detail);
        texts.push(&b.source_url);
    }
    for q in &record.requirements {
        texts.push(&q.text);
        texts.push(&q.source_url);
    }
    for d in &record.documents {
        texts.push(&d.name);
        texts.push(&d.source_url);
    }
    for s in &record.steps {
        texts.push(&s.title);
        texts.push(&s.detail);
    }
    for t in &record.timeline {
        texts.push(&t.stage);
        texts.push(&t.note);
        texts.push(&t.event_label);
    }
    for f in &record.faqs {
        texts.push(&f.question);
        texts.push(&f.answer);
    }
    if texts.iter().any(|t| has_sample_marker(t)) {
        row.errors
            .push("penanda contoh masih ada: [contoh data] atau CONTOH-".into());
    }

    for url in record
        .benefits
        .iter()
        .map(|b| &b.source_url)
        .chain(record.requirements.iter().map(|q| &q.source_url))
        .chain(record.documents.iter().map(|d| &d.source_url))
    {
        if !url.trim().is_empty() && !is_https(url) {
            row.errors.push(format!("URL sumber bukan https: {url}"));
        }
    }

    if !title_ok(&record.title_tag) && !record.title_tag.is_empty() {
        row.errors.push(format!(
            "Title Tag {} karakter, maksimal {TITLE_MAX}",
            record.title_tag.chars().count()
        ));
    }
    if !meta_ok(&record.meta_description) && !record.meta_description.is_empty() {
        row.errors.push(format!(
            "Meta Description {} karakter, harus {META_MIN} sampai {META_MAX}",
            record.meta_description.chars().count()
        ));
    }

    count_rule(
        "FAQ",
        record.faqs.len(),
        4,
        Some(4),
        &mut row.errors,
        &mut row.warnings,
    );
    count_rule(
        "Keuntungan",
        record.benefits.len(),
        3,
        Some(8),
        &mut row.errors,
        &mut row.warnings,
    );
    count_rule(
        "Persyaratan",
        record.requirements.len(),
        5,
        Some(9),
        &mut row.errors,
        &mut row.warnings,
    );
    count_rule(
        "Dokumen",
        record.documents.len(),
        4,
        Some(8),
        &mut row.errors,
        &mut row.warnings,
    );
    count_rule(
        "Cara Mendaftar",
        record.steps.len(),
        4,
        Some(6),
        &mut row.errors,
        &mut row.warnings,
    );
    count_rule(
        "Jadwal",
        record.timeline.len(),
        3,
        Some(6),
        &mut row.errors,
        &mut row.warnings,
    );

    if record.quota.trim().is_empty() {
        row.warnings.push("field opsional kosong: Kuota".into());
    }
    if record.duration.trim().is_empty() {
        row.warnings.push("field opsional kosong: Durasi".into());
    }
    if record.language_requirement.trim().is_empty() {
        row.warnings
            .push("field opsional kosong: Syarat Bahasa".into());
    }
    if record.visa_requirement.trim().is_empty() {
        row.warnings
            .push("field opsional kosong: Visa/Paspor".into());
    }

    let age = (today - checked_at).num_days();
    if age > FRESH_DAYS {
        row.warnings.push(format!(
            "Tanggal Cek lebih dari {FRESH_DAYS} hari lalu ({age} hari)"
        ));
    }
    if let Some(close) = record.close_date {
        let to_close = (close - today).num_days();
        if (0..=DEADLINE_WARNING_DAYS).contains(&to_close)
            && (today - checked_at).num_days() > REVIEW_WINDOW_DAYS
        {
            row.warnings
                .push("deadline dalam 7 hari tapi data belum ditinjau ulang".into());
        }
    }
    if record.provider_type.eq_ignore_ascii_case("lainnya")
        || record.country.eq_ignore_ascii_case("lainnya")
    {
        row.warnings
            .push("kategori lainnya dipakai, cek ulang klasifikasi".into());
    }
}

fn count_rule(
    label: &str,
    count: usize,
    min: usize,
    max: Option<usize>,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if count < min {
        errors.push(format!("{label} kurang dari {min} baris: {count}"));
    } else if let Some(max) = max
        && count > max
    {
        warnings.push(format!("{label} lebih dari {max} baris: {count}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::scholarship::DeadlineType;
    use crate::import::{ChildRow, DocumentRow, FaqRow, RequirementRow, StepRow, TimelineRow};

    fn valid_record() -> Record {
        let date = |y, m, d| NaiveDate::from_ymd_opt(y, m, d).unwrap();
        Record {
            ext_id: "SCH-001".into(),
            slug: "nl-scholarship".into(),
            name: "NL Scholarship".into(),
            official_name: "NL Scholarship".into(),
            abbreviation: "NLS".into(),
            provider: "Nuffic Neso Indonesia".into(),
            provider_type: "Yayasan".into(),
            location: "Luar Negeri".into(),
            country: "Belanda".into(),
            levels: vec!["S1".into(), "S2".into()],
            funding_type: "Fully Funded".into(),
            open_date: Some(date(2026, 11, 1)),
            close_date: Some(date(2027, 1, 31)),
            deadline_type: DeadlineType::Fixed,
            official_url: "https://www.nesoindonesia.or.id".into(),
            checked_at: Some(date(2026, 11, 25)),
            quota: String::new(),
            duration: "Tahun pertama studi".into(),
            language_requirement: "IELTS 6.5".into(),
            visa_requirement: "Ya".into(),
            research_proposal: "Ya untuk S2".into(),
            title_tag: "NL Scholarship 2027: Syarat, Cara Daftar, dan Deadline".into(),
            meta_description: "Biaya kuliah penuh, uang saku bulanan, tiket pesawat, dan asuransi untuk studi S1 dan S2 di Belanda. Cek syarat, dokumen, dan deadline di sini.".into(),
            notes: "Ringkasan contoh yang cukup panjang untuk satu paragraf.".into(),
            published: true,
            benefits: (0..4).map(|i| ChildRow { item: format!("Item {i}"), detail: "Detail".into(), source_url: String::new() }).collect(),
            requirements: (0..5).map(|i| RequirementRow { group_name: "Syarat Umum".into(), text: format!("Syarat {i}"), source_url: String::new() }).collect(),
            documents: (0..4).map(|i| DocumentRow { name: format!("Dokumen {i}"), required: true, source_url: String::new() }).collect(),
            steps: (1..=4).map(|i| StepRow { step_no: i, title: format!("Langkah {i}"), detail: "Detail".into() }).collect(),
            timeline: (0..3).map(|i| TimelineRow { stage: format!("Tahap {i}"), event_date: None, event_label: "Feb 2027".into(), note: String::new() }).collect(),
            faqs: (0..4).map(|i| FaqRow { question: format!("Pertanyaan {i}"), answer: "Jawaban".into() }).collect(),
            related_names: vec!["Chevening".into(), "MEXT".into(), "GKS".into()],
            related_missing: 0,
        }
    }

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 11, 26).unwrap()
    }

    fn errors_for(mutate: impl FnOnce(&mut Record)) -> Vec<String> {
        let mut r = valid_record();
        mutate(&mut r);
        let mut row = RowReport {
            id: r.ext_id.clone(),
            name: r.name.clone(),
            errors: vec![],
            warnings: vec![],
        };
        validate_record(&r, today(), &mut row);
        row.errors
    }

    fn assert_error(mutate: impl FnOnce(&mut Record), needle: &str) {
        let errors = errors_for(mutate);
        assert!(
            errors.iter().any(|e| e.contains(needle)),
            "expected error containing {needle:?}, got {errors:?}"
        );
    }

    #[test]
    fn valid_fixture_has_no_errors() {
        assert_eq!(errors_for(|_| {}), Vec::<String>::new());
    }

    #[test]
    fn missing_required_field() {
        assert_error(|r| r.name.clear(), "field wajib kosong: Nama Tampilan");
    }

    #[test]
    fn close_before_open() {
        assert_error(
            |r| r.close_date = Some(NaiveDate::from_ymd_opt(2026, 10, 1).unwrap()),
            "Tanggal Tutup sebelum Tanggal Buka",
        );
    }

    #[test]
    fn url_not_https() {
        assert_error(
            |r| r.official_url = "http://example.com".into(),
            "URL Resmi bukan https",
        );
    }

    #[test]
    fn source_url_not_https() {
        assert_error(
            |r| r.benefits[0].source_url = "http://example.com".into(),
            "URL sumber bukan https",
        );
    }

    #[test]
    fn slug_format() {
        assert_error(|r| r.slug = "NL_Scholarship".into(), "slug tidak valid");
        assert!(slug_ok("nl-scholarship-2027"));
        assert!(!slug_ok(""));
        assert!(!slug_ok("nl scholarship"));
    }

    #[test]
    fn faq_count() {
        assert_error(|r| r.faqs.truncate(3), "FAQ kurang dari 4");
    }

    #[test]
    fn benefits_count() {
        assert_error(|r| r.benefits.truncate(2), "Keuntungan kurang dari 3");
    }

    #[test]
    fn requirements_count() {
        assert_error(|r| r.requirements.truncate(4), "Persyaratan kurang dari 5");
    }

    #[test]
    fn documents_count() {
        assert_error(|r| r.documents.truncate(3), "Dokumen kurang dari 4");
    }

    #[test]
    fn steps_count() {
        assert_error(|r| r.steps.truncate(3), "Cara Mendaftar kurang dari 4");
    }

    #[test]
    fn timeline_count() {
        assert_error(|r| r.timeline.truncate(2), "Jadwal kurang dari 3");
    }

    #[test]
    fn meta_description_bounds() {
        assert_error(
            |r| r.meta_description = "x".repeat(119),
            "Meta Description 119 karakter",
        );
        assert_error(
            |r| r.meta_description = "x".repeat(156),
            "Meta Description 156 karakter",
        );
        assert!(meta_ok(&"x".repeat(120)));
        assert!(meta_ok(&"x".repeat(155)));
        assert!(!meta_ok(&"x".repeat(156)));
    }

    #[test]
    fn title_tag_boundary() {
        assert!(title_ok(&"x".repeat(60)));
        assert!(!title_ok(&"x".repeat(61)));
        assert_error(|r| r.title_tag = "x".repeat(61), "Title Tag 61 karakter");
    }

    #[test]
    fn sample_markers_rejected() {
        assert_error(|r| r.notes = "Isi [contoh data]".into(), "penanda contoh");
        assert_error(|r| r.ext_id = "CONTOH-NL".into(), "penanda contoh");
        assert_error(
            |r| r.faqs[0].answer = "Jawaban [CONTOH DATA]".into(),
            "penanda contoh",
        );
    }

    #[test]
    fn checked_at_not_in_future() {
        assert_error(
            |r| r.checked_at = Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap()),
            "Tanggal Cek berada di masa depan",
        );
    }

    #[test]
    fn related_names_must_exist() {
        let mut r = valid_record();
        r.related_names = vec!["Tidak Ada".into()];
        let report = validate_set(&[r], &[], today());
        assert!(
            report.rows[0]
                .errors
                .iter()
                .any(|e| e.contains("tidak ditemukan"))
        );
    }

    #[test]
    fn slug_collision_with_redirect() {
        let report = validate_set(&[valid_record()], &["nl-scholarship".to_string()], today());
        assert!(
            report.rows[0]
                .errors
                .iter()
                .any(|e| e.contains("sumber redirect"))
        );
    }

    #[test]
    fn duplicate_slug_in_workbook() {
        let a = valid_record();
        let mut b = valid_record();
        b.ext_id = "SCH-002".into();
        b.name = "NL Scholarship Dua".into();
        let report = validate_set(&[a, b], &[], today());
        assert!(
            report.rows[1]
                .errors
                .iter()
                .any(|e| e.contains("slug duplikat"))
        );
    }

    #[test]
    fn stale_checked_at_warns_not_blocks() {
        let mut r = valid_record();
        r.checked_at = Some(NaiveDate::from_ymd_opt(2026, 6, 1).unwrap());
        let mut row = RowReport {
            id: r.ext_id.clone(),
            name: r.name.clone(),
            errors: vec![],
            warnings: vec![],
        };
        validate_record(&r, today(), &mut row);
        assert!(row.errors.is_empty());
        assert!(
            row.warnings
                .iter()
                .any(|w| w.contains("lebih dari 90 hari"))
        );
    }

    #[test]
    fn unreviewed_deadline_warns() {
        let mut r = valid_record();
        r.checked_at = Some(NaiveDate::from_ymd_opt(2026, 11, 1).unwrap());
        r.close_date = Some(NaiveDate::from_ymd_opt(2026, 12, 1).unwrap());
        let mut row = RowReport {
            id: r.ext_id.clone(),
            name: r.name.clone(),
            errors: vec![],
            warnings: vec![],
        };
        validate_record(&r, today(), &mut row);
        assert!(row.warnings.iter().any(|w| w.contains("belum ditinjau")));
    }
}
