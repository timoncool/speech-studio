//! Книга → главы с текстом: EPUB (оглавление и порядок spine), FB2 (секции), PDF (закладки, иначе страницы
//! целиком), DOCX (стили заголовков), HTML и Markdown (заголовки), TXT в любой кодировке («Глава N», «Chapter N»).
//! Обложка, титул, оглавление и выходные данные помечаются к пропуску; окно показывает структуру до синтеза.

use std::io::Cursor;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Chapter {
    pub title: String,
    pub text: String,
    /// Почему глава, скорее всего, не для озвучки: `cover`, `front` (титул, выходные данные), `toc`, `empty`.
    pub skip: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct Book {
    pub title: String,
    pub author: String,
    pub lang: String,
    pub chapters: Vec<Chapter>,
    #[serde(skip)]
    pub cover: Option<(Vec<u8>, String)>,
    /// What was taken out of the text because a book's reader does not say it: footnote marks and site addresses.
    #[serde(default)]
    pub removed: Removed,
}

#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
pub struct Removed {
    pub notes: usize,
    pub links: usize,
}

static NOTE_MARK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\[\d{1,3}\]|\[\*\]|[¹²³⁴⁵⁶⁷⁸⁹⁰]+").expect("note mark"));
static LINK: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)[ \t]*\(?\b(?:https?://|www\.)[^\s<>«»]*[^\s<>«».,;:!?)]\)?").expect("link"));

static CAPITAL_WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[А-ЯЁ][а-яё]{2,}\b").expect("capital word"));

/// The names of a book, as far as the text shows them: words with a capital letter where a sentence does not begin,
/// met at least `min` times, the most frequent first, at most `limit`.
pub fn names(chapters: &[Chapter], min: usize, limit: usize) -> Vec<(String, usize)> {
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for c in chapters.iter().filter(|c| c.skip.is_none()) {
        for m in CAPITAL_WORD.find_iter(&c.text) {
            let before = c.text[..m.start()].trim_end_matches([' ', '\t', '«', '"', '(', '—', '–', '-']);
            if !(before.is_empty() || before.ends_with(['.', '!', '?', '…', '\n', ':'])) {
                *counts.entry(m.as_str().to_string()).or_default() += 1;
            }
        }
    }
    let mut out: Vec<(String, usize)> = counts.into_iter().filter(|(_, n)| *n >= min).collect();
    out.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    out.truncate(limit);
    out
}

/// Footnote marks and site addresses out of a chapter, counted. A superscript after a digit or a unit of measure is a
/// power («10³», «50 м²»), not a mark, and stays.
fn strip_marks(text: &str, removed: &mut Removed) -> String {
    let t = NOTE_MARK.replace_all(text, |c: &regex::Captures| {
        let m = c.get(0).expect("the match");
        let before: String = text[..m.start()].chars().rev().take_while(|ch| ch.is_alphanumeric()).collect::<Vec<_>>().into_iter().rev().collect();
        let power = m.as_str().starts_with(|ch: char| "¹²³⁴⁵⁶⁷⁸⁹⁰".contains(ch))
            && (before.chars().last().is_some_and(|ch| ch.is_ascii_digit()) || UNITS.contains(&before.to_lowercase().as_str()));
        if power {
            return m.as_str().to_string();
        }
        removed.notes += 1;
        String::new()
    });
    removed.links += LINK.find_iter(&t).count();
    LINK.replace_all(&t, "").into_owned()
}

/// Units a power follows: «м²», «км²», «см³».
const UNITS: [&str; 14] = ["м", "см", "мм", "км", "дм", "дюйм", "фут", "m", "cm", "mm", "km", "dm", "ft", "in"];


static SPACES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[ \t\u{a0}\u{2009}\u{202f}]+").expect("spaces"));
static BLANKS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\n{3,}").expect("blanks"));
static FRONT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^\s*(cover|обложка|title ?page|титул|copyright|все права|выходные данные|аннотация к изданию|colophon|imprint|дизайн обложки)").expect("front")
});
static TOC: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^\s*(оглавление|содержание|table of contents|contents|índice|sommaire|inhalt)\s*$").expect("toc"));
static TXT_HEADING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?im)^[ \t]*((?:глава|часть|книга|пролог|эпилог|послесловие|предисловие|chapter|part|prologue|epilogue|capítulo|chapitre|kapitel)\b[^\n]{0,80}|[IVXLC]{1,6}\.?|\d{1,3}\.)[ \t]*$").expect("txt heading")
});

