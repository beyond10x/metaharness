//! Validate actual HTML tokens, not attribute spelling or source substrings.
use crate::{BASE, Files};
use html5ever::tokenizer::{
    BufferQueue, TagKind, Token, TokenSink, TokenSinkResult, Tokenizer, TokenizerOpts,
};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet},
};

#[derive(Default)]
struct HtmlFacts {
    ids: Vec<String>,
    links: Vec<String>,
    doctype: bool,
    language: bool,
    viewport: bool,
    error: Option<String>,
}

#[derive(Default)]
struct FactsSink(RefCell<HtmlFacts>);

impl TokenSink for FactsSink {
    type Handle = ();

    fn process_token(&self, token: Token, _line_number: u64) -> TokenSinkResult<()> {
        let mut facts = self.0.borrow_mut();
        match token {
            Token::DoctypeToken(doctype) => facts.doctype = doctype.name.as_deref() == Some("html"),
            Token::TagToken(tag) if tag.kind == TagKind::StartTag => {
                if ["script", "iframe", "object", "embed", "base", "style"]
                    .contains(&tag.name.as_ref())
                {
                    facts.error =
                        Some(format!("active or routing HTML is forbidden: {}", tag.name));
                }
                for attr in tag.attrs {
                    let name = attr.name.local.as_ref();
                    let value = attr.value.as_ref();
                    if name.starts_with("on")
                        || ["srcdoc", "http-equiv", "srcset", "imagesrcset", "style"]
                            .contains(&name)
                    {
                        facts.error = Some(format!(
                            "active or unsupported HTML attribute is forbidden: {name}"
                        ));
                    }
                    match name {
                        "id" => facts.ids.push(value.to_owned()),
                        "href" | "src" => facts.links.push(value.to_owned()),
                        "lang" if tag.name.as_ref() == "html" && value == "en" => {
                            facts.language = true;
                        }
                        "name" if tag.name.as_ref() == "meta" && value == "viewport" => {
                            facts.viewport = true;
                        }
                        _ => {}
                    }
                }
            }
            Token::ParseError(error) => facts.error = Some(format!("invalid HTML: {error}")),
            _ => {}
        }
        TokenSinkResult::Continue
    }
}

fn facts(bytes: &[u8]) -> Result<HtmlFacts, String> {
    let html = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
    let input = BufferQueue::default();
    input.push_back(html.into());
    let tokenizer = Tokenizer::new(FactsSink::default(), TokenizerOpts::default());
    let _ = tokenizer.feed(&input);
    tokenizer.end();
    let facts = tokenizer.sink.0.into_inner();
    if let Some(error) = &facts.error {
        return Err(error.clone());
    }
    if !facts.doctype || !facts.language || !facts.viewport {
        return Err("missing document declaration, language or viewport".into());
    }
    Ok(facts)
}

fn target(current: &str, link: &str) -> Result<Option<(String, Option<String>)>, String> {
    let link = link.trim();
    let link = link
        .strip_prefix("https://beyond10x.github.io")
        .filter(|path| path.starts_with(BASE))
        .unwrap_or(link);
    if link.starts_with("https://") || link.starts_with("http://") || link.starts_with("mailto:") {
        return Ok(None);
    }
    if link.contains(':') || link.starts_with("//") || link.contains('\\') {
        return Err(format!("unsafe link {link}"));
    }
    let (path, fragment) = link
        .split_once('#')
        .map_or((link, None), |(p, f)| (p, Some(f.to_owned())));
    let path = path.split_once('?').map_or(path, |(p, _)| p);
    let key = if path.is_empty() {
        current.to_owned()
    } else {
        let relative = path
            .strip_prefix(BASE)
            .ok_or_else(|| format!("wrong base or unsupported link {link}"))?;
        if relative.split('/').any(|part| part == "." || part == "..") {
            return Err(format!("unsafe link {link}"));
        }
        if relative.is_empty() || relative.ends_with('/') {
            format!("{relative}index.html")
        } else if relative
            .rsplit('/')
            .next()
            .is_some_and(|name| !name.contains('.'))
        {
            format!("{relative}/index.html")
        } else {
            relative.to_owned()
        }
    };
    Ok(Some((key, fragment)))
}

fn documents(files: &Files) -> Result<BTreeMap<&str, HtmlFacts>, String> {
    files
        .iter()
        .filter(|(path, _)| {
            std::path::Path::new(path)
                .extension()
                .is_some_and(|ext| ext == "html")
        })
        .map(|(path, bytes)| {
            Ok((
                path.as_str(),
                facts(bytes).map_err(|e| format!("{path}: {e}"))?,
            ))
        })
        .collect()
}

