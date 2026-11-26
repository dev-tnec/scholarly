use askama::Template;
use axum::extract::{RawQuery, State};
use axum::response::{IntoResponse, Response};
use chrono::{NaiveDate, Utc};
use sqlx::PgPool;

use super::{AppState, Html, server_error};
use crate::domain::filter::{
    DirectoryFilters, NEGARA_OPTIONS, PENDANAAN_OPTIONS, PENYELENGGARA_OPTIONS, PER_PAGE,
    StatusFilter, active_groups, status_sql, urlencode,
};
use crate::domain::scholarship::{
    Status, countdown_days, detail_path, fmt_short, status, today_jakarta,
};
use crate::seo::jsonld;

pub const POPULAR: [&str; 7] = [
    "Beasiswa LPDP",
    "Chevening",
    "MEXT",
    "GKS",
    "NL Scholarship",
    "Beasiswa Indonesia Bangkit",
    "DAAD",
];

const TITLE: &str = "Direktori Beasiswa S1, S2, S3 Dalam dan Luar Negeri | Scholarly";
const META: &str = "Direktori beasiswa terkurasi untuk pelajar Indonesia: dalam negeri dan luar negeri, S1 sampai S3. Cek syarat, dokumen, dan deadline beasiswa di Scholarly.";

#[derive(Template)]
#[template(path = "directory.html")]
pub struct DirectoryView {
    pub title: String,
    pub meta_description: String,
    pub canonical: String,
    pub robots: String,
    pub website_jsonld: String,
    pub itemlist_jsonld: String,
    pub rel_prev: String,
    pub rel_next: String,
    pub q: String,
    pub popular: Vec<PopularChip>,
    pub groups: Vec<FilterGroupView>,
    pub active_chips: Vec<ActiveChip>,
    pub active_count: usize,
    pub result_line: String,
    pub is_terbaru: bool,
    pub is_deadline: bool,
    pub is_populer: bool,
    pub sort_hidden: Vec<(String, String)>,
    pub cards: Vec<CardView>,
    pub total: i64,
    pub show_more_url: String,
}

pub struct PopularChip {
    pub name: String,
    pub url: String,
}

pub struct FilterGroupView {
    pub param: String,
    pub label: String,
    pub open: bool,
    pub has_search: bool,
    pub options: Vec<FilterOptionView>,
}

pub struct FilterOptionView {
    pub value: String,
    pub label: String,
    pub count: i64,
    pub checked: bool,
}

pub struct ActiveChip {
    pub label: String,
    pub remove_url: String,
}

pub struct CardView {
    pub url: String,
    pub name: String,
    pub provider: String,
    pub country: String,
    pub levels_label: String,
    pub funding: String,
    pub funding_class: String,
    pub deadline_label: String,
    pub urgency: String,
    pub graphic: String,
}

#[derive(sqlx::FromRow)]
struct CardRow {
    slug: String,
    name: String,
    provider: String,
    country: String,
    levels: Vec<String>,
    funding_type: String,
    close_date: Option<NaiveDate>,
    deadline_type: String,
}

#[derive(sqlx::FromRow)]
struct FacetRow {
    total: i64,
    c_s1: i64,
    c_s2: i64,
    c_s3: i64,
    c_dalam: i64,
    c_luar: i64,
    c_full: i64,
    c_parsial: i64,
    c_tunjangan: i64,
    c_pemerintah: i64,
    c_kampus: i64,
    c_swasta: i64,
    c_yayasan: i64,
    c_buka: i64,
    c_segera: i64,
    c_tutup: i64,
}

#[derive(sqlx::FromRow)]
struct CountryCount {
    country: String,
    count: i64,
}

enum Bind {
    Text(String),
    Countries(Vec<String>),
}

/// Runtime-bound dynamic WHERE builder. Values are bound, never interpolated.
struct WhereBuilder {
    clauses: Vec<String>,
    binds: Vec<Bind>,
}

impl WhereBuilder {
    fn new() -> Self {
        Self {
            clauses: vec!["s.published".into()],
            binds: vec![],
        }
    }

    fn param(&mut self, value: &str) -> usize {
        self.binds.push(Bind::Text(value.to_string()));
        self.binds.len()
    }

    fn text_clause(&mut self, sql: impl FnOnce(usize) -> String, value: &str) {
        let idx = self.param(value);
        self.clauses.push(sql(idx));
    }