/// Абзацы через пустую строку, пробелы внутри схлопнуты.
pub fn tidy(text: &str) -> String {
    let t = text.replace("\r\n", "\n").replace('\r', "\n");
    let lines: Vec<String> = t.lines().map(|l| SPACES.replace_all(l.trim(), " ").into_owned()).collect();
    let joined = lines.join("\n");
    BLANKS.replace_all(joined.trim(), "\n\n").into_owned()
}

fn mark_skips(chapters: &mut [Chapter]) {
    let n = chapters.len();
    for (i, c) in chapters.iter_mut().enumerate() {
        let chars = c.text.chars().filter(|ch| ch.is_alphanumeric()).count();
        c.skip = if chars == 0 {
            Some("empty".into())
        } else if TOC.is_match(&c.title) || TOC.is_match(c.text.lines().next().unwrap_or("")) {
            Some("toc".into())
        } else if FRONT.is_match(&c.title) || (i < 2 && n > 2 && chars < 300 && FRONT.is_match(&c.text)) {
            Some("front".into())
        } else if c.title.to_lowercase().contains("cover") || c.title.to_lowercase().contains("обложк") {
            Some("cover".into())
        } else {
            None
        };
    }
}

pub fn parse(file_name: &str, bytes: &[u8]) -> Result<Book, String> {
    let ext = std::path::Path::new(file_name).extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    let mut book = match ext.as_str() {
        "epub" => epub(bytes)?,
        "fb2" => fb2(&decode(bytes))?,
        "pdf" => pdf(bytes)?,
        "docx" => docx(bytes)?,
        "html" | "htm" => html(&decode(bytes)),
        "md" => markdown(&decode(bytes)),
        "txt" => txt(&decode(bytes)),
        _ => return Err(t!("book-unknown-format", name = file_name.to_string())),
    };
    if book.title.trim().is_empty() {
        book.title = std::path::Path::new(file_name).file_stem().and_then(|s| s.to_str()).unwrap_or("book").to_string();
    }
    let mut removed = std::mem::take(&mut book.removed);
    for c in &mut book.chapters {
        c.text = tidy(&strip_marks(&c.text, &mut removed));
        c.title = SPACES.replace_all(c.title.trim(), " ").into_owned();
    }
    book.removed = removed;
    book.chapters.retain(|c| !c.text.is_empty() || !c.title.is_empty());
    if book.chapters.is_empty() {
        return Err(t!("book-empty", name = file_name.to_string()));
    }
    mark_skips(&mut book.chapters);
    Ok(book)
}

/// Байты текста в строку: BOM, иначе угадывание кодировки (старые русские книги — cp1251, KOI8-R).
pub fn decode(bytes: &[u8]) -> String {
    if let Some((enc, bom)) = encoding_rs::Encoding::for_bom(bytes) {
        return enc.decode_without_bom_handling(&bytes[bom..]).0.into_owned();
    }
    if let Ok(s) = std::str::from_utf8(bytes) {
        return s.to_string();
    }
    let mut det = chardetng::EncodingDetector::new();
    det.feed(bytes, true);
    det.guess(None, true).decode(bytes).0.into_owned()
}

