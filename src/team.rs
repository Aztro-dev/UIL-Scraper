use colored::{Color, ColoredString, Colorize};
use scraper::{ElementRef, Selector};
use std::cmp;

use crate::request::{RequestFields, Subject};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum Team {
    Normal {
        score: i16,
        conference: u8,
        name: String,
    },
    ComputerScience {
        score: i16,
        conference: u8,
        prog: Option<i16>,
        name: String,
    },
}

impl Team {
    pub fn get_score(&self) -> i16 {
        match *self {
            Team::Normal {
                score,
                name: _,
                conference: _,
            } => score,
            Team::ComputerScience {
                score,
                conference: _,
                prog: _,
                name: _,
            } => score,
        }
    }
    pub fn get_prog(&self) -> Option<i16> {
        match *self {
            Team::Normal {
                score: _,
                name: _,
                conference: _,
            } => None,
            Team::ComputerScience {
                score: _,
                prog,
                name: _,
                conference: _,
            } => prog,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            Team::Normal {
                score: _,
                name,
                conference: _,
            } => name.clone(),
            Team::ComputerScience {
                score: _,
                prog: _,
                conference: _,
                name,
            } => name.clone(),
        }
    }
    pub fn get_conference(&self) -> u8 {
        match self {
            Team::Normal {
                score: _,
                name: _,
                conference,
            } => *conference,
            Team::ComputerScience {
                score: _,
                prog: _,
                conference,
                name: _,
            } => *conference,
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
                    conference: fields.clone().conference,
                },
                _ => Team::Normal {
                    score: cells[2].parse::<i16>().unwrap_or(0),
                    conference: fields.clone().conference,
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
                conference: 0,
                name: String::new(),
            },
        );
        let mut longest_team_name = 0;
        for team in results.iter() {
            if team.get_name().len() > longest_team_name {
                longest_team_name = team.get_name().len();
            }
        }
        let first = results.first().unwrap();
        let score_length = first.get_score().checked_ilog10().unwrap_or(0) as usize + 1;
        let mut previous_score = results.first().unwrap().get_score();
        let mut previous_place = 0;
        for (place, team) in results.iter().enumerate() {
            let name = team.get_name();
            let score = team.get_score();

            let place = if score == previous_score {
                previous_place
            } else {
                place
            };

            if score != previous_score {
                previous_score = score;
            }

            previous_place = place;
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

            if let Some(prog) = team.get_prog() {
                let prog_length = std::cmp::max(
                    first.get_prog().unwrap().checked_ilog10().unwrap_or(0) as usize + 1,
                    "N/A".len(),
                );
                base.input = format!("{} (prog {:>prog_length$})", base.input, prog);
            } else if matches!(subject, Subject::ComputerScience { .. }) {
                let prog_length = std::cmp::max(
                    first.get_prog().unwrap().checked_ilog10().unwrap_or(0) as usize + 1,
                    "N/A".len(),
                );
                base.input = format!("{} (prog {:prog_length$})", base.input, "N/A");
            }
            let conference = team.get_conference();

            let conference_str: ColoredString = match conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };

            println!("{base} {conference_str}");
        }
    }
}