pub fn site(files: &Files) -> Result<(), String> {
    for (path, bytes) in files.iter().filter(|(path, _)| {
        std::path::Path::new(path)
            .extension()
            .is_some_and(|ext| ext == "css")
    }) {
        crate::css::check(bytes).map_err(|error| format!("{path}: {error}"))?;
    }
    let pages = documents(files)?;
    let mut anchors = BTreeMap::new();
    for (path, page) in &pages {
        let unique: BTreeSet<_> = page.ids.iter().collect();
        if unique.len() != page.ids.len() {
            return Err(format!("duplicate anchor in {path}"));
        }
        anchors.insert(*path, unique);
    }
    for (path, page) in &pages {
        for link in &page.links {
            if let Some((destination, fragment)) = target(path, link)? {
                if !files.contains_key(&destination) {
                    return Err(format!("missing route {link} in {path}"));
                }
                if let Some(fragment) = fragment.filter(|f| !f.is_empty())
                    && !anchors
                        .get(destination.as_str())
                        .is_some_and(|ids| ids.contains(&fragment))
                {
                    return Err(format!("missing anchor {link} in {path}"));
                }
            }
        }
    }
    Ok(())
}

pub fn inventory(files: &Files, commit: &str) -> Result<Vec<u8>, String> {
    let mut routes = BTreeMap::new();
    for (path, facts) in documents(files)? {
        let path = format!(
            "{BASE}{}",
            path.strip_suffix("index.html")
                .ok_or("noncanonical HTML path")?
        );
        let anchors: BTreeSet<_> = facts.ids.into_iter().collect();
        routes.insert(path, anchors);
    }
    let routes: Vec<_> = routes
        .into_iter()
        .map(|(path, anchors)| serde_json::json!({"path":path,"anchors":anchors}))
        .collect();
    serde_json::to_vec_pretty(&serde_json::json!({"schema":"b10x-project-routes/v1","repository":"metaharness","commit":commit,"baseUrl":BASE,"routes":routes})).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;

    fn document(body: &str) -> Files {
        BTreeMap::from([("index.html".into(), format!("<!doctype html><html lang=\"en\"><meta name=\"viewport\" content=\"width=device-width\"><body>{body}</body></html>").into_bytes())])
    }

    #[test]
    fn html_attribute_spellings_and_own_absolute_links_cannot_hide_broken_targets() {
        for body in [
            "<a href='/metaharness/missing/'>broken</a>",
            "<a HREF=/metaharness/missing/>broken</a>",
            "<a\nhref = \"/metaharness/missing/\">broken</a>",
            "<a href=\"https://beyond10x.github.io/metaharness/missing/\">broken</a>",
        ] {
            assert!(site(&document(body)).is_err(), "missed {body}");
        }
    }

    #[test]
    fn duplicate_ids_and_active_html_are_refused() {
        for body in [
            "<p id='same'></p><p id=\"same\"></p>",
            "<a href='#' onclick='alert(1)'>active</a>",
            "<script>alert(1)</script>",
        ] {
            assert!(site(&document(body)).is_err(), "missed {body}");
        }
    }

    #[test]
    fn unsupported_html_asset_surfaces_are_refused() {
        for body in [
            "<img srcset='/metaharness/missing.png 2x' alt='Missing image'>",
            "<link imagesrcset='/metaharness/missing.png 2x'>",
            "<div style=\"background:url('/metaharness/missing.png')\"></div>",
            "<style>body {background:url('/metaharness/missing.png')}</style>",
        ] {
            assert!(site(&document(body)).is_err(), "missed {body}");
        }
    }

    #[test]
    fn css_asset_imports_and_escaped_urls_are_refused() {
        for css in [
            "body {background-image:url('/metaharness/missing.png')}",
            "@import '/metaharness/missing.css';",
            r"@\69mport '/metaharness/missing.css';",
            r"body {background:u\72l('/metaharness/missing.png')}",
            "@media screen {div {background: URL(/metaharness/missing.png)}}",
            "body {background:image-set('/metaharness/missing.png' 2x)}",
        ] {
            let mut files = document("<link rel='stylesheet' href='/metaharness/styles.css'>");
            files.insert("styles.css".into(), css.as_bytes().to_vec());
            assert!(site(&files).is_err(), "missed {css}");
        }
    }
}