    fn push_like(&mut self, needle: &str) {
        let idx = self.param(needle);
        self.clauses.push(format!(
            "(s.name ILIKE '%' || ${idx} || '%' OR s.official_name ILIKE '%' || ${idx} || '%' \
             OR s.provider ILIKE '%' || ${idx} || '%' OR s.country ILIKE '%' || ${idx} || '%')"
        ));
    }

    fn sql(&self) -> String {
        format!("WHERE {}", self.clauses.join(" AND "))
    }

    async fn fetch_all<R>(
        &self,
        pool: &PgPool,
        select: &str,
        tail: &str,
    ) -> Result<Vec<R>, sqlx::Error>
    where
        R: for<'r> sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Unpin,
    {
        // Every dynamic fragment is a constant string from this file; only
        // bound parameter values vary, so the audit for AssertSqlSafe holds.
        let query = format!("{select} {} {tail}", self.sql());
        let mut q = sqlx::query_as::<_, R>(sqlx::AssertSqlSafe(query));
        for bind in &self.binds {
            q = match bind {
                Bind::Text(t) => q.bind(t),
                Bind::Countries(c) => q.bind(c),
            };
        }
        q.fetch_all(pool).await
    }

    async fn count(&self, pool: &PgPool) -> Result<i64, sqlx::Error> {
        // Constant fragments only, see the audit note in fetch_all.
        let query = format!("SELECT count(*) FROM scholarships s {}", self.sql());
        let mut q = sqlx::query_scalar::<_, i64>(sqlx::AssertSqlSafe(query));
        for bind in &self.binds {
            q = match bind {
                Bind::Text(t) => q.bind(t),
                Bind::Countries(c) => q.bind(c),
            };
        }
        q.fetch_one(pool).await
    }
}

fn rows_sql(select: &str, where_sql: &str, order: &str, bind_count: usize) -> String {
    format!(
        "{select} {where_sql} ORDER BY {order}, s.slug ASC LIMIT ${} OFFSET ${}",
        bind_count + 1,
        bind_count + 2
    )
}

fn add_filters(where_: &mut WhereBuilder, f: &DirectoryFilters) {
    if !f.q.is_empty() {
        where_.push_like(&f.q);
    }
    for v in &f.jenjang {
        where_.text_clause(|i| format!("${i} = ANY(s.levels)"), v);
    }
    for v in &f.lokasi {
        where_.text_clause(|i| format!("s.location = ${i}"), v);
    }
    for v in &f.negara {
        if v == "lainnya" {
            where_.binds.push(Bind::Countries(
                NEGARA_OPTIONS.iter().map(|s| s.to_string()).collect(),
            ));
            let idx = where_.binds.len();
            where_
                .clauses
                .push(format!("NOT (s.country = ANY(${idx}))"));
        } else {
            where_.text_clause(|i| format!("s.country = ${i}"), v);
        }
    }
    for v in &f.pendanaan {
        where_.text_clause(|i| format!("s.funding_type = ${i}"), v);
    }
    for v in &f.penyelenggara {
        where_.text_clause(|i| format!("s.provider_type = ${i}"), v);
    }
    if !f.status.is_empty() {
        let status_expr = status_sql("s");
        let parts: Vec<String> = f
            .status
            .iter()
            .map(|s| {
                let idx = where_.param(match s {
                    StatusFilter::Open => "buka",
                    StatusFilter::ClosingSoon => "segera",
                    StatusFilter::Closed => "tutup",
                });
                format!("({status_expr})::text = ${idx}")
            })
            .collect();
        where_.clauses.push(format!("({})", parts.join(" OR ")));
    }
}

#[axum::debug_handler]
pub async fn page(State(state): State<AppState>, RawQuery(query): RawQuery) -> Response {
    let filters = DirectoryFilters::parse(query.as_deref());
    render(state, filters).await
}

