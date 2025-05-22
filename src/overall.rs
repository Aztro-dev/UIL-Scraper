use std::sync::{Arc, Mutex};

use chrono::Datelike;
use colored::{ColoredString, Colorize};

use crate::{
    Individual,
    cli::Cli,
    individual::IndividualMisc,
    overall,
    request::{RequestFields, Subject},
    scrape::scrape_subject,
    team::{Team, TeamMisc},
};

pub fn rankings(
    request_fields: RequestFields,
    conferences: Vec<u8>,
    mute: bool,
) -> Option<(Vec<Individual>, Vec<Team>)> {
    let supported_subjects = [
        Subject::Accounting,
        Subject::ComputerScience,
        Subject::Mathematics,
        Subject::NumberSense,
        Subject::Calculator,
        Subject::Science,
        Subject::SocialStudies,
        Subject::Spelling,
    ];
    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();
    for subject in supported_subjects {
        let mut fields = request_fields.clone();
        fields.subject = subject;
        let results = scrape_subject(fields.clone(), conferences.clone(), mute);
        if results.is_none() {
            continue;
        }
        let (mut indiv, mut team) = results.unwrap();

        indiv.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });
        team.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });

        let mut indiv_points = Vec::new();
        let mut team_points = Vec::new();

        for (place, individual) in indiv.iter().enumerate() {
            let position = place + 1;
            let points = 5000.0 / (position as f32 + 4.0);
            let mut individual_copy = individual.clone();
            individual_copy.score = points as i16;
            individual_copy.misc = IndividualMisc::Normal;
            indiv_points.push(individual_copy);
        }
        for (place, team) in team.iter().enumerate() {
            let position = place + 1;
            let points = 5000.0 / (position as f32 + 4.0);
            let mut team_copy = team.clone();
            team_copy.score = points as i16;
            team_copy.misc = TeamMisc::Normal;
            team_points.push(team_copy);
        }

        for indiv in indiv_points {
            let mut found = false;
            for result in &mut individual_results {
                if result.name == indiv.name {
                    found = true;
                    result.score += indiv.score;
                }
            }
            if !found {
                individual_results.push(indiv);
            }
        }

        for team in team_points {
            let mut found = false;
            for result in &mut team_results {
                if result.school == team.school {
                    found = true;
                    result.score += team.score;
                }
            }
            if !found {
                team_results.push(team);
            }
        }

        if fields.district.is_some() {
            use std::{thread, time};

            println!("Pausing to (hopefully) prevent rate limiting");
            let second = time::Duration::from_millis(500);

            thread::sleep(second);
        }
    }
    Some((individual_results, team_results))
}

