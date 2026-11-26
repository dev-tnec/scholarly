use std::collections::BTreeMap;

use super::scholarship::Status;

pub const PER_PAGE: i64 = 24;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DirectoryFilters {
    pub q: String,
    pub jenjang: Vec<String>,
    pub lokasi: Vec<String>,
    pub negara: Vec<String>,
    pub pendanaan: Vec<String>,
    pub penyelenggara: Vec<String>,
    pub status: Vec<StatusFilter>,
    pub sort: Sort,
    pub page: i64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StatusFilter {
    Open,
    ClosingSoon,
    Closed,
}

impl StatusFilter {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "buka" | "Buka" => Some(Self::Open),
            "segera" | "Segera Tutup" => Some(Self::ClosingSoon),
            "tutup" | "Tutup" => Some(Self::Closed),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Open => Status::Open.label(),
            Self::ClosingSoon => Status::ClosingSoon.label(),
            Self::Closed => Status::Closed.label(),
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Open => "buka",
            Self::ClosingSoon => "segera",
            Self::Closed => "tutup",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sort {
    #[default]
    Terbaru,
    DeadlineTerdekat,
    PalingDicari,
}

impl Sort {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "terbaru" | "Terbaru" => Some(Self::Terbaru),
            "deadline" | "Deadline Terdekat" => Some(Self::DeadlineTerdekat),
            "populer" | "Paling Dicari" => Some(Self::PalingDicari),
            _ => None,
        }
    }

    pub fn value(self) -> &'static str {
        match self {
            Self::Terbaru => "terbaru",
            Self::DeadlineTerdekat => "deadline",
            Self::PalingDicari => "populer",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Terbaru => "Terbaru",
            Self::DeadlineTerdekat => "Deadline Terdekat",
            Self::PalingDicari => "Paling Dicari",
        }
    }

    pub fn order_clause(self) -> &'static str {
        match self {
            Self::Terbaru => "s.checked_at DESC, s.id DESC",
            Self::DeadlineTerdekat => "s.close_date ASC NULLS LAST, s.id ASC",
            Self::PalingDicari => "s.popularity DESC, s.name ASC",
        }
    }
}

pub const JENJANG_OPTIONS: [&str; 3] = ["S1", "S2", "S3"];
pub const LOKASI_OPTIONS: [&str; 2] = ["Dalam Negeri", "Luar Negeri"];
pub const PENDANAAN_OPTIONS: [&str; 3] = ["Fully Funded", "Parsial", "Tunjangan"];
pub const PENYELENGGARA_OPTIONS: [&str; 4] = ["Pemerintah", "Kampus", "Swasta", "Yayasan"];
pub const NEGARA_OPTIONS: [&str; 8] = [
    "Indonesia",
    "Inggris",
    "Australia",
    "Jepang",
    "Korea Selatan",
    "Amerika Serikat",
    "Belanda",
    "Jerman",
];