async fn render(state: AppState, f: DirectoryFilters) -> Response {
    let today = today_jakarta(Utc::now());
    let pool = &state.pool;
    let base = &state.config.base_url;

    // Facet counts use the search term only; other groups stay unapplied so
    // every option count is visible while a group is active.
    let mut where_q = WhereBuilder::new();
    if !f.q.is_empty() {
        where_q.push_like(&f.q);
    }
    let status_expr = status_sql("s");
    let facet_select = format!(
        "SELECT count(*) AS total, \
         count(*) FILTER (WHERE 'S1' = ANY(s.levels)) AS c_s1, \
         count(*) FILTER (WHERE 'S2' = ANY(s.levels)) AS c_s2, \
         count(*) FILTER (WHERE 'S3' = ANY(s.levels)) AS c_s3, \
         count(*) FILTER (WHERE s.location = 'Dalam Negeri') AS c_dalam, \
         count(*) FILTER (WHERE s.location = 'Luar Negeri') AS c_luar, \
         count(*) FILTER (WHERE s.funding_type = 'Fully Funded') AS c_full, \
         count(*) FILTER (WHERE s.funding_type = 'Parsial') AS c_parsial, \
         count(*) FILTER (WHERE s.funding_type = 'Tunjangan') AS c_tunjangan, \
         count(*) FILTER (WHERE s.provider_type = 'Pemerintah') AS c_pemerintah, \
         count(*) FILTER (WHERE s.provider_type = 'Kampus') AS c_kampus, \
         count(*) FILTER (WHERE s.provider_type = 'Swasta') AS c_swasta, \
         count(*) FILTER (WHERE s.provider_type = 'Yayasan') AS c_yayasan, \
         count(*) FILTER (WHERE ({status_expr})::text = 'buka') AS c_buka, \
         count(*) FILTER (WHERE ({status_expr})::text = 'segera') AS c_segera, \
         count(*) FILTER (WHERE ({status_expr})::text = 'tutup') AS c_tutup \
         FROM scholarships s"
    );
    let facets: Vec<FacetRow> = match where_q.fetch_all(pool, &facet_select, "").await {
        Ok(rows) => rows,
        Err(e) => return server_error("facet query", e),
    };
    let facets = facets.into_iter().next().unwrap_or(FacetRow {
        total: 0,
        c_s1: 0,
        c_s2: 0,
        c_s3: 0,
        c_dalam: 0,
        c_luar: 0,
        c_full: 0,
        c_parsial: 0,
        c_tunjangan: 0,
        c_pemerintah: 0,
        c_kampus: 0,
        c_swasta: 0,
        c_yayasan: 0,
        c_buka: 0,
        c_segera: 0,
        c_tutup: 0,
    });

    let country_counts: Vec<CountryCount> = match where_q
        .fetch_all(
            pool,
            "SELECT s.country AS country, count(*) AS count FROM scholarships s",
            "GROUP BY s.country ORDER BY count(*) DESC, s.country",
        )
        .await
    {
        Ok(rows) => rows,
        Err(e) => return server_error("country facet", e),
    };

    let mut where_main = WhereBuilder::new();
    add_filters(&mut where_main, &f);
    let total = match where_main.count(pool).await {
        Ok(n) => n,
        Err(e) => return server_error("directory count", e),
    };

    let offset = (f.page - 1).max(0) * PER_PAGE;
    let rows_query = where_main;
    let select = "SELECT s.slug, s.name, s.provider, s.country, s.levels, s.funding_type, s.close_date, \
                  s.deadline_type FROM scholarships s";
    let query = rows_sql(
        select,
        &rows_query.sql(),
        f.sort.order_clause(),
        rows_query.binds.len(),
    );
    // Constant fragments only, see the audit note in fetch_all.
    let mut q = sqlx::query_as::<_, CardRow>(sqlx::AssertSqlSafe(query));
    for bind in &rows_query.binds {
        q = match bind {
            Bind::Text(t) => q.bind(t),
            Bind::Countries(c) => q.bind(c),
        };
    }
    let rows: Vec<CardRow> = match q.bind(PER_PAGE).bind(offset).fetch_all(pool).await {
        Ok(rows) => rows,
        Err(e) => return server_error("directory rows", e),
    };

    let cards: Vec<CardView> = rows.into_iter().map(|r| card_view(r, today)).collect();

    let from = if total == 0 { 0 } else { offset + 1 };
    let to = (offset + PER_PAGE).min(total);
    let result_line = if total == 0 {
        "Tidak ada beasiswa yang cocok".to_string()
    } else {
        format!("Menampilkan {from} sampai {to} dari {total} beasiswa")
    };

    let mut active_chips = Vec::new();
    for (param, values) in active_groups(&f) {
        for (value, label) in values {
            active_chips.push(ActiveChip {
                label,
                remove_url: f.without(param, &value),
            });
        }
    }
    let active_count = active_chips.len();

    let mut sort_hidden = Vec::new();
    if !f.q.is_empty() {
        sort_hidden.push(("q".to_string(), f.q.clone()));
    }
    for v in &f.jenjang {
        sort_hidden.push(("jenjang".into(), v.clone()));
    }
    for v in &f.lokasi {
        sort_hidden.push(("lokasi".into(), v.clone()));
    }
    for v in &f.negara {
        sort_hidden.push(("negara".into(), v.clone()));
    }
    for v in &f.pendanaan {
        sort_hidden.push(("pendanaan".into(), v.clone()));
    }
    for v in &f.penyelenggara {
        sort_hidden.push(("penyelenggara".into(), v.clone()));
    }
    for s in &f.status {
        sort_hidden.push(("status".into(), s.value().into()));
    }

    let show_more_url = if offset + PER_PAGE < total {
        f.page_url(f.page + 1)
    } else {
        String::new()
    };
    let rel_prev = if f.page > 1 {
        format!("{base}{}", f.page_url(f.page - 1))
    } else {
        String::new()
    };
    let rel_next = if offset + PER_PAGE < total {
        format!("{base}{}", f.page_url(f.page + 1))
    } else {
        String::new()
    };

    let view = DirectoryView {
        title: TITLE.into(),
        meta_description: META.into(),
        canonical: format!("{base}/"),
        robots: if f.is_filtered_or_paged() {
            "noindex, follow".into()
        } else {
            "index, follow, max-image-preview:large, max-snippet:-1".into()
        },
        website_jsonld: jsonld::website_with_search_action(base),
        itemlist_jsonld: jsonld::item_list(
            base,
            &cards
                .iter()
                .map(|c| (c.name.clone(), c.url.clone()))
                .collect::<Vec<_>>(),
        ),
        rel_prev,
        rel_next,
        q: f.q.clone(),
        popular: POPULAR
            .iter()
            .map(|name| PopularChip {
                name: name.to_string(),
                url: format!("/?q={}", urlencode(name)),
            })
            .collect(),
        groups: build_groups(&f, &facets, &country_counts),
        active_chips,
        active_count,
        result_line,
        is_terbaru: f.sort == crate::domain::filter::Sort::Terbaru,
        is_deadline: f.sort == crate::domain::filter::Sort::DeadlineTerdekat,
        is_populer: f.sort == crate::domain::filter::Sort::PalingDicari,
        sort_hidden,
        cards,
        total,
        show_more_url,
    };
    Html(view).into_response()
}