pub fn sweepstakes(
    request_fields: RequestFields,
    conferences: Vec<u8>,
    mute: bool,
) -> Option<(Vec<Individual>, Vec<Team>)> {
    let supported_subjects = [
        Subject::Accounting,
        Subject::ComputerApplications,
        Subject::CurrentEvents,
        Subject::ComputerScience,
        Subject::Calculator,
        Subject::Spelling,
        Subject::Science,
        Subject::SocialStudies,
        Subject::Mathematics,
        Subject::NumberSense,
    ];
    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();
    for subject in supported_subjects {
        if subject == Subject::ComputerApplications && request_fields.year > 2024 {
            // Computer Apps is discontinued
            continue;
        }
        let mut fields = request_fields.clone();
        fields.subject = subject.clone();
        let results = scrape_subject(fields.clone(), conferences.clone(), mute);
        if results.is_none() {
            continue;
        }
        let (mut indiv, mut team) = results.unwrap();

        indiv.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });
        team.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            b_score.cmp(&a_score)
        });

        if indiv.is_empty() {
            continue;
        }

        let indiv_ties = Individual::get_ties(indiv.clone());
        let team_ties = Team::get_ties(team.clone());

        const INDIV_POINTS: [f32; 6] = [15.0, 12.0, 10.0, 8.0, 6.0, 4.0];
        const TEAM_POINTS: [f32; 2] = [10.0, 5.0];
        const TEAM_CS_POINTS: [f32; 3] = [20.0, 16.0, 12.0];

        for indiv in indiv.iter_mut() {
            let copy = indiv.clone();
            for (i, group) in indiv_ties
                .iter()
                .enumerate()
                .take(std::cmp::min(indiv_ties.len(), 6))
            {
                if group.contains(&copy) {
                    let mut sum = 0.0;
                    for points in INDIV_POINTS
                        .iter()
                        .take(std::cmp::min(i + group.len(), 6))
                        .skip(i)
                    {
                        sum += points;
                    }
                    indiv.points = sum / group.len() as f32;
                }
            }
            let mut found = false;
            for result in individual_results.iter_mut() {
                if result.name == indiv.name {
                    found = true;
                    result.points += indiv.points;
                }
            }
            if !found {
                individual_results.push(indiv.clone());
            }
        }

        for mut team in team {
            let copy = team.clone();
            let positions = if subject != Subject::ComputerScience {
                2
            } else {
                3
            };
            for (i, group) in team_ties
                .iter()
                .enumerate()
                .take(std::cmp::min(team_ties.len(), positions))
            {
                if group.contains(&copy) {
                    let mut sum = 0.0;
                    for ii in i..std::cmp::min(i + group.len(), positions) {
                        if positions == 2 {
                            sum += TEAM_POINTS[ii];
                        } else {
                            sum += TEAM_CS_POINTS[ii];
                        }
                    }
                    team.points = sum / group.len() as f32;
                }
            }
            let mut found = false;
            for result in team_results.iter_mut() {
                if result.school == team.school {
                    found = true;
                    result.points += team.points;
                }
            }
            if !found {
                team_results.push(team);
            }
        }

        if fields.district.is_some() {
            use std::{thread, time};

            println!("Pausing to (hopefully) prevent rate limiting");
            let second = time::Duration::from_millis(500);

            thread::sleep(second);
        }
    }
    for (index, indiv) in individual_results.iter().enumerate() {
        if index > 25 {
            break;
        }
        println!("{}: {} points", indiv.name.clone(), indiv.points);
    }
    Some((individual_results, team_results))
}