/// Текст HTML-фрагмента: абзацы и заголовки — с новой строки, скрипты и стили — мимо.
fn html_text(doc: &scraper::Html) -> String {
    let mut out = String::new();
    fn walk(node: ego_tree::NodeRef<scraper::Node>, out: &mut String) {
        for child in node.children() {
            match child.value() {
                scraper::Node::Text(t) => out.push_str(t),
                scraper::Node::Element(e) => {
                    let name = e.name();
                    if matches!(name, "script" | "style" | "head" | "title" | "nav") {
                        continue;
                    }
                    // a footnote and its mark: EPUB 3 and DPUB-ARIA say so, older books put the number in <sup>
                    let kind = e.attr("epub:type").or_else(|| e.attr("role")).unwrap_or("");
                    if ["noteref", "footnote", "endnote", "rearnote", "doc-noteref", "doc-footnote", "doc-endnote"].iter().any(|k| kind.contains(k))
                        || (name == "sup" && child.children().all(|c| c.value().as_text().is_some_and(|t| t.trim().chars().all(|ch| ch.is_ascii_digit() || ch == '*' || ch == '[' || ch == ']'))))
                    {
                        continue;
                    }
                    let block = matches!(name, "p" | "div" | "br" | "li" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" | "blockquote" | "section" | "tr" | "pre");
                    if block {
                        out.push_str("\n\n");
                    }
                    walk(child, out);
                    if block {
                        out.push_str("\n\n");
                    }
                }
                _ => {}
            }
        }
    }
    walk(doc.tree.root(), &mut out);
    out
}

fn first_heading(doc: &scraper::Html) -> Option<String> {
    let sel = scraper::Selector::parse("h1, h2, h3").ok()?;
    doc.select(&sel).next().map(|e| e.text().collect::<String>()).map(|s| SPACES.replace_all(s.trim(), " ").into_owned()).filter(|s| !s.is_empty())
}

fn epub(bytes: &[u8]) -> Result<Book, String> {
    use rbook::Epub;
    let book = Epub::read(Cursor::new(bytes.to_vec())).map_err(|e| t!("book-damaged", detail = e.to_string()))?;
    let meta = book.metadata();
    let mut out = Book {
        title: meta.title().map(|t| t.value().to_string()).unwrap_or_default(),
        author: meta.creators().map(|c| c.value().to_string()).collect::<Vec<_>>().join(", "),
        lang: meta.language().map(|l| l.value().to_string()).unwrap_or_default(),
        ..Default::default()
    };
    let mut labels: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    if let Some(root) = book.toc().contents() {
        for e in root.flatten() {
            if let Some(m) = e.manifest_entry() {
                labels.entry(m.href().as_str().to_string()).or_insert_with(|| e.label().to_string());
            }
        }
    }
    out.cover = book.manifest().cover_image().and_then(|c| c.read_bytes().ok().map(|b| (b, c.media_type().to_string())));
    let reader = book.reader_builder().linear_behavior(rbook::epub::reader::LinearBehavior::LinearOnly).create();
    for item in reader {
        let item = item.map_err(|e| t!("book-damaged", detail = e.to_string()))?;
        let href = item.manifest_entry().href().as_str().to_string();
        let doc = scraper::Html::parse_document(item.content());
        let title = labels.get(&href).cloned().or_else(|| first_heading(&doc)).unwrap_or_default();
        out.chapters.push(Chapter { title, text: html_text(&doc), skip: None });
    }
    Ok(out)
}

fn fb2(xml: &str) -> Result<Book, String> {
    let doc = roxmltree::Document::parse_with_options(xml, roxmltree::ParsingOptions { allow_dtd: true, ..Default::default() })
        .map_err(|e| t!("book-damaged", detail = e.to_string()))?;
    let root = doc.root_element();
    fn find<'a, 'i>(n: roxmltree::Node<'a, 'i>, name: &str) -> Option<roxmltree::Node<'a, 'i>> {
        n.children().find(|c| c.is_element() && c.tag_name().name() == name)
    }
    fn text_of(n: roxmltree::Node) -> String {
        n.descendants().filter(|d| d.is_text()).map(|d| d.text().unwrap_or("")).collect()
    }
    let mut out = Book::default();
    if let Some(info) = find(root, "description").and_then(|d| find(d, "title-info")) {
        out.title = find(info, "book-title").map(text_of).unwrap_or_default();
        out.lang = find(info, "lang").map(text_of).unwrap_or_default();
        out.author = info
            .children()
            .filter(|c| c.tag_name().name() == "author")
            .map(|a| ["first-name", "middle-name", "last-name"].iter().filter_map(|k| find(a, k).map(text_of)).collect::<Vec<_>>().join(" "))
            .collect::<Vec<_>>()
            .join(", ");
        if let Some(href) = find(info, "coverpage").and_then(|c| find(c, "image")).and_then(|i| i.attributes().find(|a| a.name() == "href").map(|a| a.value().trim_start_matches('#').to_string())) {
            if let Some(bin) = root.children().find(|c| c.tag_name().name() == "binary" && c.attribute("id") == Some(href.as_str())) {
                use base64::Engine;
                let data: String = text_of(bin).split_whitespace().collect();
                if let Ok(b) = base64::engine::general_purpose::STANDARD.decode(data) {
                    out.cover = Some((b, bin.attribute("content-type").unwrap_or("image/jpeg").to_string()));
                }
            }
        }
    }
    fn section_text(n: roxmltree::Node, out: &mut String) {
        for c in n.children().filter(|c| c.is_element()) {
            match c.tag_name().name() {
                "title" => {}
                "p" | "v" | "subtitle" | "text-author" => {
                    let note = |d: &roxmltree::Node| d.ancestors().any(|a| a.tag_name().name() == "a" && a.attribute("type") == Some("note"));
                    out.push_str(&c.descendants().filter(|d| d.is_text() && !note(d)).map(|d| d.text().unwrap_or("")).collect::<String>());
                    out.push_str("\n\n");
                }
                "empty-line" => out.push_str("\n\n"),
                _ => section_text(c, out),
            }
        }
    }
    for body in root.children().filter(|c| c.tag_name().name() == "body" && c.attribute("name").is_none_or(|n| n != "notes" && n != "comments")) {
        let sections: Vec<roxmltree::Node> = body.children().filter(|c| c.tag_name().name() == "section").collect();
        if sections.is_empty() {
            let mut text = String::new();
            section_text(body, &mut text);
            out.chapters.push(Chapter { title: String::new(), text, skip: None });
        }
        for s in sections {
            let title = find(s, "title").map(|t| t.children().filter(|c| c.is_element()).map(text_of).collect::<Vec<_>>().join(". ")).unwrap_or_default();
            let mut text = String::new();
            section_text(s, &mut text);
            out.chapters.push(Chapter { title, text, skip: None });
        }
    }
    Ok(out)
}

fn pdf(bytes: &[u8]) -> Result<Book, String> {
    let pages = strip_running_lines(pdf_extract::extract_text_from_mem_by_pages(bytes).map_err(|e| t!("book-damaged", detail = e.to_string()))?);
    let mut out = Book::default();
    let toc = lopdf::Document::load_mem(bytes).ok().and_then(|d| d.get_toc().ok()).map(|t| t.toc).unwrap_or_default();
    let top = toc.iter().map(|e| e.level).min().unwrap_or(1);
    let mut marks: Vec<(usize, String)> = toc.iter().filter(|e| e.level == top && e.page >= 1).map(|e| (e.page - 1, e.title.clone())).collect();
    marks.sort_by_key(|m| m.0);
    marks.dedup_by_key(|m| m.0);
    if marks.is_empty() {
        out.chapters.push(Chapter { title: String::new(), text: pages.join("\n\n"), skip: None });
        return Ok(out);
    }
    if marks[0].0 > 0 {
        out.chapters.push(Chapter { title: String::new(), text: pages[..marks[0].0.min(pages.len())].join("\n\n"), skip: None });
    }
    for (k, (page, title)) in marks.iter().enumerate() {
        let end = marks.get(k + 1).map(|m| m.0).unwrap_or(pages.len()).min(pages.len());
        let text = if *page < end { pages[*page..end].join("\n\n") } else { String::new() };
        out.chapters.push(Chapter { title: title.clone(), text, skip: None });
    }
    Ok(out)
}

/// Колонтитулы и номера страниц: строка, которой начинается или кончается больше трети страниц, и голые номера.
fn strip_running_lines(pages: Vec<String>) -> Vec<String> {
    let edge = |p: &String, last: bool| -> Vec<String> {
        let lines: Vec<&str> = p.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        let pick: Vec<&str> = if last { lines.iter().rev().take(2).copied().collect() } else { lines.iter().take(2).copied().collect() };
        pick.into_iter().map(|l| l.trim_matches(|c: char| c.is_ascii_digit() || c.is_whitespace()).to_string()).filter(|l| !l.is_empty()).collect()
    };
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for p in &pages {
        let mut seen = std::collections::HashSet::new();
        for l in edge(p, false).into_iter().chain(edge(p, true)) {
            if seen.insert(l.clone()) {
                *freq.entry(l).or_default() += 1;
            }
        }
    }
    let limit = (pages.len() / 3).max(2);
    let running: std::collections::HashSet<String> = freq.into_iter().filter(|(_, n)| *n > limit).map(|(l, _)| l).collect();
    pages
        .into_iter()
        .map(|p| {
            p.lines()
                .filter(|l| {
                    let t = l.trim();
                    let core = t.trim_matches(|c: char| c.is_ascii_digit() || c.is_whitespace());
                    !(t.chars().all(|c| c.is_ascii_digit()) && !t.is_empty()) && !running.contains(core)
                })
                .collect::<Vec<_>>()
                .join("\n")
        })
        .collect()
}

fn docx(bytes: &[u8]) -> Result<Book, String> {
    use docx_rs::{DocumentChild, ParagraphChild, RunChild};
    let doc = docx_rs::read_docx(bytes).map_err(|e| t!("book-damaged", detail = e.to_string()))?;
    let heading_ids: std::collections::HashSet<String> = doc
        .styles
        .styles
        .iter()
        .filter(|s| s.name.starts_with("Title") || s.name.starts_with("title") || matches!(s.name.get_heading_number(), Some(1 | 2)))
        .map(|s| s.style_id.clone())
        .collect();
    let mut out = Book::default();
    let mut current = Chapter::default();
    for child in &doc.document.children {
        let DocumentChild::Paragraph(p) = child else { continue };
        let text: String = p
            .children
            .iter()
            .filter_map(|c| match c {
                ParagraphChild::Run(r) => Some(r.children.iter().filter_map(|rc| match rc { RunChild::Text(t) => Some(t.text.clone()), RunChild::Tab(_) => Some(" ".into()), _ => None }).collect::<String>()),
                _ => None,
            })
            .collect();
        let style = p.property.style.as_ref().map(|s| s.val.clone()).unwrap_or_default();
        if heading_ids.contains(&style) && !text.trim().is_empty() {
            if !current.text.trim().is_empty() || !current.title.is_empty() {
                out.chapters.push(std::mem::take(&mut current));
            }
            current.title = text;
        } else {
            current.text.push_str(&text);
            current.text.push_str("\n\n");
        }
    }
    out.chapters.push(current);
    Ok(out)
}

fn html(source: &str) -> Book {
    let doc = scraper::Html::parse_document(source);
    let mut out = Book::default();
    if let Ok(sel) = scraper::Selector::parse("title") {
        out.title = doc.select(&sel).next().map(|e| e.text().collect()).unwrap_or_default();
    }
    let parts: Vec<&str> = H_SPLIT.split(source).collect();
    if parts.len() <= 1 {
        out.chapters.push(Chapter { title: first_heading(&doc).unwrap_or_default(), text: html_text(&doc), skip: None });
        return out;
    }
    for part in parts {
        let frag = scraper::Html::parse_fragment(&format!("<h1{part}"));
        let title = first_heading(&frag).unwrap_or_default();
        let mut text = html_text(&frag);
        if let Some(rest) = text.trim_start().strip_prefix(title.as_str()) {
            text = rest.to_string();
        }
        out.chapters.push(Chapter { title, text, skip: None });
    }
    out.chapters.retain(|c| !(c.title.is_empty() && c.text.trim().is_empty()));
    out
}

static H_SPLIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)<h[12]").expect("h split"));

