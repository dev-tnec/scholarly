pub mod validate;
pub mod workbook;

use chrono::{NaiveDate, Utc};
use serde::Serialize;
use sqlx::PgPool;
use std::collections::HashMap;
use std::path::Path;

use crate::domain::scholarship::{DeadlineType, today_jakarta};
use crate::routes::AppState;

#[derive(Clone, Debug, Default)]
pub struct Record {
    pub ext_id: String,
    pub slug: String,
    pub name: String,
    pub official_name: String,
    pub abbreviation: String,
    pub provider: String,
    pub provider_type: String,
    pub location: String,
    pub country: String,
    pub levels: Vec<String>,
    pub funding_type: String,
    pub open_date: Option<NaiveDate>,
    pub close_date: Option<NaiveDate>,
    pub deadline_type: DeadlineType,
    pub official_url: String,
    pub checked_at: Option<NaiveDate>,
    pub quota: String,
    pub duration: String,
    pub language_requirement: String,
    pub visa_requirement: String,
    pub research_proposal: String,
    pub title_tag: String,
    pub meta_description: String,
    pub notes: String,
    pub published: bool,
    pub benefits: Vec<ChildRow>,
    pub requirements: Vec<RequirementRow>,
    pub documents: Vec<DocumentRow>,
    pub steps: Vec<StepRow>,
    pub timeline: Vec<TimelineRow>,
    pub faqs: Vec<FaqRow>,
    pub related_names: Vec<String>,
    /// Related links that exist in the store but are missing or unpublished.
    pub related_missing: usize,
}

#[derive(Clone, Debug, Default)]
pub struct ChildRow {
    pub item: String,
    pub detail: String,
    pub source_url: String,
}

#[derive(Clone, Debug, Default)]
pub struct RequirementRow {
    pub group_name: String,
    pub text: String,
    pub source_url: String,
}

#[derive(Clone, Debug, Default)]
pub struct DocumentRow {
    pub name: String,
    pub required: bool,
    pub source_url: String,
}

#[derive(Clone, Debug, Default)]
pub struct StepRow {
    pub step_no: i64,
    pub title: String,
    pub detail: String,
}

#[derive(Clone, Debug, Default)]
pub struct TimelineRow {
    pub stage: String,
    pub event_date: Option<NaiveDate>,
    pub event_label: String,
    pub note: String,
}

#[derive(Clone, Debug, Default)]
pub struct FaqRow {
    pub question: String,
    pub answer: String,
}

impl Record {
    /// All parent text fields, for the [contoh data] marker scan.
    pub fn text_fields(&self) -> Vec<&str> {
        vec![
            &self.ext_id,
            &self.slug,
            &self.name,
            &self.official_name,
            &self.abbreviation,
            &self.provider,
            &self.provider_type,
            &self.location,
            &self.country,
            &self.funding_type,
            &self.official_url,
            &self.quota,
            &self.duration,
            &self.language_requirement,
            &self.visa_requirement,
            &self.research_proposal,
            &self.title_tag,
            &self.meta_description,
            &self.notes,
        ]
    }
}

#[derive(Debug, Serialize)]
pub struct RowReport {
    pub id: String,
    pub name: String,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct ImportReport {
    pub rows: Vec<RowReport>,
}

impl ImportReport {
    pub fn ok(&self) -> bool {
        self.rows.iter().all(|r| r.errors.is_empty())
    }