pub fn highscores(request_fields: RequestFields, conferences: Vec<u8>, cli: Cli) {
    let mute = cli.mute;
    let current_year: u16 = chrono::Utc::now().year() as u16;
    let subject = request_fields.subject;
    let individual_results = Arc::new(Mutex::new(Vec::new()));
    let team_results = Arc::new(Mutex::new(Vec::new()));

    let cs_year = if request_fields.region.is_some() {
        2005
    } else {
        2004
    };

    let range = match subject {
        // The UIL CS test changed scales between region 2004 and state 2004
        Subject::ComputerScience => cs_year..=current_year,
        Subject::ComputerApplications => 2004..=2024,
        _ => 2004..=current_year,
    };

    // Used to stop rate limiting more efficiently
    let mut count = 0;

    // Can't go in parallel because you get giga rate-limited
    for year in range {
        let fields = RequestFields {
            district: request_fields.district,
            region: request_fields.region,
            state: request_fields.state,
            conference: 0,
            subject: subject.clone(),
            year,
        };
        let conferences = if year > 2014 {
            conferences.clone()
        } else {
            let mut new_conf = conferences.clone();
            new_conf.pop_if(|x| *x == 6);
            new_conf
        };

        count += conferences.len()
            * if fields.district.is_some() {
                32
            } else if fields.region.is_some() {
                4
            } else {
                1
            }
            * match subject {
                Subject::Rankings => 8,
                _ => 1,
            };

        let results = match subject {
            Subject::Rankings => overall::rankings(fields.clone(), conferences.clone(), mute),
            _ => scrape_subject(fields.clone(), conferences.clone(), mute),
        };

        if results.is_some() {
            let (mut indiv, mut team) = results.unwrap();

            if !indiv.is_empty() {
                indiv.sort_by(|a, b| {
                    let a_score = a.score;
                    let b_score = b.score;
                    b_score.cmp(&a_score)
                });

                let year_str = year.to_string();

                indiv.iter_mut().for_each(|indiv| {
                    indiv.school = format!("{year_str} - {}", indiv.school);
                });

                individual_results.lock().unwrap().append(&mut indiv);
            }

            if !team.is_empty() {
                team.sort_by(|a, b| {
                    let a_score = a.score;
                    let b_score = b.score;
                    b_score.cmp(&a_score)
                });

                let year_str = year.to_string();

                team.iter_mut().for_each(|team| {
                    team.school = format!("{year_str} - {}", team.school);
                });

                team_results.lock().unwrap().append(&mut team);
            }
        }

        if count <= 20 {
            continue;
        }
        count = 0;
        if request_fields.region.is_some() {
            use std::{thread, time};

            println!("Pausing to (hopefully) prevent rate limiting");
            let second = time::Duration::from_millis(1000);

            thread::sleep(second);
        } else if request_fields.district.is_some() {
            use std::{thread, time};

            println!("Pausing to (hopefully) prevent rate limiting");
            let second = time::Duration::from_millis(2000);

            thread::sleep(second);
        } else {
            use std::{thread, time};

            println!("Pausing to (hopefully) prevent rate limiting");
            let second = time::Duration::from_millis(1000);

            thread::sleep(second);
        }
    }
    println!("{} Individual Results: ", subject.to_string());
    {
        let mut results = individual_results.lock().unwrap();

        results.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            if a_score != b_score {
                b_score.cmp(&a_score)
            } else {
                let a_year = &a.school[0..4];
                let b_year = &b.school[0..4];

                a_year.cmp(b_year)
            }
        });

        let top_score = results.first().unwrap().score;

        let mut longest_name_len = 0;
        let score_len = top_score.checked_ilog10().unwrap_or(0) as usize + 1;

        results.iter().for_each(|indiv| {
            longest_name_len = std::cmp::max(longest_name_len, indiv.name.len());
        });

        let mut results_copy = results.clone();

        let indiv_positions = cli.individual_positions.unwrap_or(10);
        if indiv_positions != 0 {
            results_copy.resize(std::cmp::max(indiv_positions, 1), Individual::default());
        }

        for indiv in results_copy.iter() {
            let conference_str: ColoredString = match indiv.conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };
            let base: ColoredString = format!(
                "{:longest_name_len$} => {:>score_len$} ({conference_str} {})",
                indiv.name, indiv.score, indiv.school,
            )
            .into();

            println!("{base}");
        }
        println!();

        // NOTE: TODO
        if subject == Subject::Science {
            results.iter_mut().for_each(|indiv| {
                indiv.score = indiv.get_biology().unwrap_or(-120);
            });
            results.sort_by(|a, b| a.score.cmp(&b.score));
            for indiv in results.iter() {
                let conference_str: ColoredString = match indiv.conference {
                    1 => "1A".white(),
                    2 => "2A".yellow(),
                    3 => "3A".bright_blue(),
                    4 => "4A".green(),
                    5 => "5A".red(),
                    6 => "6A".magenta(),
                    _ => "".into(),
                };
                let base: ColoredString = format!(
                    "{:longest_name_len$} => {:>score_len$} ({conference_str} {})",
                    indiv.name, indiv.score, indiv.school,
                )
                .into();

                println!("{base}");
            }
        }
    }

    println!("{} Team Results: ", subject.to_string());
    {
        let mut results = team_results.lock().unwrap();

        results.sort_by(|a, b| {
            let a_score = a.score;
            let b_score = b.score;
            if a_score != b_score {
                b_score.cmp(&a_score)
            } else {
                let a_year = &a.school[0..4];
                let b_year = &b.school[0..4];
                a_year.cmp(b_year)
            }
        });

        let top_score = results.first().unwrap().score;
        let team_positions = cli.team_positions.unwrap_or(10);
        if team_positions != 0 {
            results.resize(std::cmp::max(team_positions, 1), Team::default());
        }

        let mut longest_name_len = 0;
        let score_len = top_score.checked_ilog10().unwrap_or(0) as usize + 1;

        results.iter().for_each(|team| {
            longest_name_len = std::cmp::max(longest_name_len, team.school[7..].len());
        });

        for team in results.iter() {
            let conference_str: ColoredString = match team.conference {
                1 => "1A".white(),
                2 => "2A".yellow(),
                3 => "3A".bright_blue(),
                4 => "4A".green(),
                5 => "5A".red(),
                6 => "6A".magenta(),
                _ => "".into(),
            };
            let base: ColoredString = format!(
                "{:longest_name_len$} => {:>score_len$} ({conference_str} {})",
                &team.school[7..],
                team.score,
                &team.school[0..4],
            )
            .into();

            println!("{base}");
        }
    }
}
