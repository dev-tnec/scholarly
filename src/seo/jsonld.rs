use serde_json::{Value, json};

use crate::domain::scholarship::detail_path;

/// JSON-LD for a script tag must never carry a raw `</script>`, PRD section 12.
pub fn escape_closing_tags(json: &str) -> String {
    json.replace("</", "<\\/")
}

pub fn website_with_search_action(base_url: &str) -> String {
    let value = json!({
        "@context": "https://schema.org",
        "@type": "WebSite",
        "name": "Scholarly",
        "url": base_url,
        "description": "Direktori beasiswa dalam dan luar negeri untuk pelajar Indonesia, S1 sampai S3.",
        "publisher": {
            "@type": "Organization",
            "name": "Titik Nol English Course",
            "url": "https://titiknolenglish.com"
        },
        "potentialAction": {
            "@type": "SearchAction",
            "target": {
                "@type": "EntryPoint",
                "urlTemplate": format!("{base_url}/?q={{search_term_string}}")
            },
            "query-input": "required name=search_term_string"
        }
    });
    escape_closing_tags(&value.to_string())
}

pub fn organization(base_url: &str) -> String {
    let value = json!({
        "@context": "https://schema.org",
        "@type": "Organization",
        "name": "Titik Nol English Course",
        "url": "https://titiknolenglish.com",
        "logo": format!("{base_url}/static/img/og.png")
    });
    escape_closing_tags(&value.to_string())
}

pub fn item_list(base_url: &str, items: &[(String, String)]) -> String {
    let list: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(i, (name, url))| {
            json!({
                "@type": "ListItem",
                "position": i + 1,
                "name": name,
                "url": url
            })
        })
        .collect();
    let value = json!({
        "@context": "https://schema.org",
        "@type": "ItemList",
        "name": "Daftar beasiswa untuk pelajar Indonesia",
        "url": base_url,
        "numberOfItems": items.len(),
        "itemListElement": list
    });
    escape_closing_tags(&value.to_string())
}

pub fn breadcrumb(base_url: &str, name: &str, slug: &str) -> String {
    let value = json!({
        "@context": "https://schema.org",
        "@type": "BreadcrumbList",
        "itemListElement": [
            {
                "@type": "ListItem",
                "position": 1,
                "name": "Beranda",
                "item": base_url
            },
            {
                "@type": "ListItem",
                "position": 2,
                "name": "Beasiswa",
                "item": format!("{base_url}/#beasiswa")
            },
            {
                "@type": "ListItem",
                "position": 3,
                "name": name,
                "item": format!("{base_url}{}", detail_path(slug))
            }
        ]
    });
    escape_closing_tags(&value.to_string())
}

pub fn faq_page(faqs: &[(String, String)]) -> String {
    let value = json!({
        "@context": "https://schema.org",
        "@type": "FAQPage",
        "mainEntity": faqs
            .iter()
            .map(|(q, a)| json!({
                "@type": "Question",
                "name": q,
                "acceptedAnswer": { "@type": "Answer", "text": a }
            }))
            .collect::<Vec<Value>>()
    });
    escape_closing_tags(&value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jsonld_escapes_script_close() {
        let faqs = [(
            r#"Apa itu <script>alert("x")</script>?"#.into(),
            "Jawaban".into(),
        )];
        let out = faq_page(&faqs);
        assert!(!out.contains("</script>"), "raw closing tag leaked: {out}");
        assert!(out.contains("<\\/script>"));
    }

    #[test]
    fn website_search_action_targets_directory() {
        let out = website_with_search_action("https://scholarly.test");
        assert!(out.contains("https://scholarly.test/?q="));
        assert!(out.contains("SearchAction"));
    }

    #[test]
    fn breadcrumb_uses_trailing_slash_detail_url() {
        let out = breadcrumb("https://scholarly.test", "NL Scholarship", "nl-scholarship");
        assert!(out.contains("https://scholarly.test/nl-scholarship/"));
    }
}