fn checked(values: &[String], value: &str) -> bool {
    values.iter().any(|v| v == value)
}

fn build_groups(
    f: &DirectoryFilters,
    facets: &FacetRow,
    countries: &[CountryCount],
) -> Vec<FilterGroupView> {
    let count_of = |name: &str| -> i64 {
        countries
            .iter()
            .find(|c| c.country == name)
            .map(|c| c.count)
            .unwrap_or(0)
    };
    let known_total: i64 = NEGARA_OPTIONS.iter().map(|c| count_of(c)).sum();
    let lainnya = (facets.total - known_total).max(0);

    vec![
        FilterGroupView {
            param: "jenjang".into(),
            label: "Jenjang".into(),
            open: true,
            has_search: false,
            options: vec![
                option("jenjang", "S1", "Sarjana (S1)", facets.c_s1, &f.jenjang),
                option("jenjang", "S2", "Magister (S2)", facets.c_s2, &f.jenjang),
                option("jenjang", "S3", "Doktoral (S3)", facets.c_s3, &f.jenjang),
            ],
        },
        FilterGroupView {
            param: "lokasi".into(),
            label: "Lokasi".into(),
            open: true,
            has_search: false,
            options: vec![
                option(
                    "lokasi",
                    "Dalam Negeri",
                    "Dalam Negeri",
                    facets.c_dalam,
                    &f.lokasi,
                ),
                option(
                    "lokasi",
                    "Luar Negeri",
                    "Luar Negeri",
                    facets.c_luar,
                    &f.lokasi,
                ),
            ],
        },
        FilterGroupView {
            param: "negara".into(),
            label: "Negara".into(),
            open: false,
            has_search: true,
            options: NEGARA_OPTIONS
                .iter()
                .map(|c| option("negara", c, c, count_of(c), &f.negara))
                .chain(std::iter::once(option(
                    "negara", "lainnya", "Lainnya", lainnya, &f.negara,
                )))
                .collect(),
        },
        FilterGroupView {
            param: "pendanaan".into(),
            label: "Pendanaan".into(),
            open: false,
            has_search: false,
            options: vec![
                option(
                    "pendanaan",
                    PENDANAAN_OPTIONS[0],
                    PENDANAAN_OPTIONS[0],
                    facets.c_full,
                    &f.pendanaan,
                ),
                option(
                    "pendanaan",
                    PENDANAAN_OPTIONS[1],
                    PENDANAAN_OPTIONS[1],
                    facets.c_parsial,
                    &f.pendanaan,
                ),
                option(
                    "pendanaan",
                    PENDANAAN_OPTIONS[2],
                    PENDANAAN_OPTIONS[2],
                    facets.c_tunjangan,
                    &f.pendanaan,
                ),
            ],
        },
        FilterGroupView {
            param: "penyelenggara".into(),
            label: "Penyelenggara".into(),
            open: false,
            has_search: false,
            options: vec![
                option(
                    "penyelenggara",
                    PENYELENGGARA_OPTIONS[0],
                    PENYELENGGARA_OPTIONS[0],
                    facets.c_pemerintah,
                    &f.penyelenggara,
                ),
                option(
                    "penyelenggara",
                    PENYELENGGARA_OPTIONS[1],
                    PENYELENGGARA_OPTIONS[1],
                    facets.c_kampus,
                    &f.penyelenggara,
                ),
                option(
                    "penyelenggara",
                    PENYELENGGARA_OPTIONS[2],
                    PENYELENGGARA_OPTIONS[2],
                    facets.c_swasta,
                    &f.penyelenggara,
                ),
                option(
                    "penyelenggara",
                    PENYELENGGARA_OPTIONS[3],
                    PENYELENGGARA_OPTIONS[3],
                    facets.c_yayasan,
                    &f.penyelenggara,
                ),
            ],
        },
        FilterGroupView {
            param: "status".into(),
            label: "Status Pendaftaran".into(),
            open: false,
            has_search: false,
            options: vec![
                FilterOptionView {
                    value: "buka".into(),
                    label: Status::Open.label().into(),
                    count: facets.c_buka,
                    checked: f.status.contains(&StatusFilter::Open),
                },
                FilterOptionView {
                    value: "segera".into(),
                    label: Status::ClosingSoon.label().into(),
                    count: facets.c_segera,
                    checked: f.status.contains(&StatusFilter::ClosingSoon),
                },
                FilterOptionView {
                    value: "tutup".into(),
                    label: Status::Closed.label().into(),
                    count: facets.c_tutup,
                    checked: f.status.contains(&StatusFilter::Closed),
                },
            ],
        },
    ]
}

