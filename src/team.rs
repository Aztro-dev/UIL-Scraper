use colored::{Color, ColoredString, Colorize};
use scraper::{ElementRef, Selector};
use std::{cmp, collections::HashMap};
use supports_color::Stream;

use crate::{
    advance::AdvanceTypeTeam,
    request::{RequestFields, Subject, district_as_region},
};

#[derive(Clone, PartialEq, PartialOrd, Debug)]
pub struct Team {
    pub school: String,
    pub score: i16,
    pub conference: u8,
    pub district: Option<u8>,
    pub region: Option<u8>,
    pub points: f32,
    pub advance: Option<AdvanceTypeTeam>,
    pub misc: TeamMisc,
}

#[derive(Clone, Eq, PartialEq, PartialOrd, Debug)]
pub enum TeamMisc {
    Normal,

    ComputerScience { prog: Option<i16> },
}

impl Default for Team {
    fn default() -> Self {
        Self {
            school: String::new(),
            district: None,
            region: None,
            conference: 0,
            score: 0,
            points: 0.0,
            advance: None,
            misc: TeamMisc::Normal,
        }
    }
}

impl Team {
    pub const fn get_prog(&self) -> Option<i16> {
        match self.misc {
            TeamMisc::Normal => None,
            TeamMisc::ComputerScience { prog } => prog,
        }
    }

    pub fn get_ties(sorted: Vec<Self>) -> Vec<Vec<Self>> {
        let mut groups: Vec<Vec<Self>> = Vec::new();
        let mut current_group: Vec<Self> = Vec::new();

        for (i, team) in sorted.iter().enumerate() {
            if i == 0 || team.score == sorted[i - 1].score {
                current_group.push(team.clone());
            } else {
                groups.push(current_group);
                current_group = vec![team.clone()];
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
        let span_selector = Selector::parse("span").unwrap();

        let mut place_index = 0;
        let mut points_index = 0;
        let mut advance_index = 0;

        let school_index = if fields.year > 2022 { 1 } else { 0 };
        let score_index = 2;

        for row in table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();
            for (index, column) in cells.iter().enumerate() {
                if column == "Place" {
                    place_index = index;
                }
                if column == "Points" {
                    points_index = index;
                }
                if column == "Advance?" {
                    advance_index = index;
                }
            }
            if cells[place_index] == "Place" {
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
            let mut school = cells[school_index].clone();

            if fields.year > 2022 {
                let _ = school.split_off(school.find(&span_text).unwrap());
            } else {
                let _ = school.split_off(school.find(", ").unwrap_or(school.len()));
                if school.split_off(school.find("H S").unwrap_or(school.len())) == "H S" {
                    school += "HS";
                }
            }

            let school = school.trim().to_string();

            let district = fields.district;
            let region = fields.region;
            let score = match fields.clone().subject {
                Subject::ComputerScience => &cells[if fields.year > 2022 { 3 } else { score_index }],
                _ => &cells[score_index],
            }.trim()
            .parse::<f32>()
            .unwrap_or(0.0)
            as i16;

            let points = cells[points_index].parse::<f32>().unwrap_or(0.0);

            let advance_str = if advance_index != 0 {
                &cells[advance_index]
            } else {
                &String::new()
            };
            let advance = match advance_str.as_str() {
                "Region" | "State" => Some(AdvanceTypeTeam::Advance),
                "Alternate" => Some(AdvanceTypeTeam::Alternate),
                _ => None,
            };

            let misc = match fields.clone().subject {
                Subject::ComputerScience => TeamMisc::ComputerScience {
                    prog: cells[if fields.year > 2022 { 2 } else { 0 }]
                        .parse::<i16>()
                        .ok(),
                },
                _ => TeamMisc::Normal {},
            };

            let team: Self = Self {
                score,
                school: school.clone(),
                conference: fields.clone().conference,
                district,
                region,
                points,
                advance,
                misc,
            };

            results.push(team);
        }
        Some(results)
    }

    pub fn display_results(
        mut results: Vec<Self>,
        subject: Subject,
        positions: usize,
        find: &Option<String>,
    ) {
        let support = supports_color::on(Stream::Stdout);

        results.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });

        results.dedup();

        let mut longest_team_name = 0;
        for team in results.iter() {
            if team.school.len() < longest_team_name {
                continue;
            }
            if find.is_none() {
                longest_team_name = team.school.len();
                continue;
            }
            let find_name = find.clone().unwrap_or_default();
            if !team.school.contains(&find_name) {
                continue;
            }
            longest_team_name = team.school.len();
        }

        let place_length = if find.is_none() {
            results.len().checked_ilog10().unwrap_or(0) as usize + 1
        } else {
            let mut previous_score = results.first().unwrap().score;
            let mut previous_place = 0;
            let mut longest_place = 0;
            for (place, team) in results.iter().enumerate() {
                let school = team.school.clone();
                let score = team.score;

                let place = if score == previous_score {
                    previous_place
                } else {
                    place
                };

                if score != previous_score {
                    previous_score = score;
                }
                previous_place = place;

                if school.contains(&find.clone().unwrap()) {
                    longest_place = place;
                }
            }
            longest_place.checked_ilog10().unwrap_or(0) as usize + 1
        };

        let first = results.first().unwrap();
        let score_length = first.score.checked_ilog10().unwrap_or(0) as usize + 1;
        let mut previous_score = first.score;
        let mut previous_place = 0;

        for (place, team) in results.iter().enumerate() {
            let school = team.school.clone();
            let score = team.score;

            let place = if score == previous_score {
                previous_place
            } else {
                place
            };

            if score != previous_score {
                previous_score = score;
            }
            previous_place = place;

            if positions != 0 && find.is_none() && place >= cmp::min(results.len(), positions) {
                break;
            }

            if let Some(find_name) = find.clone() {
                if !school.contains(&find_name) {
                    continue;
                }
            }

            let mut base: ColoredString = format!(
                "{:place_length$} {:longest_team_name$} => {:>score_length$}",
                place + 1,
                school,
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
                    first.get_prog().unwrap_or(0).checked_ilog10().unwrap_or(0) as usize + 1,
                    "N/A".len(),
                );
                base.input = format!("{} (prog {:>prog_length$})", base.input, prog);
            } else if matches!(subject, Subject::ComputerScience) {
                let prog_length = std::cmp::max(
                    first.get_prog().unwrap_or(1).checked_ilog10().unwrap_or(0) as usize + 1,
                    "N/A".len(),
                );
                base.input = format!("{} (prog {:prog_length$})", base.input, "N/A");
            }
            let conference = team.conference;

            let mut conference_str: ColoredString = match conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };

            let advance = team.advance.clone();

            let mut advance_status = "".green();
            if advance.is_some() {
                let advance = advance.unwrap();
                if advance == AdvanceTypeTeam::Advance {
                    advance_status = "(Advanced)".green();
                } else {
                    advance_status = "(Wildcard)".truecolor(0xFF, 0xA5, 0x00);
                }
            }
            match support {
                Some(support) => {
                    if !support.has_basic {
                        base.fgcolor = None;
                        base.bgcolor = None;
                        conference_str.fgcolor = None;
                        conference_str.bgcolor = None;
                        advance_status.fgcolor = None;
                        advance_status.bgcolor = None;
                    }
                }
                _ => {
                    base.fgcolor = None;
                    base.bgcolor = None;
                    conference_str.fgcolor = None;
                    conference_str.bgcolor = None;
                    advance_status.fgcolor = None;
                    advance_status.bgcolor = None;
                }
            };

            if let Some(district) = team.district {
                let region = district_as_region(Some(district)).unwrap_or(0);

                let mut region_str: ColoredString = match region {
                    1 => "Region 1".red(),
                    2 => "Region 2".yellow(),
                    3 => "Region 3".green(),
                    4 => "Region 4".blue(),
                    _ => "".into(),
                };

                match support {
                    Some(support) => {
                        if !support.has_basic {
                            region_str.fgcolor = None;
                            region_str.bgcolor = None;
                        }
                    }
                    _ => {
                        region_str.fgcolor = None;
                        region_str.bgcolor = None;
                    }
                }

                println!(
                    "{base} {conference_str} - District {district:<2} {region_str} {advance_status}"
                );
            } else if let Some(region) = team.region {
                println!("{base} {conference_str} - Region {region} {advance_status}");
            } else {
                println!("{base} {conference_str}");
            }
        }
    }

    pub fn get_advancing(mut results: Vec<Self>) -> Vec<Self> {
        results.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });

        results.dedup();

        // u8: district or region
        // u8: conference
        // i16: score
        let mut winning_teams: HashMap<(u8, u8), Self> = HashMap::new();
        for team in results.iter() {
            let location = team.district.unwrap_or(team.region.unwrap_or(0));
            winning_teams
                .entry((location, team.conference))
                .or_insert(team.clone());
        }

        // u8: region or district
        // u8: conference
        // i16: score
        let mut wildcarding_teams: HashMap<(u8, u8), Self> = HashMap::new();

        for team in results.iter() {
            if team.district.is_some() {
                let location = team.district.unwrap();
                if winning_teams
                    .get(&(location, team.conference))
                    .unwrap()
                    .score
                    > team.score
                {
                    let region_value = district_as_region(Some(location)).unwrap();
                    let result =
                        wildcarding_teams.insert((region_value, team.conference), team.clone());
                    if let Some(old_value) = result {
                        wildcarding_teams.insert((region_value, team.conference), old_value);
                    }
                }
            } else if team.region.is_some() {
                let location = team.region.unwrap();
                if winning_teams
                    .get(&(location, team.conference))
                    .unwrap()
                    .score
                    > team.score
                {
                    let result = wildcarding_teams.insert((1, team.conference), team.clone());
                    if let Some(old_value) = result {
                        wildcarding_teams.insert((1, team.conference), old_value);
                    }
                }
            }
        }

        let mut advancing_teams: Vec<Self> = Vec::new();

        for (_, team) in winning_teams {
            advancing_teams.push(team);
        }

        for (_, team) in wildcarding_teams {
            advancing_teams.push(team);
        }

        advancing_teams
    }
}
