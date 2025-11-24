use crate::types::{Board, Member};
use anyhow::Result;
use scraper::{ElementRef, Html, Selector};
use winnow::Result as PResult;
use winnow::ascii::{line_ending, multispace0, space0};
use winnow::combinator::{opt, repeat, terminated};
use winnow::prelude::*;
use winnow::token::take_while;

pub fn parse_html_boards(html_content: &str) -> Result<Vec<Board>> {
    let document = Html::parse_document(html_content);
    let mut boards = Vec::new();

    let section_selector = Selector::parse("section").unwrap();
    let heading_selector = Selector::parse("h1, h2, h3").unwrap();
    let tab_wrapper_selector = Selector::parse(".elementor-tabs-wrapper").unwrap();
    let tab_content_wrapper_selector = Selector::parse(".elementor-tabs-content-wrapper").unwrap();

    let year_regex = regex::Regex::new(r"20\d{2}/20\d{2}").unwrap();

    let mut current_year = "Okänt år".to_string();

    for section in document.select(&section_selector) {
        for heading in section.select(&heading_selector) {
            let text = heading.text().collect::<Vec<_>>().concat();
            if let Some(mat) = year_regex.find(&text) {
                current_year = mat.as_str().to_string();
            }
        }

        if let Some(tab_wrapper) = section.select(&tab_wrapper_selector).next() {
            let titles: Vec<String> = tab_wrapper
                .select(&Selector::parse(".elementor-tab-desktop-title").unwrap())
                .map(|el| el.text().collect::<Vec<_>>().concat().trim().to_string())
                .collect();

            if let Some(content_wrapper) = section.select(&tab_content_wrapper_selector).next() {
                let contents: Vec<ElementRef> = content_wrapper
                    .select(&Selector::parse(".elementor-tab-content").unwrap())
                    .collect();

                for (i, title) in titles.iter().enumerate() {
                    if let Some(content_node) = contents.get(i) {
                        let raw_text = extract_text_preserving_lines(*content_node);

                        let members = parse_members_from_text(&raw_text);

                        if !members.is_empty() {
                            boards.push(Board {
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

    Ok(boards)
}

fn extract_text_preserving_lines(element: ElementRef) -> String {
    let mut text = String::new();

    for p in element.select(&Selector::parse("p").unwrap()) {
        for node in p.children() {
            if let Some(element) = node.value().as_element() {
                if element.name() == "br" {
                    text.push('\n');
                }
            } else if let Some(t) = node.value().as_text() {
                text.push_str(t);
            }
        }
        text.push('\n');
    }
    text
}

fn parse_members_from_text(input: &str) -> Vec<Member> {
    let result = parse_member_list.parse(input);
    match result {
        Ok(members) => members,
        Err(_) => Vec::new(),
    }
}

fn parse_member_list(input: &mut &str) -> PResult<Vec<Member>> {
    repeat(0.., terminated(parse_member, multispace0)).parse_next(input)
}

fn parse_member(input: &mut &str) -> PResult<Member> {
    space0.parse_next(input)?;

    let position = take_while(1.., |c| c != ':' && c != '\n' && c != '\r').parse_next(input)?;

    ":".parse_next(input)?;
    space0.parse_next(input)?;

    let name = take_while(1.., |c| c != '\n' && c != '\r').parse_next(input)?;

    let _ = opt(line_ending).parse_next(input)?;

    Ok(Member {
        position: position.trim().to_string(),
        name: name.trim().to_string(),
    })
}