fn markdown(source: &str) -> Book {
    use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
    let mut out = Book::default();
    let mut current = Chapter::default();
    let mut in_heading: Option<HeadingLevel> = None;
    let mut heading = String::new();
    for ev in Parser::new(source) {
        match ev {
            Event::Start(Tag::Heading { level, .. }) if matches!(level, HeadingLevel::H1 | HeadingLevel::H2) => {
                in_heading = Some(level);
                heading.clear();
            }
            Event::End(TagEnd::Heading(level)) if in_heading == Some(level) => {
                in_heading = None;
                if !current.text.trim().is_empty() || !current.title.is_empty() {
                    out.chapters.push(std::mem::take(&mut current));
                }
                current.title = heading.trim().to_string();
            }
            Event::Text(t) | Event::Code(t) => {
                if in_heading.is_some() {
                    heading.push_str(&t);
                } else {
                    current.text.push_str(&t);
                }
            }
            Event::SoftBreak => current.text.push(' '),
            Event::HardBreak | Event::End(TagEnd::Paragraph) | Event::End(TagEnd::Item) | Event::End(TagEnd::Heading(_)) => current.text.push_str("\n\n"),
            _ => {}
        }
    }
    out.chapters.push(current);
    if out.title.is_empty() {
        out.title = out.chapters.first().map(|c| c.title.clone()).unwrap_or_default();
    }
    out
}

