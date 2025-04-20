use colored::{Color, ColoredString, Colorize};
use scraper::{selectable::Selectable, *};
use std::cmp;
use supports_color::Stream;

use crate::request::{RequestFields, Subject, district_as_region};

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Individual {
    pub name: String,
    pub school: String,
    pub conference: u8,
    pub district: Option<u8>,
    pub region: Option<u8>,
    pub score: i16,
    pub points: f32,
    pub misc: IndividualMisc,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum IndividualMisc {
    Normal,
    Science {
        biology: i16,
        chemistry: i16,
        physics: i16,
    },
}

impl Individual {
    pub fn get_biology(&self) -> Option<i16> {
        match self.misc {
            IndividualMisc::Science {
                biology,
                chemistry: _,
                physics: _,
            } => Some(biology),
            _ => None,
        }
    }

    pub fn get_chemistry(&self) -> Option<i16> {
        match self.misc {
            IndividualMisc::Science {
                biology: _,
                chemistry,
                physics: _,
            } => Some(chemistry),
            _ => None,
        }
    }
    pub fn get_physics(&self) -> Option<i16> {
        match self.misc {
            IndividualMisc::Science {
                biology: _,
                chemistry: _,
                physics,
            } => Some(physics),
            _ => None,
        }
    }

    pub fn get_ties(sorted: Vec<Self>) -> Vec<Vec<Self>> {
        let mut groups: Vec<Vec<Self>> = Vec::new();
        let mut current_group: Vec<Self> = Vec::new();

        for (i, individual) in sorted.iter().enumerate() {
            if i == 0 || individual.score == sorted[i - 1].score {
                current_group.push(individual.clone());
            } else {
                groups.push(current_group);
                current_group = vec![individual.clone()];
            }
        }

        if !current_group.is_empty() {
            groups.push(current_group);
        }

        groups
    }

    pub fn parse_table(table: ElementRef, fields: &RequestFields) -> Option<Vec<Self>> {
        let mut results: Vec<Self> = Vec::new();

        let row_selector = Selector::parse("tr").ok()?;
        let cell_selector = Selector::parse("td").ok()?;

        let mut points_index = 0;

        for row in table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();

            let place = &cells[0];
            if place == "Place" {
                for (index, column) in cells.iter().enumerate() {
                    if column == "Points" {
                        points_index = index;
                        break;
                    }
                }
                // We continue because this row doesn't contain any data
                continue;
            }
            let school = cells[1].clone();
            let name = &cells[2].trim().to_string();
            let individual_misc: IndividualMisc = match fields.clone().subject {
                Subject::Science => IndividualMisc::Science {
                    biology: cells[4].parse::<i16>().unwrap_or(0),
                    chemistry: cells[5].parse::<i16>().unwrap_or(0),
                    physics: cells[6].parse::<i16>().unwrap_or(0),
                },
                _ => IndividualMisc::Normal {},
            };
            let individual = Individual {
                name: name.clone(),
                school: school.clone(),
                conference: fields.clone().conference,
                district: fields.clone().district,
                region: fields.clone().region,
                score: cells[match fields.clone().subject {
                    Subject::Science => 7,
                    _ => 4,
                }]
                .parse::<i16>()
                .unwrap_or(0),
                points: cells[points_index].parse::<f32>().unwrap_or(0.0),
                misc: individual_misc,
            };
            results.push(individual);
        }
        Some(results)
    }

    pub fn display_results(mut results: Vec<Self>, positions: usize) {
        let support = supports_color::on(Stream::Stdout);

        results.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });

        results.dedup();

        if positions != 0 {
            results.resize(
                cmp::min(results.len(), positions),
                Individual {
                    score: 0,
                    conference: 1,
                    district: None,
                    region: None,
                    school: String::new(),
                    name: String::new(),
                    points: 0.0,
                    misc: IndividualMisc::Normal,
                },
            );
        }

        let mut longest_individual_name = 0;

        for individual in results.iter() {
            if individual.name.len() > longest_individual_name {
                longest_individual_name = individual.name.len();
            }
        }

        let score_length =
            results.first().unwrap().score.checked_ilog10().unwrap_or(0) as usize + 1;

        let mut previous_score = results.first().unwrap().score;
        let mut previous_place = 0;
        for (place, individual) in results.iter().enumerate() {
            let name = individual.name.clone();
            let score = individual.score;

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

            let school = individual.school.clone();
            let conference = individual.conference;

            let mut conference_str: ColoredString = match conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };

            let district = individual.district;
            if district.is_some() {
                let region = district_as_region(district).unwrap_or(0);

                let mut region_str: ColoredString = match region {
                    1 => "R1".red(),
                    2 => "R2".yellow(),
                    3 => "R3".green(),
                    4 => "R4".blue(),
                    _ => "".into(),
                };
                match support {
                    Some(support) => {
                        if !support.has_basic {
                            base.fgcolor = None;
                            base.bgcolor = None;
                            conference_str.fgcolor = None;
                            conference_str.bgcolor = None;
                            region_str.fgcolor = None;
                            region_str.bgcolor = None;
                        }
                    }
                    _ => {
                        base.fgcolor = None;
                        base.bgcolor = None;
                        conference_str.fgcolor = None;
                        conference_str.bgcolor = None;
                        region_str.fgcolor = None;
                        region_str.bgcolor = None;
                    }
                };

                let district = district.unwrap();
                println!("{base} ({conference_str} D{district:<2} {region_str} - {school})");
            } else if let Some(region) = individual.region {
                match support {
                    Some(support) => {
                        if !support.has_basic {
                            base.fgcolor = None;
                            base.bgcolor = None;
                            conference_str.fgcolor = None;
                            conference_str.bgcolor = None;
                        }
                    }
                    _ => {
                        base.fgcolor = None;
                        base.bgcolor = None;
                        conference_str.fgcolor = None;
                        conference_str.bgcolor = None;
                    }
                };
                println!("{base} ({conference_str} R{region} - {school})");
            } else {
                match support {
                    Some(support) => {
                        if !support.has_basic {
                            base.fgcolor = None;
                            base.bgcolor = None;
                            conference_str.fgcolor = None;
                            conference_str.bgcolor = None;
                        }
                    }
                    _ => {
                        base.fgcolor = None;
                        base.bgcolor = None;
                        conference_str.fgcolor = None;
                        conference_str.bgcolor = None;
                    }
                };
                println!("{base} ({conference_str} - {school})");
            }
        }
    }
}
