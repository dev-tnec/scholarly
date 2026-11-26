use askama::Template;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;

use super::{AppState, Html, server_error};
use crate::domain::scholarship::{
    DeadlineType, Status, countdown_days, detail_path, fmt_long, fmt_short, status, today_jakarta,
};
use crate::seo::jsonld;

#[derive(Template)]
#[template(path = "detail.html")]
pub struct DetailView {
    pub title: String,
    pub meta_description: String,
    pub canonical: String,
    pub robots: String,
    pub og_image: String,
    pub breadcrumb_jsonld: String,
    pub faq_jsonld: String,
    pub name: String,
    pub provider: String,
    pub levels: Vec<String>,
    pub levels_label: String,
    pub country: String,
    pub funding_type: String,
    pub status_label: String,
    pub status_class: String,
    pub paragraphs: Vec<String>,
    pub benefits: Vec<BenefitView>,
    pub requirements_umum: Vec<String>,
    pub requirements_akademik: Vec<String>,
    pub documents: Vec<String>,
    pub steps: Vec<StepView>,
    pub timeline: Vec<TimelineView>,
    pub tips: Vec<String>,
    pub faqs: Vec<FaqView>,
    pub checked_label: String,
    pub official_url: String,
    pub deadline_date_label: String,
    pub deadline_days_label: String,
    pub deadline_support: String,
    pub deadline_closed: bool,
    pub primary_label: String,
    pub primary_url: String,
    pub quick_duration: String,
    pub related: Vec<RelatedView>,
}

pub struct BenefitView {
    pub item: String,
    pub detail: String,
}

pub struct StepView {
    pub no: i32,
    pub title: String,
    pub detail: String,
}

pub struct TimelineView {
    pub stage: String,
    pub date_label: String,
    pub note: String,
    pub highlight: bool,
}

pub struct FaqView {
    pub question: String,
    pub answer: String,
}

pub struct RelatedView {
    pub url: String,
    pub name: String,
    pub meta: String,
    pub country: String,
}

#[derive(sqlx::FromRow)]
struct ScholarshipRow {
    id: i64,
    slug: String,
    name: String,
    provider: String,
    location: String,
    country: String,
    levels: Vec<String>,
    funding_type: String,
    close_date: Option<NaiveDate>,
    deadline_type: String,
    official_url: String,
    checked_at: NaiveDate,
    duration: String,
    title_tag: String,
    meta_description: String,
    notes: String,
}

#[derive(sqlx::FromRow)]
struct BenefitRow {
    item: String,
    detail: String,
}

#[derive(sqlx::FromRow)]
struct RequirementRow {
    group_name: String,
    text: String,
}

#[derive(sqlx::FromRow)]
struct DocumentRow {
    name: String,
}

#[derive(sqlx::FromRow)]
struct StepRow {
    step_no: i32,
    title: String,
    detail: String,
}

#[derive(sqlx::FromRow)]
struct TimelineRow {
    stage: String,
    event_date: Option<NaiveDate>,
    event_label: String,
    note: String,
}

#[derive(sqlx::FromRow)]
struct FaqRow {
    question: String,
    answer: String,
}

#[derive(sqlx::FromRow)]
struct RelatedRow {
    slug: String,
    name: String,
    country: String,
    levels: Vec<String>,
    close_date: Option<NaiveDate>,
}

#[axum::debug_handler]
pub async fn page(
    State(state): State<AppState>,
    uri: axum::http::Uri,
    Path(slug): Path<String>,
) -> Response {
    if !uri.path().ends_with('/') {
        return axum::response::Redirect::permanent(&format!("/{slug}/")).into_response();
    }
    match render(&state, &slug).await {
        Ok(response) => response,
        Err(e) => server_error("detail page", e),
    }
}