fn txt(source: &str) -> Book {
    let mut out = Book::default();
    let heads: Vec<regex::Match> = TXT_HEADING.find_iter(source).collect();
    if heads.len() < 2 {
        out.chapters.push(Chapter { title: String::new(), text: source.to_string(), skip: None });
        return out;
    }
    if heads[0].start() > 0 {
        out.chapters.push(Chapter { title: String::new(), text: source[..heads[0].start()].to_string(), skip: None });
    }
    for (k, h) in heads.iter().enumerate() {
        let end = heads.get(k + 1).map(|n| n.start()).unwrap_or(source.len());
        out.chapters.push(Chapter { title: h.as_str().trim().to_string(), text: source[h.end()..end].to_string(), skip: None });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn txt_chapters_and_cp1251_are_read() {
        let text = "Об издании\n\nГлава 1\nЖил-был кот.\n\nГлава 2\nИ пёс.\n";
        let (bytes, _, _) = encoding_rs::WINDOWS_1251.encode(text);
        let b = parse("book.txt", &bytes).unwrap();
        let titles: Vec<&str> = b.chapters.iter().map(|c| c.title.as_str()).collect();
        assert_eq!(titles, vec!["", "Глава 1", "Глава 2"]);
        assert_eq!(b.chapters[2].text, "И пёс.");
    }

    #[test]
    fn markdown_and_html_headings_cut_chapters() {
        let md = parse("a.md", "# Книга\n\n## Первая\n\nТекст *один*.\n\n## Вторая\n\nТекст два.".as_bytes()).unwrap();
        assert_eq!(md.chapters.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), vec!["Книга", "Первая", "Вторая"]);
        assert_eq!(md.chapters[1].text, "Текст один.");
        let h = parse("a.html", "<html><head><title>T</title><style>p{}</style></head><body><h1>Оглавление</h1><p>1. Раз</p><h2>Раз</h2><p>Абзац <b>один</b>.</p><p>Два.</p></body></html>".as_bytes()).unwrap();
        assert_eq!(h.title, "T");
        assert_eq!(h.chapters.len(), 2);
        assert_eq!(h.chapters[0].skip.as_deref(), Some("toc"));
        assert_eq!(h.chapters[1].text, "Абзац один.\n\nДва.");
    }

    #[test]
    fn names_are_capital_words_inside_sentences_met_often_enough() {
        let text = "Утром Анна ушла. Потом пришёл Олег, и Анна обрадовалась. Вечером Анна и Олег гуляли.\n\nОлег улыбнулся. Вдруг позвонила Мария.";
        let chapters = vec![Chapter { title: String::new(), text: text.into(), skip: None }];
        assert_eq!(names(&chapters, 2, 10), vec![("Анна".to_string(), 3), ("Олег".to_string(), 2)]);
    }

    #[test]
    fn footnote_marks_and_site_addresses_are_taken_out_and_counted() {
        let xml = r##"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns:l="http://www.w3.org/1999/xlink"><description><title-info><book-title>К</book-title></title-info></description><body><section><p>Слово<a l:href="#n1" type="note">[1]</a> тут.</p></section></body></FictionBook>"##;
        assert_eq!(parse("n.fb2", xml.as_bytes()).unwrap().chapters[0].text, "Слово тут.");
        let html = r##"<html><body><h1>Глава</h1><p>Он сказал<a epub:type="noteref" href="#n1">1</a> и ушёл<sup>2</sup>.</p><aside epub:type="footnote"><p>Пояснение.</p></aside></body></html>"##;
        assert_eq!(parse("n.html", html.as_bytes()).unwrap().chapters[0].text, "Он сказал и ушёл.");
        let b = parse("n.txt", "Текст[1] со сноской¹ и ссылкой (https://example.com/page). Конец.".as_bytes()).unwrap();
        assert_eq!(b.chapters[0].text, "Текст со сноской и ссылкой. Конец.");
        assert_eq!(b.removed, Removed { notes: 2, links: 1 });
        let units = parse("u.txt", "Дом в 120 м², куб 10³ см³.".as_bytes()).unwrap();
        assert_eq!(units.chapters[0].text, "Дом в 120 м², куб 10³ см³.");
        assert_eq!(units.removed.notes, 0);
    }

    #[test]
    fn fb2_sections_author_and_notes() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns:l="http://www.w3.org/1999/xlink"><description><title-info><author><first-name>Лев</first-name><last-name>Толстой</last-name></author><book-title>Повесть</book-title><lang>ru</lang></title-info></description><body><section><title><p>Глава 1</p></title><p>Раз.</p><p>Два.</p></section><section><title><p>Глава 2</p></title><p>Три.</p></section></body><body name="notes"><section><p>Сноска</p></section></body></FictionBook>"#;
        let b = parse("b.fb2", xml.as_bytes()).unwrap();
        assert_eq!((b.title.as_str(), b.author.as_str(), b.lang.as_str()), ("Повесть", "Лев Толстой", "ru"));
        assert_eq!(b.chapters.len(), 2);
        assert_eq!(b.chapters[0].text, "Раз.\n\nДва.");
    }

    #[test]
    fn epub_chapters_follow_the_toc() {
        use rbook::epub::{Epub, EpubChapter};
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("b.epub");
        Epub::new()
            .edit()
            .title("Сказка")
            .author("Автор")
            .language("ru")
            .chapter(EpubChapter::new("Начало").xhtml_body("<p>Жил-был кот.</p><p>Он спал.</p>"))
            .chapter(EpubChapter::new("Конец").xhtml_body("<p>Кот проснулся.</p>"))
            .write()
            .save(&path)
            .unwrap();
        let b = parse("b.epub", &std::fs::read(&path).unwrap()).unwrap();
        assert_eq!((b.title.as_str(), b.author.as_str(), b.lang.as_str()), ("Сказка", "Автор", "ru"));
        let real: Vec<&Chapter> = b.chapters.iter().filter(|c| c.skip.is_none()).collect();
        assert_eq!(real.iter().map(|c| c.title.as_str()).collect::<Vec<_>>(), vec!["Начало", "Конец"]);
        assert!(real[0].text.contains("Жил-был кот.") && real[0].text.contains("Он спал."));
    }

    #[test]
    fn docx_headings_cut_chapters() {
        use docx_rs::{Docx, Paragraph, Run, Style, StyleType};
        let mut buf = std::io::Cursor::new(Vec::new());
        Docx::new()
            .add_style(Style::new("Heading1", StyleType::Paragraph).name("Heading 1"))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Глава первая")).style("Heading1"))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Текст главы.")))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Глава вторая")).style("Heading1"))
            .add_paragraph(Paragraph::new().add_run(Run::new().add_text("Ещё текст.")))
            .build()
            .pack(&mut buf)
            .unwrap();
        let b = parse("d.docx", &buf.into_inner()).unwrap();
        assert_eq!(b.chapters.iter().map(|c| (c.title.as_str(), c.text.as_str())).collect::<Vec<_>>(), vec![("Глава первая", "Текст главы."), ("Глава вторая", "Ещё текст.")]);
    }
}

