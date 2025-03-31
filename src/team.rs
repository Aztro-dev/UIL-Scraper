use colored::{Color, ColoredString};
use scraper::{ElementRef, Selector};
use std::cmp;

use crate::request::{RequestFields, Subject};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum Team {
    Normal {
        score: i16,
        name: String,
    },
    ComputerScience {
        score: i16,
        prog: Option<i16>,
        name: String,
    },
}

impl Team {
    pub fn get_score(&self) -> i16 {
        match *self {
            Team::Normal { score, name: _ } => score,
            Team::ComputerScience {
                score,
                prog: _,
                name: _,
            } => score,
        }
    }
    pub fn get_prog(&self) -> Option<i16> {
        match *self {
            Team::Normal { score: _, name: _ } => None,
            Team::ComputerScience {
                score: _,
                prog,
                name: _,
            } => prog,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            Team::Normal { score: _, name } => name.clone(),
            Team::ComputerScience {
                score: _,
                prog: _,
                name,
            } => name.clone(),
        }
    }
    pub fn parse_table(table: ElementRef, fields: &RequestFields) -> Option<Vec<Self>> {
        let mut results: Vec<Self> = Vec::new();

        let row_selector = Selector::parse("tr").ok()?;
        let cell_selector = Selector::parse("td").ok()?;
        let span_selector = Selector::parse("span").unwrap();

        for row in table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();
            let place = &cells[0];
            if place == "Place" {
                // We continue because this row doesn't contain any data
                continue;
            }
            // Extract the direct text (e.g., "#name" or "#address")

            let mut span_text = String::new();
            // Separate span text from direct text
            for cell in row.select(&cell_selector) {
                // Extract the span text if it exists
                let text = cell
                    .select(&span_selector)
                    .next()
                    .map(|span| span.text().collect::<String>())
                    .unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                span_text = text;
            }
            let mut school = cells[1].clone();
            let _ = school.split_off(school.find(&span_text).unwrap());
            let team: Team = match fields.clone().subject {
                Subject::ComputerScience => Team::ComputerScience {
                    score: cells[3].parse::<i16>().unwrap_or(0),
                    prog: cells[2].parse::<i16>().ok(),
                    name: school,
                },
                _ => Team::Normal {
                    score: cells[2].parse::<i16>().unwrap_or(0),
                    name: school,
                },
            };

            results.push(team);
        }
        Some(results)
    }

    pub fn display_results(mut results: Vec<Self>, subject: Subject) {
        results.sort_by(|a, b| {
            let a_score = a.get_score();
            let b_score = b.get_score();
            b_score.cmp(&a_score)
        });
        results.resize(
            cmp::min(results.len(), 25),
            Team::Normal {
                score: 0,
                name: String::new(),
            },
        );
        let mut longest_team_name = 0;
        for team in results.iter() {
            if team.get_name().len() > longest_team_name {
                longest_team_name = team.get_name().len();
            }
        }
        let score_length = results
            .get(0)
            .unwrap()
            .get_score()
            .checked_ilog10()
            .unwrap_or(0) as usize
            + 1;
        for (place, team) in results.iter().enumerate() {
            let name = team.get_name();
            let score = team.get_score();
            let mut base: ColoredString = format!(
                "{:2} {:longest_team_name$} => {:>score_length$}",
                place + 1,
                name,
                score
            )
            .into();
            match place + 1 {
                1 => {
                    base.fgcolor = Some(Color::Black);
                    base.bgcolor = Some(Color::Yellow);
                }
                2 => {
                    base.fgcolor = Some(Color::Black);
                    base.bgcolor = Some(Color::BrightWhite);
                }
                3 => {
                    base.fgcolor = Some(Color::Black);
                    base.bgcolor = Some(Color::BrightRed);
                }

                _ => base.fgcolor = None,
            };

            match subject {
                Subject::ComputerScience => {
                    if let Some(prog) = team.get_prog() {
                        let prog_length = results
                            .get(0)
                            .unwrap()
                            .get_prog()
                            .unwrap()
                            .checked_ilog10()
                            .unwrap_or(0) as usize
                            + 1;
                        base.input = format!("{} (prog {:>prog_length$})", base.input, prog);
                    }
                }
                _ => {}
            }
            println!("{base}");
        }
    }
}