/// Percent-decode a query component, plus '+' as space.
fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                if let Ok(v) = u8::from_str_radix(hex, 16) {
                    out.push(v);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Parse a raw query string into ordered key-value pairs, tolerating repeats.
fn parse_pairs(query: &str) -> Vec<(String, String)> {
    query
        .split('&')
        .filter(|p| !p.is_empty())
        .map(|pair| {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            (percent_decode(k), percent_decode(v))
        })
        .collect()
}

impl DirectoryFilters {
    /// Strict-typed parsing: unknown enum values are dropped, canonical stays /.
    pub fn parse(query: Option<&str>) -> Self {
        let mut f = Self::default();
        for (key, value) in parse_pairs(query.unwrap_or("")) {
            match key.as_str() {
                "q" => f.q = value.trim().to_string(),
                "jenjang" => push_known(&mut f.jenjang, &value, &JENJANG_OPTIONS),
                "lokasi" => push_known(&mut f.lokasi, &value, &LOKASI_OPTIONS),
                "negara" => {
                    if value == "lainnya" {
                        push_known(&mut f.negara, &value, &["lainnya"]);
                    } else {
                        push_known(&mut f.negara, &value, &NEGARA_OPTIONS);
                    }
                }
                "pendanaan" => push_known(&mut f.pendanaan, &value, &PENDANAAN_OPTIONS),
                "penyelenggara" => push_known(&mut f.penyelenggara, &value, &PENYELENGGARA_OPTIONS),
                "status" => {
                    if let Some(s) = StatusFilter::parse(&value)
                        && !f.status.contains(&s)
                    {
                        f.status.push(s);
                    }
                }
                "sort" => {
                    if let Some(s) = Sort::parse(&value) {
                        f.sort = s;
                    }
                }
                "page" => {
                    if let Ok(p) = value.parse::<i64>() {
                        f.page = p.max(1);
                    }
                }
                _ => {}
            }
        }
        if f.q.is_empty() {
            f.page = f.page.max(1);
        }
        f
    }

    /// Any filter, sort, or page beyond 1 makes the URL non-canonical: the
    /// directory canonical is always / and such URLs carry noindex, follow.
    pub fn is_filtered_or_paged(&self) -> bool {
        !self.q.is_empty()
            || !self.jenjang.is_empty()
            || !self.lokasi.is_empty()
            || !self.negara.is_empty()
            || !self.pendanaan.is_empty()
            || !self.penyelenggara.is_empty()
            || !self.status.is_empty()
            || self.sort != Sort::default()
            || self.page > 1
    }

    /// Query string for the current state, without a leading '?'.
    pub fn to_query(&self) -> String {
        let mut parts = Vec::new();
        if !self.q.is_empty() {
            parts.push(format!("q={}", urlencode(&self.q)));
        }
        for v in &self.jenjang {
            parts.push(format!("jenjang={}", urlencode(v)));
        }
        for v in &self.lokasi {
            parts.push(format!("lokasi={}", urlencode(v)));
        }
        for v in &self.negara {
            parts.push(format!("negara={}", urlencode(v)));
        }
        for v in &self.pendanaan {
            parts.push(format!("pendanaan={}", urlencode(v)));
        }
        for v in &self.penyelenggara {
            parts.push(format!("penyelenggara={}", urlencode(v)));
        }
        for v in &self.status {
            parts.push(format!("status={}", v.value()));
        }
        if self.sort != Sort::default() {
            parts.push(format!("sort={}", self.sort.value()));
        }
        if self.page > 1 {
            parts.push(format!("page={}", self.page));
        }
        parts.join("&")
    }

    /// URL for a given page number, keeping filters.
    pub fn page_url(&self, page: i64) -> String {
        let mut me = self.clone();
        me.page = page.max(1);
        let q = me.to_query();
        if q.is_empty() {
            "/".into()
        } else {
            format!("/?{q}")
        }
    }

    /// URL with one filter value removed, for the active-chip x buttons.
    pub fn without(&self, param: &str, value: &str) -> String {
        let mut me = self.clone();
        let target = match param {
            "jenjang" => &mut me.jenjang,
            "lokasi" => &mut me.lokasi,
            "negara" => &mut me.negara,
            "pendanaan" => &mut me.pendanaan,
            "penyelenggara" => &mut me.penyelenggara,
            _ => &mut Vec::new(),
        };
        target.retain(|v| v != value);
        if param == "q" {
            me.q.clear();
        }
        if param == "status" {
            me.status.retain(|s| s.value() != value);
        }
        me.page = 1;
        let q = me.to_query();
        if q.is_empty() {
            "/".into()
        } else {
            format!("/?{q}")
        }
    }
}

fn push_known(target: &mut Vec<String>, value: &str, known: &[&str]) {
    if known.contains(&value) && !target.iter().any(|v| v == value) {
        target.push(value.to_string());
    }
}

pub fn urlencode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 2);
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            b' ' => out.push_str("%20"),
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// SQL CASE computing status at render time in Asia/Jakarta time.
pub fn status_sql(alias: &str) -> String {
    let s = if alias.is_empty() {
        String::new()
    } else {
        format!("{alias}.")
    };
    let today = "(CURRENT_TIMESTAMP AT TIME ZONE 'Asia/Jakarta')::date";
    format!(
        "CASE WHEN {s}deadline_type = 'rolling' THEN 'buka' \
         WHEN {s}close_date IS NULL THEN 'buka' \
         WHEN {s}close_date < {today} THEN 'tutup' \
         WHEN {s}close_date <= {today} + {CLOSING} THEN 'segera' \
         ELSE 'buka' END",
        s = s,
        CLOSING = super::scholarship::CLOSING_SOON_DAYS
    )
}

/// Build filters for the remaining groups, given per-group selected values.
pub struct Group<'a> {
    pub param: &'a str,
    pub label: &'a str,
    pub values: Vec<String>,
}