#[cfg(test)]
mod live {
    /// Разбор настоящего PDF: `BOOK_PDF` — путь к файлу.
    #[test]
    #[ignore]
    fn a_real_pdf_gives_chapters() {
        let path = std::env::var("BOOK_PDF").expect("BOOK_PDF");
        let b = super::parse(&path, &std::fs::read(&path).unwrap()).unwrap();
        for c in &b.chapters {
            eprintln!("[{}] {:?} chars={} head={:?}", c.skip.as_deref().unwrap_or("-"), c.title, c.text.chars().count(), c.text.chars().take(80).collect::<String>());
        }
        assert!(!b.chapters.is_empty());
    }

    /// A real book cut into blocks as the window imports it, every block without a word to say printed: `BOOK` — the
    /// book's file.
    #[test]
    #[ignore]
    fn a_real_book_s_blocks_without_words() {
        let path = std::env::var("BOOK").expect("BOOK");
        let b = super::parse(&path, &std::fs::read(&path).unwrap()).unwrap();
        let (mut total, mut wordless) = (0, 0);
        for c in &b.chapters {
            for (speaker, text) in crate::speech::project::split_text_lines(&c.text, "n", None) {
                total += 1;
                if crate::audio::qc::normalize(&text).is_empty() {
                    wordless += 1;
                    eprintln!("[{}] {speaker}: {text:?}", c.title);
                }
            }
        }
        eprintln!("blocks {total}, without a word {wordless}");
    }
}
