//! Credential-free Markdown documentation with local link checks and source provenance.
mod content;
mod css;
mod output;
mod render;
mod validate;

use render::Page;
use std::fmt::Write as _;
use std::{collections::BTreeMap, path::Path};

const BASE: &str = "/metaharness/";
const CSS: &str = include_str!("../../../website/styles.css");
const LANDING: &str = include_str!("../../../website/index.html");
const LOGO: &str = include_str!("../../../website/static/img/logo.svg");

type Files = BTreeMap<String, Vec<u8>>;

struct Source<'a> {
    key: &'a str,
    group: &'a str,
    markdown: &'a str,
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn route(key: &str) -> Result<String, String> {
    if key.is_empty()
        || key.split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || !part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
        })
    {
        return Err(format!("invalid documentation route {key}"));
    }
    Ok(if key == "index" {
        format!("{BASE}docs/")
    } else {
        format!("{BASE}docs/{key}/")
    })
}

fn check_commit(commit: &str) -> Result<(), String> {
    if commit.len() != 40
        || !commit.bytes().all(|c| c.is_ascii_hexdigit())
        || commit.bytes().all(|c| c == b'0')
    {
        return Err("commit must be a nonzero full 40-digit Git revision".into());
    }
    Ok(())
}

fn navigation(pages: &[Page], current: &str) -> String {
    let mut html = String::new();
    let mut group = "";
    for page in pages {
        if page.group != group {
            if !group.is_empty() {
                html.push_str("</ul>");
            }
            group = &page.group;
            let _ = write!(html, "<p class=\"nav-group\">{}</p><ul>", escape(group));
        }
        let active = if page.route == current {
            " aria-current=\"page\""
        } else {
            ""
        };
        let _ = write!(
            html,
            "<li><a href=\"{}\"{active}>{}</a></li>",
            page.route,
            escape(&page.label)
        );
    }
    if !group.is_empty() {
        html.push_str("</ul>");
    }
    html
}

fn shell(title: &str, route: &str, body: &str, class: &str) -> String {
    format!(
        r##"<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="description" content="Metaharness: one interface to agent harnesses. Observable runs, explicit control and evidence from the harness's own record.">
<meta name="theme-color" content="#101b22">
<link rel="canonical" href="https://beyond10x.github.io{route}">
<link rel="icon" type="image/svg+xml" href="/metaharness/img/logo.svg">
<link rel="stylesheet" href="/metaharness/styles.css">
<title>{} — Metaharness</title>
</head>
<body class="{class}">
<a class="skip" href="#main">Skip to content</a>
<header class="topbar"><a class="wordmark" href="/metaharness/"><img src="/metaharness/img/logo.svg" width="32" height="32" alt="">Metaharness<span class="byline">/ beyond10x</span></a><nav aria-label="Primary"><a href="/metaharness/docs/">Documentation</a><a href="/metaharness/docs/harnesses/b10x/">b10x</a><a href="https://beyond10x.github.io/">Ecosystem ↗</a><a href="https://github.com/beyond10x/metaharness">GitHub ↗</a></nav></header>
{body}
<footer><p>Metaharness · One interface. An observed record.</p><div><a href="/metaharness/docs/status/">Status &amp; evidence</a><a href="https://github.com/beyond10x/metaharness">Source ↗</a><a href="https://beyond10x.github.io/">beyond10x ↗</a></div></footer>
</body>
</html>
"##,
        escape(title)
    )
}

fn document(page: &Page, pages: &[Page]) -> String {
    let nav = navigation(pages, &page.route);
    let mut toc = String::new();
    for heading in page.headings.iter().filter(|h| h.level == 2) {
        let _ = write!(
            toc,
            "<li><a href=\"#{}\">{}</a></li>",
            escape(&heading.id),
            escape(&heading.label)
        );
    }
    let toc = if toc.is_empty() {
        String::new()
    } else {
        format!(
            "<details class=\"page-outline\"><summary>On this page</summary><nav aria-label=\"On this page\"><ul>{toc}</ul></nav></details>"
        )
    };
    let position = pages
        .iter()
        .position(|p| p.route == page.route)
        .expect("page belongs to site");
    let mut adjacent = String::new();
    if position > 0 {
        let prev = &pages[position - 1];
        let _ = write!(
            adjacent,
            "<a rel=\"prev\" href=\"{}\"><span>Previous</span>← {}</a>",
            prev.route,
            escape(&prev.label)
        );
    }
    if let Some(next) = pages.get(position + 1) {
        let _ = write!(
            adjacent,
            "<a rel=\"next\" href=\"{}\"><span>Next</span>{} →</a>",
            next.route,
            escape(&next.label)
        );
    }
    let body = format!(
        r#"<div class="docs-layout">
<aside class="sidebar"><nav aria-label="Documentation">{nav}</nav></aside>
<main id="main" tabindex="-1" class="doc-main">
<details class="mobile-navigation"><summary>Browse documentation</summary><nav aria-label="Mobile documentation">{nav}</nav></details>
<p class="breadcrumb"><a href="/metaharness/docs/">Docs</a> / {}</p>
{toc}<article class="markdown">{}</article>
<nav class="adjacent" aria-label="Adjacent pages">{adjacent}</nav>
</main></div>"#,
        escape(&page.group),
        page.body
    );
    shell(&page.title, &page.route, &body, "documentation")
}

