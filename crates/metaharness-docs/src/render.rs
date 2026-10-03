//! Markdown rendering with stable Docusaurus-compatible heading fragments.
use crate::{Source, escape, route};
use pulldown_cmark::{CowStr, Event, Options, Parser, Tag, TagEnd, html};
use std::collections::BTreeSet;
use std::fmt::Write as _;

pub struct Heading {
    pub id: String,
    pub label: String,
    pub level: u8,
}

pub struct Page {
    pub route: String,
    pub title: String,
    pub label: String,
    pub group: String,
    pub body: String,
    pub headings: Vec<Heading>,
}

fn frontmatter(markdown: &str) -> Result<(&str, &str, &str), String> {
    if let Some(rest) = markdown.strip_prefix("---\n") {
        let (metadata, body) = rest.split_once("\n---\n").ok_or("unclosed frontmatter")?;
        let value = |name| metadata.lines().find_map(|line| line.strip_prefix(name));
        let title = value("title: ").ok_or("missing frontmatter title")?;
        Ok((title, value("sidebar_label: ").unwrap_or(title), body))
    } else {
        Ok(("Documentation", "Documentation", markdown))
    }
}

// Docusaurus's GitHub heading convention: lowercase, remove punctuation (but keep
// hyphens/underscores), replace each space separately, then suffix collisions.
fn slug(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .filter_map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                Some(c)
            } else if c.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .collect()
}

fn admonitions(markdown: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut fence: Option<(char, usize)> = None;
    let mut open = false;
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        if let Some(character) = trimmed.chars().next().filter(|c| *c == '`' || *c == '~') {
            let count = trimmed.chars().take_while(|c| *c == character).count();
            if count >= 3 {
                match fence {
                    Some((kind, length)) if kind == character && count >= length => fence = None,
                    None => fence = Some((character, count)),
                    _ => {}
                }
            }
        }
        if fence.is_none()
            && let Some(spec) = trimmed.strip_prefix(":::")
        {
            if spec.is_empty() {
                if !open {
                    return Err("closing unopened admonition".into());
                }
                output.push_str("\n</aside>\n\n");
                open = false;
            } else {
                if open {
                    return Err("nested admonitions are unsupported".into());
                }
                let (kind, title) = spec.split_once(' ').unwrap_or((spec, spec));
                if !["info", "note", "warning", "danger", "tip"].contains(&kind) {
                    return Err(format!("unsupported admonition {kind}"));
                }
                let _ = write!(
                    output,
                    "\n<aside class=\"admonition {kind}\" aria-label=\"{}\">\n\n**{title}**\n\n",
                    escape(kind)
                );
                open = true;
            }
        } else {
            output.push_str(line);
            output.push('\n');
        }
    }
    if open {
        return Err("unclosed admonition".into());
    }
    Ok(output)
}

fn markdown_link(key: &str, destination: &str) -> Result<String, String> {
    if destination.starts_with('#') || destination.starts_with('/') || destination.contains(':') {
        return Ok(destination.to_owned());
    }
    let (path, fragment) = destination
        .split_once('#')
        .map_or((destination, None), |(p, f)| (p, Some(f)));
    let Some(path) = path.strip_suffix(".md") else {
        return Err(format!("unsupported relative link {destination}"));
    };
    let mut segments: Vec<_> = key.split('/').collect();
    segments.pop();
    for segment in path.split('/') {
        match segment {
            "." => {}
            ".." => {
                segments.pop().ok_or("link escapes documentation root")?;
            }
            "" => return Err("empty link path segment".into()),
            _ => segments.push(segment),
        }
    }
    let mut resolved = route(&segments.join("/"))?;
    if let Some(fragment) = fragment {
        resolved.push('#');
        resolved.push_str(fragment);
    }
    Ok(resolved)
}

pub fn page(source: &Source<'_>) -> Result<Page, String> {
    let (title, label, body) = frontmatter(source.markdown)?;
    let markdown = admonitions(body)?;
    let options =
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_HEADING_ATTRIBUTES;
    let mut events: Vec<_> = Parser::new_ext(&markdown, options).collect();
    let mut headings = Vec::new();
    let mut ids = BTreeSet::new();
    for index in 0..events.len() {
        if let Event::Start(Tag::Heading { level, id, .. }) = &events[index] {
            let level = *level as u8;
            let text: String = events[index + 1..]
                .iter()
                .take_while(|e| !matches!(e, Event::End(TagEnd::Heading(_))))
                .filter_map(|e| match e {
                    Event::Text(text) | Event::Code(text) => Some(text.as_ref()),
                    _ => None,
                })
                .collect();
            let base = id.as_ref().map_or_else(|| slug(&text), ToString::to_string);
            let mut fragment = base.clone();
            let mut suffix = 0;
            while !ids.insert(fragment.clone()) {
                if id.is_some() {
                    return Err(format!("duplicate explicit anchor {base}"));
                }
                suffix += 1;
                fragment = format!("{base}-{suffix}");
            }
            if let Event::Start(Tag::Heading { id, .. }) = &mut events[index] {
                *id = Some(CowStr::from(fragment.clone()));
            }
            headings.push(Heading {
                id: fragment,
                label: text,
                level,
            });
        }
        if let Event::Start(Tag::Link { dest_url, .. } | Tag::Image { dest_url, .. }) =
            &mut events[index]
        {
            *dest_url = CowStr::from(markdown_link(source.key, dest_url)?);
        }
    }
    let mut body = String::new();
    html::push_html(&mut body, events.into_iter());
    // Native keyboard scrolling is available for long code and reference tables.
    body = body.replace("<pre>", "<pre tabindex=\"0\" aria-label=\"Code example\">")
        .replace("<table>", "<div class=\"table-wrap\" tabindex=\"0\" role=\"region\" aria-label=\"Reference table\"><table>")
        .replace("</table>", "</table></div>");
    Ok(Page {
        route: route(source.key)?,
        title: title.into(),
        label: label.into(),
        group: source.group.into(),
        body,
        headings,
    })
}