pub fn active_groups(f: &DirectoryFilters) -> Vec<(&'static str, Vec<(String, String)>)> {
    // (param, label, value, human label) chips for the active filter row.
    let mut chips: Vec<(&'static str, Vec<(String, String)>)> = Vec::new();
    if !f.q.is_empty() {
        chips.push(("q", vec![(f.q.clone(), format!("Cari: {}", f.q))]));
    }
    let plain = [
        ("jenjang", &f.jenjang),
        ("lokasi", &f.lokasi),
        ("negara", &f.negara),
        ("pendanaan", &f.pendanaan),
        ("penyelenggara", &f.penyelenggara),
    ];
    for (param, values) in plain {
        if !values.is_empty() {
            chips.push((
                param,
                values
                    .iter()
                    .map(|v| (v.clone(), v.clone()))
                    .collect::<Vec<_>>(),
            ));
        }
    }
    if !f.status.is_empty() {
        chips.push((
            "status",
            f.status
                .iter()
                .map(|s| (s.value().to_string(), s.label().to_string()))
                .collect::<Vec<_>>(),
        ));
    }
    chips
}

/// Order map keys for deterministic query strings in tests.
pub fn sorted_pairs(map: BTreeMap<String, String>) -> Vec<(String, String)> {
    map.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_filters() {
        let f = DirectoryFilters::parse(Some(
            "q=belanda&jenjang=S1&jenjang=S2&negara=Belanda&pendanaan=Fully%20Funded&status=segera&sort=deadline&page=2",
        ));
        assert_eq!(f.q, "belanda");
        assert_eq!(f.jenjang, ["S1", "S2"]);
        assert_eq!(f.negara, ["Belanda"]);
        assert_eq!(f.pendanaan, ["Fully Funded"]);
        assert_eq!(f.status, [StatusFilter::ClosingSoon]);
        assert_eq!(f.sort, Sort::DeadlineTerdekat);
        assert_eq!(f.page, 2);
    }

    #[test]
    fn unknown_values_are_ignored() {
        let f = DirectoryFilters::parse(Some(
            "jenjang=S9&negara=Mars&status=mungkin&sort=acak&jenjang=S1",
        ));
        assert_eq!(f.jenjang, ["S1"]);
        assert!(f.negara.is_empty());
        assert!(f.status.is_empty());
        assert_eq!(f.sort, Sort::Terbaru);
    }

    #[test]
    fn mixed_valid_and_unknown() {
        let f = DirectoryFilters::parse(Some(
            "penyelenggara=Pemerintah&penyelenggara=Raja&lokasi=Luar%20Negeri&page=0",
        ));
        assert_eq!(f.penyelenggara, ["Pemerintah"]);
        assert_eq!(f.lokasi, ["Luar Negeri"]);
        assert_eq!(f.page, 1);
    }

    #[test]
    fn repeats_do_not_duplicate() {
        let f = DirectoryFilters::parse(Some("jenjang=S1&jenjang=S1"));
        assert_eq!(f.jenjang, ["S1"]);
    }

    #[test]
    fn noindex_only_when_not_canonical() {
        assert!(!DirectoryFilters::parse(None).is_filtered_or_paged());
        assert!(!DirectoryFilters::parse(Some("page=1")).is_filtered_or_paged());
        assert!(DirectoryFilters::parse(Some("q=lpdp")).is_filtered_or_paged());
        assert!(DirectoryFilters::parse(Some("sort=deadline")).is_filtered_or_paged());
        assert!(DirectoryFilters::parse(Some("page=3")).is_filtered_or_paged());
    }

    #[test]
    fn query_roundtrip_and_removal() {
        let f = DirectoryFilters::parse(Some("q=lpdp&jenjang=S2&status=segera&page=2"));
        assert_eq!(f.to_query(), "q=lpdp&jenjang=S2&status=segera&page=2");
        let removed = f.without("jenjang", "S2");
        assert_eq!(removed, "/?q=lpdp&status=segera");
        let cleared = f.without("q", "lpdp");
        assert_eq!(cleared, "/?jenjang=S2&status=segera");
    }

    #[test]
    fn page_urls() {
        let f = DirectoryFilters::parse(Some("q=lpdp"));
        assert_eq!(f.page_url(1), "/?q=lpdp");
        assert_eq!(f.page_url(2), "/?q=lpdp&page=2");
        assert_eq!(DirectoryFilters::default().page_url(1), "/");
    }

    #[test]
    fn urlencode_keeps_safe_chars() {
        assert_eq!(urlencode("NL Scholarship 2027"), "NL%20Scholarship%202027");
        assert_eq!(urlencode("Fully-Funded_a.b~c"), "Fully-Funded_a.b~c");
    }

    #[test]
    fn percent_decode_handles_plus_and_utf8() {
        let f = DirectoryFilters::parse(Some("q=Cari+beasiswa%20luar"));
        assert_eq!(f.q, "Cari beasiswa luar");
    }
}