async fn render(state: &AppState, slug: &str) -> Result<Response, sqlx::Error> {
    let pool: &PgPool = &state.pool;
    let base = &state.config.base_url;
    let today = today_jakarta(Utc::now());

    let row: Option<ScholarshipRow> = sqlx::query_as(
        "SELECT id, slug, name, provider, location, country, levels, funding_type, close_date, \
         deadline_type, official_url, checked_at, duration, title_tag, meta_description, notes \
         FROM scholarships WHERE slug = $1 AND published",
    )
    .bind(slug)
    .fetch_optional(pool)
    .await?;
    let Some(row) = row else {
        return Ok(super::not_found_response());
    };

    let benefits: Vec<BenefitRow> = sqlx::query_as(
        "SELECT item, detail FROM benefits WHERE scholarship_id = $1 ORDER BY sort_order, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let requirements: Vec<RequirementRow> = sqlx::query_as(
        "SELECT group_name, text FROM requirements WHERE scholarship_id = $1 ORDER BY sort_order, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let documents: Vec<DocumentRow> = sqlx::query_as(
        "SELECT name FROM documents WHERE scholarship_id = $1 ORDER BY sort_order, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let steps: Vec<StepRow> = sqlx::query_as(
        "SELECT step_no, title, detail FROM steps WHERE scholarship_id = $1 ORDER BY step_no, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let timeline: Vec<TimelineRow> = sqlx::query_as(
        "SELECT stage, event_date, event_label, note FROM timeline_events WHERE scholarship_id = $1 ORDER BY sort_order, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let faqs: Vec<FaqRow> = sqlx::query_as(
        "SELECT question, answer FROM faqs WHERE scholarship_id = $1 ORDER BY sort_order, id",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;
    let related: Vec<RelatedRow> = sqlx::query_as(
        "SELECT rs.slug, rs.name, rs.country, rs.levels, rs.close_date FROM related rd \
         JOIN scholarships rs ON rs.id = rd.related_id \
         WHERE rd.scholarship_id = $1 AND rs.published ORDER BY rd.id LIMIT 3",
    )
    .bind(row.id)
    .fetch_all(pool)
    .await?;

    let deadline_type = DeadlineType::parse(&row.deadline_type);
    let st = status(row.close_date, deadline_type, today);
    let days = countdown_days(row.close_date, deadline_type, today);
    let closed = st == Status::Closed;

    let deadline_date_label = match row.close_date {
        Some(d) => fmt_short(d),
        None if deadline_type == DeadlineType::Rolling => "Sepanjang tahun".into(),
        None => "Belum diumumkan".into(),
    };
    let deadline_days_label = if closed {
        String::new()
    } else {
        days.map(|d| {
            if d == 0 {
                "Tutup hari ini".to_string()
            } else {
                format!("Sisa {d} hari")
            }
        })
        .unwrap_or_default()
    };
    let deadline_support = if closed {
        "Pendaftaran sudah ditutup.".to_string()
    } else if deadline_type == DeadlineType::Rolling {
        "Pendaftaran dibuka sepanjang tahun.".to_string()
    } else if row.close_date.is_none() {
        "Tanggal tutup belum diumumkan, cek situs resmi.".to_string()
    } else {
        String::new()
    };
    let (primary_label, primary_url) = if closed {
        ("Lihat Beasiswa Lainnya".to_string(), "/".to_string())
    } else {
        (
            "Daftar Lewat Situs Resmi".to_string(),
            row.official_url.clone(),
        )
    };

    let faq_pairs: Vec<(String, String)> = faqs
        .iter()
        .map(|f| (f.question.clone(), f.answer.clone()))
        .collect();
    let canonical = format!("{base}{}", detail_path(&row.slug));
    let indexable = (today - row.checked_at).num_days() <= 180;

    let view = DetailView {
        title: row.title_tag.clone(),
        meta_description: row.meta_description.clone(),
        canonical: canonical.clone(),
        robots: if indexable {
            "index, follow, max-image-preview:large, max-snippet:-1".into()
        } else {
            "noindex, follow".into()
        },
        og_image: format!("{base}/static/og/{}.png", row.slug),
        breadcrumb_jsonld: jsonld::breadcrumb(base, &row.name, &row.slug),
        faq_jsonld: jsonld::faq_page(&faq_pairs),
        name: row.name.clone(),
        provider: row.provider.clone(),
        levels: row.levels.clone(),
        levels_label: row.levels.join(", "),
        country: row.country.clone(),
        funding_type: row.funding_type.clone(),
        status_label: st.head_label().into(),
        status_class: st.css_class().into(),
        paragraphs: paragraphs(&row.notes),
        benefits: benefits
            .into_iter()
            .map(|b| BenefitView {
                item: b.item,
                detail: b.detail,
            })
            .collect(),
        requirements_umum: requirements
            .iter()
            .filter(|r| r.group_name == "Syarat Umum")
            .map(|r| r.text.clone())
            .collect(),
        requirements_akademik: requirements
            .iter()
            .filter(|r| r.group_name != "Syarat Umum")
            .map(|r| r.text.clone())
            .collect(),
        documents: documents.into_iter().map(|d| d.name).collect(),
        steps: steps
            .into_iter()
            .map(|s| StepView {
                no: s.step_no,
                title: s.title,
                detail: s.detail,
            })
            .collect(),
        timeline: timeline
            .into_iter()
            .map(|t| {
                let date_label = match t.event_date {
                    Some(d) => fmt_short(d),
                    None if !t.event_label.is_empty() => t.event_label.clone(),
                    None => "Belum dikonfirmasi".into(),
                };
                let lower = t.stage.to_lowercase();
                TimelineView {
                    highlight: lower.contains("tenggat") || lower.contains("deadline"),
                    stage: t.stage,
                    date_label,
                    note: t.note,
                }
            })
            .collect(),
        tips: tips(&row.location, &row.levels),
        faqs: faqs
            .into_iter()
            .map(|f| FaqView {
                question: f.question,
                answer: f.answer,
            })
            .collect(),
        checked_label: fmt_long(row.checked_at),
        official_url: row.official_url.clone(),
        deadline_date_label,
        deadline_days_label,
        deadline_support,
        deadline_closed: closed,
        primary_label,
        primary_url,
        quick_duration: if row.duration.trim().is_empty() {
            "Cek situs resmi".into()
        } else {
            row.duration.clone()
        },
        related: related
            .into_iter()
            .map(|r| {
                let levels = r.levels.join(", ");
                let meta = match r.close_date {
                    Some(d) => format!("{levels}, tutup {}", month_year(d)),
                    None => levels,
                };
                RelatedView {
                    url: detail_path(&r.slug),
                    name: r.name,
                    meta,
                    country: r.country,
                }
            })
            .collect(),
    };
    Ok(Html(view).into_response())
}

fn paragraphs(notes: &str) -> Vec<String> {
    let mut parts: Vec<String> = notes
        .split("\n\n")
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect();
    if parts.len() == 1 && parts[0].contains('\n') {
        parts = parts[0]
            .split('\n')
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .collect();
    }
    parts
}

fn month_year(date: NaiveDate) -> String {
    fmt_short(date)
        .split_once(' ')
        .map(|(_, rest)| rest.to_string())
        .unwrap_or_default()
}

/// Guidance copy per content-master-template variants 4.2 and 4.3.
/// The workbook has no tips sheet, so the three paragraphs are composed from
/// the record's location and level, never from invented facts.
fn tips(location: &str, levels: &[String]) -> Vec<String> {
    let abroad = location == "Luar Negeri";
    let doctoral = levels.iter().any(|l| l == "S3");

    let first = if abroad {
        "Banyak pendaftar gugur di tahap administrasi karena berkas tidak lengkap, transkrip belum dilegalisir, atau esai yang terlalu generik. Baca dua kali daftar dokumen sebelum unggah, lalu sebutkan alasan spesifik kamu memilih program studi: modul kuliah, laboratorium, atau dosen yang relevan dengan rencana studimu."
    } else {
        "Banyak pendaftar gugur karena berkas tidak sesuai format dan esai yang terlalu umum. Baca dua kali daftar dokumen sebelum unggah, lalu jelaskan rencana studi yang konkret dan alasan kamu memilih kampus tujuan."
    };

    let second = if abroad && doctoral {
        "Untuk jenjang S3, siapkan proposal penelitian dan konfirmasi calon pembimbing lebih awal. Skor bahasa juga butuh waktu, jadi mulai persiapan IELTS atau TOEFL minimal enam bulan sebelum tenggat supaya kamu masih bisa ujian ulang."
    } else if abroad {
        "Siapkan IELTS atau TOEFL minimal enam bulan sebelum tenggat. Sertifikat bahasa sering menjadi bagian yang paling lama, dan kamu perlu waktu untuk mengulang tes bila skor pertama belum memenuhi syarat program."
    } else {
        "Kalau penyelenggara mensyaratkan skor bahasa Inggris, siapkan tes minimal tiga bulan sebelum tenggat dan cek format tes yang diterima. Jangan menunggu pengumuman resmi untuk mulai belajar."
    };

    let third = "Kalau kamu bingung memilih program atau menyusun esai motivasi, program Scholarship Mentoring Titik Nol mendampingi kamu dari riset program sampai dokumen siap kirim. Tanyakan ke Minol bila kamu ingin tahu langkah pertama yang sesuai kondisimu.";

    vec![first.to_string(), second.to_string(), third.to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paragraphs_split_on_blank_line() {
        let out = paragraphs("Satu.\n\nDua.\n\nTiga.");
        assert_eq!(out, ["Satu.", "Dua.", "Tiga."]);
    }

    #[test]
    fn paragraphs_split_on_single_newlines() {
        let out = paragraphs("Satu.\nDua.");
        assert_eq!(out, ["Satu.", "Dua."]);
    }

    #[test]
    fn tips_variant_counts_and_banned_words() {
        for (location, levels) in [
            ("Luar Negeri", vec!["S1".to_string()]),
            ("Dalam Negeri", vec!["S2".to_string()]),
            ("Luar Negeri", vec!["S3".to_string()]),
        ] {
            let t = tips(location, &levels);
            assert_eq!(t.len(), 3);
            for p in &t {
                for banned in [
                    "tersebut",
                    "merupakan",
                    "dijamin",
                    "pasti",
                    "nggak",
                    "banget",
                    "gimana",
                    "Admin",
                ] {
                    assert!(!p.contains(banned), "{banned} in {p}");
                }
            }
        }
    }

    #[test]
    fn month_year_short() {
        let d = NaiveDate::from_ymd_opt(2026, 11, 5).unwrap();
        assert_eq!(month_year(d), "Nov 2026");
    }
}