    pub fn print(&self) {
        for row in &self.rows {
            let head = format!(
                "[{}] {}",
                row.id,
                if row.name.is_empty() {
                    "(tanpa nama)"
                } else {
                    &row.name
                }
            );
            if !row.errors.is_empty() {
                for e in &row.errors {
                    println!("ERROR {head}: {e}");
                }
            }
            for w in &row.warnings {
                println!("peringatan {head}: {w}");
            }
        }
        let failed = self.rows.iter().filter(|r| !r.errors.is_empty()).count();
        println!(
            "import: {} baris diperiksa, {} gagal",
            self.rows.len(),
            failed
        );
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ImportError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("workbook: {0}")]
    Workbook(String),
    #[error("database: {0}")]
    Db(#[from] sqlx::Error),
    #[error("validation failed, tidak ada baris yang ditulis")]
    ValidationFailed,
}

/// `scholarly import`: read, validate, and write the workbook in one transaction.
pub async fn run(state: &AppState, file: &Path, dry_run: bool) -> Result<(), ImportError> {
    let (records, parse_rows) = workbook::read(file)?;
    let today = today_jakarta(Utc::now());
    let redirects = redirect_paths(&state.pool).await?;
    let mut report = validate::validate_set(&records, &redirects, today);
    report.rows.extend(parse_rows);
    report.print();

    if dry_run {
        println!("dry run: tidak ada yang ditulis ke database");
        return Ok(());
    }
    if !report.ok() {
        return Err(ImportError::ValidationFailed);
    }
    write_all(state, &records, file).await?;
    println!("import selesai: {} beasiswa ditulis", records.len());
    Ok(())
}

/// `scholarly check`: re-run the same validation against the live database.
pub async fn check(state: &AppState) -> Result<ImportReport, ImportError> {
    let records = records_from_db(&state.pool).await?;
    let redirects = redirect_paths(&state.pool).await?;
    let today = today_jakarta(Utc::now());
    Ok(validate::validate_set(&records, &redirects, today))
}

async fn redirect_paths(pool: &PgPool) -> Result<Vec<String>, sqlx::Error> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT from_path FROM redirects")
        .fetch_all(pool)
        .await?;
    Ok(rows.into_iter().map(|r| r.0).collect())
}

async fn write_all(state: &AppState, records: &[Record], file: &Path) -> Result<(), ImportError> {
    let started = Utc::now();
    let mut tx = state.pool.begin().await?;

    // The workbook is the editing surface: full replace keeps removed rows gone.
    sqlx::query("DELETE FROM scholarships")
        .execute(&mut *tx)
        .await?;

    let mut ids = HashMap::new();
    for r in records {
        let (id,): (i64,) = sqlx::query_as(
            "INSERT INTO scholarships (slug, name, official_name, abbreviation, provider, \
             provider_type, location, country, levels, funding_type, open_date, close_date, \
             deadline_type, official_url, checked_at, quota, duration, language_requirement, \
             visa_requirement, research_proposal, title_tag, meta_description, notes, published) \
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24) \
             RETURNING id",
        )
        .bind(&r.slug)
        .bind(&r.name)
        .bind(&r.official_name)
        .bind(&r.abbreviation)
        .bind(&r.provider)
        .bind(&r.provider_type)
        .bind(&r.location)
        .bind(&r.country)
        .bind(&r.levels)
        .bind(&r.funding_type)
        .bind(r.open_date)
        .bind(r.close_date)
        .bind(r.deadline_type.as_str())
        .bind(&r.official_url)
        .bind(r.checked_at)
        .bind(&r.quota)
        .bind(&r.duration)
        .bind(&r.language_requirement)
        .bind(&r.visa_requirement)
        .bind(&r.research_proposal)
        .bind(&r.title_tag)
        .bind(&r.meta_description)
        .bind(&r.notes)
        .bind(r.published)
        .fetch_one(&mut *tx)
        .await?;
        ids.insert(r.ext_id.clone(), id);
    }

    for r in records {
        let id = ids[&r.ext_id];
        for (i, b) in r.benefits.iter().enumerate() {
            sqlx::query("INSERT INTO benefits (scholarship_id, item, detail, source_url, sort_order) VALUES ($1,$2,$3,$4,$5)")
                .bind(id).bind(&b.item).bind(&b.detail).bind(&b.source_url).bind(i as i32)
                .execute(&mut *tx).await?;
        }
        for (i, q) in r.requirements.iter().enumerate() {
            sqlx::query("INSERT INTO requirements (scholarship_id, group_name, text, source_url, sort_order) VALUES ($1,$2,$3,$4,$5)")
                .bind(id).bind(&q.group_name).bind(&q.text).bind(&q.source_url).bind(i as i32)
                .execute(&mut *tx).await?;
        }
        for (i, d) in r.documents.iter().enumerate() {
            sqlx::query("INSERT INTO documents (scholarship_id, name, required, source_url, sort_order) VALUES ($1,$2,$3,$4,$5)")
                .bind(id).bind(&d.name).bind(d.required as i32).bind(&d.source_url).bind(i as i32)
                .execute(&mut *tx).await?;
        }
        for s in &r.steps {
            sqlx::query(
                "INSERT INTO steps (scholarship_id, step_no, title, detail) VALUES ($1,$2,$3,$4)",
            )
            .bind(id)
            .bind(s.step_no)
            .bind(&s.title)
            .bind(&s.detail)
            .execute(&mut *tx)
            .await?;
        }
        for (i, t) in r.timeline.iter().enumerate() {
            sqlx::query("INSERT INTO timeline_events (scholarship_id, stage, event_date, event_label, note, sort_order) VALUES ($1,$2,$3,$4,$5,$6)")
                .bind(id).bind(&t.stage).bind(t.event_date).bind(&t.event_label).bind(&t.note).bind(i as i32)
                .execute(&mut *tx).await?;
        }
        for (i, f) in r.faqs.iter().enumerate() {
            sqlx::query("INSERT INTO faqs (scholarship_id, question, answer, sort_order) VALUES ($1,$2,$3,$4)")
                .bind(id).bind(&f.question).bind(&f.answer).bind(i as i32)
                .execute(&mut *tx).await?;
        }
        for name in &r.related_names {
            let target: Option<(i64,)> = sqlx::query_as(
                "SELECT id FROM scholarships WHERE name = $1 OR official_name = $1 OR slug = $1",
            )
            .bind(name)
            .fetch_optional(&mut *tx)
            .await?;
            if let Some((related_id,)) = target {
                sqlx::query("INSERT INTO related (scholarship_id, related_id) VALUES ($1,$2)")
                    .bind(id)
                    .bind(related_id)
                    .execute(&mut *tx)
                    .await?;
            }
        }
    }

    let report_rows: Vec<RowReport> = records
        .iter()
        .map(|r| RowReport {
            id: r.ext_id.clone(),
            name: r.name.clone(),
            errors: vec![],
            warnings: vec![],
        })
        .collect();
    let report_json = serde_json::to_value(&report_rows).unwrap_or(serde_json::json!([]));
    sqlx::query(
        "INSERT INTO import_runs (started_at, finished_at, source_file, rows_ok, rows_failed, report_json) \
         VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(started)
    .bind(Utc::now())
    .bind(file.to_string_lossy().to_string())
    .bind(records.len() as i32)
    .bind(0)
    .bind(report_json)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    state.reload_redirects().await;
    Ok(())
}

#[derive(sqlx::FromRow)]
struct DbRow {
    slug: String,
    name: String,
    official_name: String,
    abbreviation: String,
    provider: String,
    provider_type: String,
    location: String,
    country: String,
    levels: Vec<String>,
    funding_type: String,
    open_date: Option<NaiveDate>,
    close_date: Option<NaiveDate>,
    deadline_type: String,
    official_url: String,
    checked_at: Option<NaiveDate>,
    title_tag: String,
    meta_description: String,
    notes: String,
    published: bool,
    n_benefits: i64,
    n_requirements: i64,
    n_documents: i64,
    n_steps: i64,
    n_timeline: i64,
    n_faqs: i64,
    n_related: i64,
}

async fn records_from_db(pool: &PgPool) -> Result<Vec<Record>, ImportError> {
    let rows: Vec<DbRow> = sqlx::query_as(
        "SELECT s.slug, s.name, s.official_name, s.abbreviation, s.provider, s.provider_type, \
         s.location, s.country, s.levels, s.funding_type, s.open_date, s.close_date, \
         s.deadline_type, s.official_url, s.checked_at, s.title_tag, s.meta_description, s.notes, s.published, \
         (SELECT count(*) FROM benefits b WHERE b.scholarship_id = s.id) AS n_benefits, \
         (SELECT count(*) FROM requirements rq WHERE rq.scholarship_id = s.id) AS n_requirements, \
         (SELECT count(*) FROM documents d WHERE d.scholarship_id = s.id) AS n_documents, \
         (SELECT count(*) FROM steps st WHERE st.scholarship_id = s.id) AS n_steps, \
         (SELECT count(*) FROM timeline_events t WHERE t.scholarship_id = s.id) AS n_timeline, \
         (SELECT count(*) FROM faqs f WHERE f.scholarship_id = s.id) AS n_faqs, \
         (SELECT count(*) FROM related rd WHERE rd.scholarship_id = s.id) AS n_related \
         FROM scholarships s",
    )
    .fetch_all(pool)
    .await?;

    let related: Vec<(String, String)> = sqlx::query_as(
        "SELECT s.slug, rs.name FROM related rd \
         JOIN scholarships s ON s.id = rd.scholarship_id \
         JOIN scholarships rs ON rs.id = rd.related_id WHERE rs.published",
    )
    .fetch_all(pool)
    .await?;

    let mut related_map: HashMap<String, Vec<String>> = HashMap::new();
    for (slug, name) in related {
        related_map.entry(slug).or_default().push(name);
    }

    Ok(rows
        .into_iter()
        .map(|r| {
            let mut published_related = related_map.get(&r.slug).cloned().unwrap_or_default();
            published_related.truncate(3);
            let missing = (r.n_related as usize).saturating_sub(published_related.len());
            Record {
                ext_id: r.slug.clone(),
                slug: r.slug,
                name: r.name,
                official_name: r.official_name,
                abbreviation: r.abbreviation,
                provider: r.provider,
                provider_type: r.provider_type,
                location: r.location,
                country: r.country,
                levels: r.levels,
                funding_type: r.funding_type,
                open_date: r.open_date,
                close_date: r.close_date,
                deadline_type: DeadlineType::parse(&r.deadline_type),
                official_url: r.official_url,
                checked_at: r.checked_at,
                title_tag: r.title_tag,
                meta_description: r.meta_description,
                notes: r.notes,
                published: r.published,
                benefits: vec![ChildRow::default(); r.n_benefits as usize],
                requirements: vec![RequirementRow::default(); r.n_requirements as usize],
                documents: vec![DocumentRow::default(); r.n_documents as usize],
                steps: vec![StepRow::default(); r.n_steps as usize],
                timeline: vec![TimelineRow::default(); r.n_timeline as usize],
                faqs: vec![FaqRow::default(); r.n_faqs as usize],
                related_names: published_related,
                related_missing: missing,
                ..Record::default()
            }
        })
        .collect())
}