fn option(
    param: &str,
    value: &str,
    label: &str,
    count: i64,
    current: &[String],
) -> FilterOptionView {
    FilterOptionView {
        value: value.to_string(),
        label: label.to_string(),
        count,
        checked: param != "status" && checked(current, value),
    }
}

fn card_view(row: CardRow, today: NaiveDate) -> CardView {
    let deadline_type = crate::domain::scholarship::DeadlineType::parse(&row.deadline_type);
    let st = status(row.close_date, deadline_type, today);
    let days = countdown_days(row.close_date, deadline_type, today);
    let urgency = match (st, days) {
        (Status::Closed, _) => String::new(),
        (_, Some(0)) => "Tutup hari ini".to_string(),
        (_, Some(d)) if d <= crate::domain::scholarship::URGENCY_DAYS => format!("Sisa {d} hari"),
        _ => String::new(),
    };
    let deadline_label = match row.close_date {
        Some(d) => format!("Tutup: {}", fmt_short(d)),
        None if deadline_type == crate::domain::scholarship::DeadlineType::Rolling => {
            "Rolling".into()
        }
        None => "Cek situs resmi".into(),
    };
    CardView {
        url: detail_path(&row.slug),
        name: row.name.clone(),
        provider: row.provider,
        country: row.country,
        levels_label: row.levels.join(", "),
        funding: row.funding_type.clone(),
        funding_class: if row.funding_type == "Fully Funded" {
            "badge-yellow".into()
        } else {
            "badge-neutral".into()
        },
        deadline_label,
        urgency,
        graphic: row
            .name
            .chars()
            .next()
            .unwrap_or('B')
            .to_uppercase()
            .to_string(),
    }
}

#[cfg(test)]
mod rows_sql_tests {
    #[test]
    fn rows_sql_keeps_where_before_order() {
        let sql = super::rows_sql(
            "SELECT 1 FROM scholarships s",
            "WHERE a = $1",
            "s.close_date",
            1,
        );
        assert!(
            sql.contains("WHERE a = $1 ORDER BY s.close_date, s.slug ASC"),
            "{sql}"
        );
        assert!(sql.contains("LIMIT $2 OFFSET $3"), "{sql}");
    }
}
