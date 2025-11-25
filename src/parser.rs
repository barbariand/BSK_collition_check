use crate::types::{Board, Member};
use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use std::collections::HashMap;

pub fn parse_html_boards(html_content: &str) -> Result<Vec<Board>> {
    let document = Html::parse_document(html_content);
    let mut boards_map: HashMap<(String, String), Board> = HashMap::new();

    let section_selector = Selector::parse("section").unwrap();
    let heading_selector = Selector::parse("h1, h2, h3").unwrap();
    let widget_tabs_selector = Selector::parse(".elementor-widget-tabs").unwrap();
    let tab_wrapper_selector = Selector::parse(".elementor-tabs-wrapper").unwrap();
    let tab_content_wrapper_selector = Selector::parse(".elementor-tabs-content-wrapper").unwrap();

    let year_regex = regex::Regex::new(r"20\d{2}/20\d{2}").unwrap();

    println!("INFO: Startar parsing...");

    for section in document.select(&section_selector) {
        let mut current_year = "Okänt år".to_string();

        for heading in section.select(&heading_selector) {
            let text = heading.text().collect::<Vec<_>>().concat();
            if let Some(mat) = year_regex.find(&text) {
                current_year = mat.as_str().to_string();
            }
        }

        for widget in section.select(&widget_tabs_selector) {
            let tab_wrapper_opt = widget.select(&tab_wrapper_selector).next();
            let content_wrapper_opt = widget.select(&tab_content_wrapper_selector).next();

            if let (Some(tab_wrapper), Some(content_wrapper)) = (tab_wrapper_opt, content_wrapper_opt) {
                let titles: Vec<String> = tab_wrapper
                    .select(&Selector::parse(".elementor-tab-desktop-title").unwrap())
                    .map(|el| el.text().collect::<Vec<_>>().concat().trim().to_string())
                    .collect();

                let contents: Vec<ElementRef> = content_wrapper
                    .select(&Selector::parse(".elementor-tab-content").unwrap())
                    .collect();

                for (i, title) in titles.iter().enumerate() {
                    if title.is_empty() { continue; }

                    if let Some(content_node) = contents.get(i) {
                        let key = (title.clone(), current_year.clone());

                        if let Some(existing) = boards_map.get(&key) {
                            if !existing.members.is_empty() {
                                continue;
                            }
                        }

                        let raw_text = extract_text_recursive_wrapper(*content_node);
                        let members = parse_members_multiline(&raw_text);

                        if !members.is_empty() {
                            boards_map.insert(key, Board {
                                name: title.clone(),
                                year: current_year.clone(),
                                members,
                            });
                        }
                    }
                }
            }
        }
    }

    let result: Vec<Board> = boards_map.into_values().collect();
    println!("INFO: Totalt {} unika styrelser parsade.", result.len());
    Ok(result)
}

fn extract_text_recursive_wrapper(element: ElementRef) -> String {
    let mut text = String::new();
    extract_text_recursive(element, &mut text);
    text
}

fn extract_text_recursive(element: ElementRef, buffer: &mut String) {
    for node in element.children() {
        if let Some(el) = node.value().as_element() {
            if el.name() == "br" || el.name() == "p" || el.name() == "div" || el.name() == "li" {
                buffer.push('\n');
            }
            if let Some(child_ref) = ElementRef::wrap(node) {
                extract_text_recursive(child_ref, buffer);
            }
        } else if let Some(t) = node.value().as_text() {
            buffer.push_str(t);
        }
    }
}

// NY LOGIK: Hanterar namn som är uppdelade på flera rader
fn parse_members_multiline(input: &str) -> Vec<Member> {
    let mut members = Vec::new();
    let mut current_member: Option<Member> = None;

    for line in input.lines() {
        let cleaned = line.trim();
        if cleaned.is_empty() { continue; }

        // Kolla om raden innehåller ett kolon (ny position)
        if let Some((pos_part, name_part)) = cleaned.split_once(':') {
            // Om vi har en pågående medlem, spara den först
            if let Some(m) = current_member.take() {
                members.push(m);
            }

            // Starta ny medlem
            let pos = pos_part.trim().to_string();
            let name = name_part.trim().to_string(); // Kan vara tom om namnet kommer på nästa rad

            if !pos.is_empty() {
                current_member = Some(Member { position: pos, name });
            }
        } else {
            // Inget kolon. Detta är troligen en fortsättning på föregående namn.
            if let Some(ref mut m) = current_member {
                if !m.name.is_empty() {
                    m.name.push(' ');
                }
                m.name.push_str(cleaned);
            }
        }
    }

    // Glöm inte spara den sista medlemmen
    if let Some(m) = current_member {
        members.push(m);
    }

    members
}
