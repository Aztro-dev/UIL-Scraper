use colored::{Color, ColoredString, Colorize};
use scraper::{selectable::Selectable, *};
use std::cmp;

use crate::request::{RequestFields, Subject};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum Individual {
    Normal {
        name: String,
        school: String,
        conference: u8,
        score: i16,
    },
    Science {
        name: String,
        school: String,
        conference: u8,
        score: i16,
        biology: i16,
        chemistry: i16,
        physics: i16,
    },
}

impl Individual {
    pub fn get_score(&self) -> i16 {
        match *self {
            Individual::Normal {
                name: _,
                school: _,
                conference: _,
                score,
            } => score,
            Individual::Science {
                name: _,
                school: _,
                conference: _,
                score,
                biology: _,
                chemistry: _,
                physics: _,
            } => score,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            Individual::Normal {
                name,
                school: _,
                conference: _,
                score: _,
            } => name.clone(),
            Individual::Science {
                name,
                school: _,
                conference: _,
                score: _,
                biology: _,
                chemistry: _,
                physics: _,
            } => name.clone(),
        }
    }

    pub fn get_school(&self) -> String {
        match self {
            Individual::Normal {
                name: _,
                school,
                conference: _,
                score: _,
            } => school.clone(),
            Individual::Science {
                name: _,
                school,
                conference: _,
                score: _,
                biology: _,
                chemistry: _,
                physics: _,
            } => school.clone(),
        }
    }
    pub fn get_conference(&self) -> u8 {
        match self {
            Individual::Normal {
                name: _,
                school: _,
                conference,
                score: _,
            } => *conference,
            Individual::Science {
                name: _,
                school: _,
                conference,
                score: _,
                biology: _,
                chemistry: _,
                physics: _,
            } => *conference,
        }
    }

    pub fn to_biology(&mut self) {
        match self {
            Individual::Science {
                name,
                school,
                score: _,
                conference,
                biology,
                chemistry: _,
                physics: _,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
                    conference: *conference,
                    score: *biology,
                    biology: *biology,
                    chemistry: *biology,
                    physics: *biology,
                }
            }
            _ => {}
        }
    }

    pub fn to_chemistry(&mut self) {
        match self {
            Individual::Science {
                name,
                school,
                score: _,
                conference,
                biology: _,
                chemistry,
                physics: _,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
                    conference: *conference,
                    score: *chemistry,
                    biology: *chemistry,
                    chemistry: *chemistry,
                    physics: *chemistry,
                }
            }
            _ => {}
        }
    }

    pub fn to_physics(&mut self) {
        match self {
            Individual::Science {
                name,
                school,
                score: _,
                conference,
                biology: _,
                chemistry: _,
                physics,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
                    score: *physics,
                    conference: *conference,
                    biology: *physics,
                    chemistry: *physics,
                    physics: *physics,
                }
            }
            _ => {}
        }
    }

    pub fn parse_table(table: ElementRef, fields: &RequestFields) -> Option<Vec<Self>> {
        let mut results: Vec<Self> = Vec::new();

        let row_selector = Selector::parse("tr").ok()?;
        let cell_selector = Selector::parse("td").ok()?;

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
            let school = cells[1].clone();
            let name = &cells[2];
            let individual: Individual = match fields.clone().subject {
                Subject::Science => Individual::Science {
                    name: name.clone(),
                    school: school.clone(),
                    conference: fields.clone().conference,
                    biology: cells[4].parse::<i16>().unwrap_or(0),
                    chemistry: cells[5].parse::<i16>().unwrap_or(0),
                    physics: cells[6].parse::<i16>().unwrap_or(0),
                    score: cells[7].parse::<i16>().unwrap_or(0),
                },
                _ => Individual::Normal {
                    name: name.clone(),
                    school: school.clone(),
                    conference: fields.clone().conference,
                    score: cells[4].parse::<i16>().unwrap_or(0),
                },
            };
            results.push(individual);
        }
        Some(results)
    }

    pub fn display_results(mut results: Vec<Self>) {
        results.sort_by(|a, b| {
            let a_score = a.get_score();
            let b_score = b.get_score();
            b_score.cmp(&a_score)
        });

        results.resize(
            cmp::min(results.len(), 25),
            Individual::Normal {
                score: 0,
                conference: 1,
                school: String::new(),
                name: String::new(),
            },
        );

        let mut longest_individual_name = 0;

        for individual in results.iter() {
            if individual.get_name().len() > longest_individual_name {
                longest_individual_name = individual.get_name().len();
            }
        }

        let score_length = results
            .first()
            .unwrap()
            .get_score()
            .checked_ilog10()
            .unwrap_or(0) as usize
            + 1;

        let mut previous_score = results.first().unwrap().get_score();
        let mut previous_place = 0;
        for (place, individual) in results.iter().enumerate() {
            let name = individual.get_name();
            let score = individual.get_score();

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
                "{:2} {:longest_individual_name$} => {:>score_length$}",
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

            let school = individual.get_school();
            let conference = individual.get_conference();

            let conference_str: ColoredString = match conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };

            println!("{base} ({conference_str} - {school})");
        }
    }
}