fn render_site(sources: &[Source<'_>], landing: &str, commit: &str) -> Result<Files, String> {
    check_commit(commit)?;
    let pages: Vec<_> = sources.iter().map(render::page).collect::<Result<_, _>>()?;
    let mut files = Files::new();
    for page in &pages {
        let path = format!(
            "{}index.html",
            page.route.strip_prefix(BASE).ok_or("wrong base route")?
        );
        if files
            .insert(path, document(page, &pages).into_bytes())
            .is_some()
        {
            return Err(format!("duplicate documentation route {}", page.route));
        }
    }
    files.insert(
        "index.html".into(),
        shell(
            "One interface to many agent harnesses",
            BASE,
            landing,
            "landing",
        )
        .into_bytes(),
    );
    files.insert("styles.css".into(), CSS.as_bytes().to_vec());
    files.insert("img/logo.svg".into(), LOGO.as_bytes().to_vec());
    files.insert(".nojekyll".into(), Vec::new());
    let provenance = serde_json::json!({"schema":"b10x-project-site/v1", "repository":"metaharness", "commit":commit, "baseUrl":BASE});
    files.insert(
        ".well-known/b10x-site.json".into(),
        serde_json::to_vec_pretty(&provenance).map_err(|e| e.to_string())?,
    );
    validate::site(&files)?;
    let inventory = validate::inventory(&files, commit)?;
    files.insert(".well-known/b10x-routes.json".into(), inventory);
    Ok(files)
}

/// Validate all thirteen documentation pages, the landing page, local links and assets.
/// # Errors
/// Returns an error for invalid Markdown, duplicate routes or broken links and fragments.
pub fn check() -> Result<(), String> {
    render_site(
        content::PAGES,
        LANDING,
        "1111111111111111111111111111111111111111",
    )?;
    println!("Metaharness: 13 documentation pages, landing, links, anchors and assets valid");
    Ok(())
}

/// Build the site with exact provenance, replacing only complete generated output.
/// The output directory is the project-site root mounted at `/metaharness/`.
/// # Errors
/// Returns an error for invalid source, invalid commit, foreign or incomplete output, symlinks or I/O failure.
pub fn build(out: &Path, commit: &str) -> Result<(), String> {
    let files = render_site(content::PAGES, LANDING, commit)?;
    output::write(out, &files)?;
    println!(
        "Built 13 documentation pages and landing at {}",
        out.display()
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    const COMMIT: &str = "1234567890123456789012345678901234567890";

    // Test seams are introduced before their implementation.
    fn fixture_site(
        pages: &[(&str, &str)],
        commit: &str,
    ) -> Result<BTreeMap<String, Vec<u8>>, String> {
        let mut sources: Vec<_> = pages
            .iter()
            .map(|(key, markdown)| Source {
                key,
                markdown,
                group: "Test",
            })
            .collect();
        for key in ["index", "status", "harnesses/b10x"] {
            if !sources.iter().any(|source| source.key == key) {
                sources.push(Source {
                    key,
                    markdown: "# Placeholder",
                    group: "Test",
                });
            }
        }
        render_site(&sources, r#"<main id="main">Fixture</main>"#, commit)
    }

    #[test]
    fn legacy_heading_ids_preserve_punctuation_code_and_duplicates() {
        let site = fixture_site(&[("index", "# Example\n## Who decides: `--decisions`\n## `observe` — measuring a harness nobody is steering\n## Same\n## Same\n")], COMMIT).unwrap();
        let html = String::from_utf8_lossy(&site["docs/index.html"]);
        for id in [
            "who-decides---decisions",
            "observe--measuring-a-harness-nobody-is-steering",
            "same",
            "same-1",
        ] {
            assert!(html.contains(&format!("id=\"{id}\"")), "{id}");
        }
    }

    #[test]
    fn broken_routes_anchors_assets_and_unsafe_links_are_refused() {
        for (link, expected) in [
            ("missing.md", "missing route"),
            ("#missing", "missing anchor"),
            ("/metaharness/missing.svg", "missing route"),
            ("javascript:alert(1)", "unsafe link"),
        ] {
            let markdown = format!("# Test\n[broken]({link})");
            let error = fixture_site(&[("index", &markdown)], COMMIT).unwrap_err();
            assert!(error.contains(expected), "{link}: {error}");
        }
    }

    #[test]
    fn duplicate_and_traversing_routes_are_refused() {
        for pages in [
            vec![("index", "# A"), ("index", "# B")],
            vec![("../escape", "# A")],
        ] {
            let error = fixture_site(&pages, COMMIT).unwrap_err();
            assert!(error.contains("route"), "{error}");
        }
    }

    #[test]
    fn invalid_provenance_is_refused() {
        for commit in [
            "",
            "main",
            "0000000000000000000000000000000000000000",
            "a99ca289123456789abcdef0123456789abcdef01z",
        ] {
            let error = fixture_site(&[("index", "# A")], commit).unwrap_err();
            assert!(error.contains("commit"), "{error}");
        }
    }

    #[test]
    fn builds_are_byte_deterministic_and_render_tables_code_and_admonitions() {
        let pages = [(
            "index",
            "# Example\n\n| A | B |\n|---|---|\n| one | two |\n\n```rust\nlet x = 1 < 2;\n```\n\n:::warning Keep this warning\nA **real** boundary.\n:::\n",
        )];
        let first = fixture_site(&pages, COMMIT).unwrap();
        assert_eq!(first, fixture_site(&pages, COMMIT).unwrap());
        let html = String::from_utf8_lossy(&first["docs/index.html"]);
        for expected in [
            "<table>",
            "language-rust",
            "1 &lt; 2",
            "admonition warning",
            "Keep this warning",
            "<strong>real</strong>",
        ] {
            assert!(html.contains(expected), "{expected}");
        }
    }
}
