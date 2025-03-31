use colored::{Color, ColoredString};
use scraper::{selectable::Selectable, *};
use std::cmp;

use crate::request::{RequestFields, Subject};

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum Individual {
    Normal {
        name: String,
        school: String,
        score: i16,
    },
    Science {
        name: String,
        school: String,
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
                score,
            } => score,
            Individual::Science {
                name: _,
                school: _,
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
                score: _,
            } => name.clone(),
            Individual::Science {
                name,
                school: _,
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
                score: _,
            } => school.clone(),
            Individual::Science {
                name: _,
                school,
                score: _,
                biology: _,
                chemistry: _,
                physics: _,
            } => school.clone(),
        }
    }

    pub fn to_biology(&mut self) {
        match self {
            Individual::Science {
                name,
                school,
                score: _,
                biology,
                chemistry: _,
                physics: _,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
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
                biology: _,
                chemistry,
                physics: _,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
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
                biology: _,
                chemistry: _,
                physics,
            } => {
                *self = Self::Science {
                    name: name.clone(),
                    school: school.clone(),
                    score: *physics,
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
                    biology: cells[4].parse::<i16>().unwrap_or(0),
                    chemistry: cells[5].parse::<i16>().unwrap_or(0),
                    physics: cells[6].parse::<i16>().unwrap_or(0),
                    score: cells[7].parse::<i16>().unwrap_or(0),
                },
                _ => Individual::Normal {
                    name: name.clone(),
                    school: school.clone(),
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
            .get(0)
            .unwrap()
            .get_score()
            .checked_ilog10()
            .unwrap_or(0) as usize
            + 1;

        for (place, individual) in results.iter().enumerate() {
            let name = individual.get_name();
            let score = individual.get_score();

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

            println!("{base} ({school})");
        }
    }
}
