use advance::{AdvanceTypeIndividual, AdvanceTypeTeam};
use chrono::Datelike;
use std::{
    cmp::Ordering,
    collections::HashMap,
    time::{Duration, Instant},
};

use colored::Colorize;

mod request;
use request::*;

mod advance;

mod individual;
use individual::*;

mod team;
use team::*;

mod cli;
use cli::*;

mod interactive;
mod progress;

mod scrape;
use scrape::scrape_subject;

mod overall;

use clap::Parser;

#[derive(Clone, Debug)]
pub(crate) struct OutputSection {
    pub title: String,
    pub lines: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct RunOutput {
    pub title: String,
    pub sections: Vec<OutputSection>,
    pub elapsed: Duration,
}

fn main() {
    if std::env::args_os().len() > 1 {
        let cli = Cli::parse();
        match execute_cli(cli, true) {
            Ok(output) => print_run_output(&output),
            Err(error) => {
                eprintln!("{}", error.red());
                std::process::exit(1);
            }
        }
    } else {
        interactive::run_tui();
    }
}

pub(crate) fn execute_cli(mut cli: Cli, show_progress: bool) -> Result<RunOutput, String> {
    let start = Instant::now();
    let subject = Subject::from_str(&cli.subject).ok_or_else(|| format!("Unsupported subject: {}", cli.subject))?;
    let year = cli
        .year
        .unwrap_or(chrono::Utc::now().year().try_into().unwrap_or(2004));
    let mute_progress = cli.mute || !show_progress;

    if cli.command.is_none() {
        resolve_level(&mut cli, show_progress);
    }

    let conferences = RequestFields::parse_range(cli.conference.clone().unwrap_or(String::from("16")))
        .ok_or_else(|| String::from("Invalid conference value"))?;

    if cli.command.is_none() && cli.highscores {
        let fields = RequestFields {
            district: cli.district,
            region: cli.region,
            state: cli.state,
            subject: subject.clone(),
            conference: 0,
            year,
        };
        let (individual_results, team_results) = overall::highscores_data(fields, conferences, mute_progress)
            .ok_or_else(|| String::from("Didn't return any results"))?;

        return Ok(RunOutput {
            title: format!("{} Highscores", subject.to_string()),
            sections: vec![
                OutputSection {
                    title: format!("{} Individual Results", subject.to_string()),
                    lines: format_highscore_individual_lines(individual_results, &cli),
                },
                OutputSection {
                    title: format!("{} Team Results", subject.to_string()),
                    lines: format_highscore_team_lines(team_results, &cli),
                },
            ],
            elapsed: start.elapsed(),
        });
    }

    let results = if cli.command.is_none() {
        let fields = RequestFields {
            district: cli.district,
            region: cli.region,
            state: cli.state,
            subject: subject.clone(),
            conference: 0,
            year,
        };
        match subject.clone() {
            Subject::Rankings => overall::rankings(fields, conferences.clone(), mute_progress),
            Subject::Sweepstakes => overall::sweepstakes(fields, conferences.clone(), mute_progress),
            _ => scrape_subject(fields, conferences.clone(), mute_progress),
        }
    } else if let Some(Commands::Compare {
        person_a: _,
        person_b: _,
        conferences,
        district,
        region,
        state,
    }) = cli.command.clone()
    {
        let conferences = RequestFields::parse_range(conferences)
            .ok_or_else(|| String::from("Conferences entered in the wrong order"))?;

        let district = if district { Some(0) } else { None };
        let region = if region { Some(0) } else { None };

        let fields = RequestFields {
            district,
            region,
            state,
            subject: subject.clone(),
            conference: 0,
            year,
        };

        let (individual_results, team_results) = match subject.clone() {
            Subject::Rankings => overall::rankings(fields, conferences.clone(), mute_progress),
            Subject::Sweepstakes => overall::sweepstakes(fields, conferences.clone(), mute_progress),
            _ => scrape_subject(fields.clone(), conferences.clone(), mute_progress),
        }
        .ok_or_else(|| String::from("No results found"))?;

        if individual_results.is_empty() || team_results.is_empty() {
            None
        } else {
            Some((individual_results, team_results))
        }
    } else {
        None
    };

    let Some((mut individual_results, mut team_results)) = results else {
        return Err(String::from("Didn't return any results"));
    };

    if let Some(Commands::Compare {
        person_a,
        person_b,
        conferences: _,
        district: _,
        region: _,
        state: _,
    }) = cli.command.clone()
    {
        individual_results.retain(|x| x.name == person_a || x.name == person_b);
        team_results.retain(|x| x.school == person_a || x.school == person_b);
    }

    if !team_results.is_empty() && !individual_results.is_empty() {
        let advancing_teams = Team::get_advancing(team_results.clone());
        for team in &mut team_results {
            if !advancing_teams.contains(team) {
                team.advance = None;
            }
        }

        let mut advancing_individuals = HashMap::new();
        for indiv in &mut individual_results {
            let advance = indiv.advance.clone();
            let Some(team) = team_results.iter().find(|team| team.school == indiv.school) else {
                continue;
            };

            let Some(team_advance) = team.advance.clone() else {
                continue;
            };

            if let Some(count) = advancing_individuals.get(&team.school) {
                if *count >= 4 {
                    continue;
                }
                advancing_individuals.insert(team.school.clone(), *count + 1);
            } else {
                advancing_individuals.insert(team.school.clone(), 1);
            }

            if advance.is_some() {
                continue;
            }

            if team_advance == AdvanceTypeTeam::Advance {
                indiv.advance = Some(AdvanceTypeIndividual::Team);
            } else {
                indiv.advance = Some(AdvanceTypeIndividual::Wild);
            }
        }
    }

    let sections = build_sections(subject.clone(), &cli, &mut individual_results, &mut team_results);
    if sections.is_empty() {
        return Err(String::from("No results found"));
    }

    Ok(RunOutput {
        title: format!("{} Results", subject.to_string()),
        sections,
        elapsed: start.elapsed(),
    })
}

fn build_sections(
    subject: Subject,
    cli: &Cli,
    individual_results: &mut Vec<Individual>,
    team_results: &mut Vec<Team>,
) -> Vec<OutputSection> {
    let mut sections = Vec::new();

    if !individual_results.is_empty() {
        if subject == Subject::Sweepstakes {
            *individual_results = individual_results
                .iter()
                .map(|individual| {
                    let mut copy = individual.clone();
                    copy.score = copy.points.round() as i16;
                    copy
                })
                .collect();
        }

        sections.push(OutputSection {
            title: String::from("Individual Total Scores"),
            lines: format_individual_lines(
                individual_results.clone(),
                cli.individual_positions.unwrap_or(0),
                &cli.find,
            ),
        });

        if subject == Subject::Science {
            let mut biology = individual_results.clone();
            biology.iter_mut().for_each(|x| x.score = x.get_biology().unwrap_or(0));
            sections.push(OutputSection {
                title: String::from("Individual Biology Scores"),
                lines: format_individual_lines(
                    biology,
                    cli.individual_positions.unwrap_or(0),
                    &cli.find,
                ),
            });

            let mut chemistry = individual_results.clone();
            chemistry.iter_mut().for_each(|x| x.score = x.get_chemistry().unwrap_or(0));
            sections.push(OutputSection {
                title: String::from("Individual Chemistry Scores"),
                lines: format_individual_lines(
                    chemistry,
                    cli.individual_positions.unwrap_or(0),
                    &cli.find,
                ),
            });

            let mut physics = individual_results.clone();
            physics.iter_mut().for_each(|x| x.score = x.get_physics().unwrap_or(0));
            sections.push(OutputSection {
                title: String::from("Individual Physics Scores"),
                lines: format_individual_lines(
                    physics,
                    cli.individual_positions.unwrap_or(0),
                    &cli.find,
                ),
            });
        }
    }

    if !team_results.is_empty() {
        if subject == Subject::Sweepstakes {
            *team_results = team_results
                .iter()
                .map(|team| {
                    let mut copy = team.clone();
                    for indiv in individual_results.iter() {
                        if indiv.school == copy.school {
                            copy.points += indiv.points;
                        }
                    }
                    copy.score = copy.points.round() as i16;
                    copy.misc = TeamMisc::Normal;
                    copy
                })
                .collect();
        }

        sections.push(OutputSection {
            title: String::from("Team Scores"),
            lines: format_team_lines(
                team_results.clone(),
                subject,
                cli.team_positions.unwrap_or(0),
                &cli.find,
            ),
        });
    }

    sections
}

fn print_run_output(output: &RunOutput) {
    println!("{}", output.title);
    println!();
    for (index, section) in output.sections.iter().enumerate() {
        println!("{}", section.title);
        for line in &section.lines {
            println!("{line}");
        }
        if index + 1 < output.sections.len() {
            println!();
        }
    }
    println!();
    println!("Time elapsed: {:?}", output.elapsed);
}

fn format_individual_lines(
    mut results: Vec<Individual>,
    positions: usize,
    find: &Option<String>,
) -> Vec<String> {
    results.sort_by(|a, b| {
        let score_cmp = b.score.cmp(&a.score);
        if score_cmp == Ordering::Equal {
            if a.conference == b.conference {
                a.school.cmp(&b.school)
            } else {
                a.conference.cmp(&b.conference)
            }
        } else {
            score_cmp
        }
    });
    results.dedup();

    let filtered = apply_individual_filter(results, positions, find);
    if filtered.is_empty() {
        return vec![String::from("No matching individual results.")];
    }

    let name_width = filtered.iter().map(|(_, item)| item.name.len()).max().unwrap_or(1);
    let score_width = filtered
        .iter()
        .map(|(_, item)| score_width(item.score))
        .max()
        .unwrap_or(1);

    filtered
        .into_iter()
        .map(|(place, item)| {
            let advance = match item.advance {
                Some(AdvanceTypeIndividual::Indiv) => " [Indv]",
                Some(AdvanceTypeIndividual::Team) => " [Team]",
                Some(AdvanceTypeIndividual::Wild) => " [Wild]",
                None => "",
            };
            let district_region = if let Some(district) = item.district {
                let region = district_as_region(Some(district)).unwrap_or(0);
                format!(" • D{district} / R{region}")
            } else if let Some(region) = item.region {
                format!(" • R{region}")
            } else {
                String::from(" • State")
            };
            format!(
                "{:>3}. {:<name_width$}  {:>score_width$}  [{}A] {}{}{}",
                place,
                item.name,
                item.score,
                item.conference,
                item.school,
                district_region,
                advance,
            )
        })
        .collect()
}

fn format_team_lines(
    mut results: Vec<Team>,
    subject: Subject,
    positions: usize,
    find: &Option<String>,
) -> Vec<String> {
    results.sort_by(|a, b| b.score.cmp(&a.score));
    results.dedup();

    let filtered = apply_team_filter(results, positions, find);
    if filtered.is_empty() {
        return vec![String::from("No matching team results.")];
    }

    let name_width = filtered.iter().map(|(_, item)| item.school.len()).max().unwrap_or(1);
    let score_width = filtered
        .iter()
        .map(|(_, item)| score_width(item.score))
        .max()
        .unwrap_or(1);

    filtered
        .into_iter()
        .map(|(place, item)| {
            let advance = match item.advance {
                Some(AdvanceTypeTeam::Advance) => " [Advanced]",
                Some(AdvanceTypeTeam::Alternate) => " [Wildcard]",
                None => "",
            };
            let district_region = if let Some(district) = item.district {
                let region = district_as_region(Some(district)).unwrap_or(0);
                format!(" • D{district} / R{region}")
            } else if let Some(region) = item.region {
                format!(" • R{region}")
            } else {
                String::from(" • State")
            };
            let prog = match (subject.clone(), item.get_prog()) {
                (Subject::ComputerScience, Some(prog)) => format!(" • prog {prog}"),
                (Subject::ComputerScience, None) => String::from(" • prog N/A"),
                _ => String::new(),
            };
            format!(
                "{:>3}. {:<name_width$}  {:>score_width$}  [{}A]{}{}{}",
                place,
                item.school,
                item.score,
                item.conference,
                district_region,
                prog,
                advance,
            )
        })
        .collect()
}

fn format_highscore_individual_lines(mut results: Vec<Individual>, cli: &Cli) -> Vec<String> {
    results.sort_by(|a, b| {
        let score_cmp = b.score.cmp(&a.score);
        if score_cmp == Ordering::Equal {
            let a_year = a.school.get(0..4).unwrap_or("");
            let b_year = b.school.get(0..4).unwrap_or("");
            a_year.cmp(b_year)
        } else {
            score_cmp
        }
    });

    if results.is_empty() {
        return vec![String::from("No matching individual highscores.")];
    }

    let limit = cli.individual_positions.unwrap_or(0);
    if limit != 0 {
        results.truncate(limit.max(1));
    }

    let name_width = results.iter().map(|item| item.name.len()).max().unwrap_or(1);
    let score_width = results.iter().map(|item| score_width(item.score)).max().unwrap_or(1);

    results
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{:>3}. {:<name_width$}  {:>score_width$}  [{}A] {}",
                index + 1,
                item.name,
                item.score,
                item.conference,
                item.school,
            )
        })
        .collect()
}

fn format_highscore_team_lines(mut results: Vec<Team>, cli: &Cli) -> Vec<String> {
    results.sort_by(|a, b| {
        let score_cmp = b.score.cmp(&a.score);
        if score_cmp == Ordering::Equal {
            let a_year = a.school.get(0..4).unwrap_or("");
            let b_year = b.school.get(0..4).unwrap_or("");
            a_year.cmp(b_year)
        } else {
            score_cmp
        }
    });

    if results.is_empty() {
        return vec![String::from("No matching team highscores.")];
    }

    let limit = cli.team_positions.unwrap_or(0);
    if limit != 0 {
        results.truncate(limit.max(1));
    }

    let name_width = results.iter().map(|item| item.school.len()).max().unwrap_or(1);
    let score_width = results.iter().map(|item| score_width(item.score)).max().unwrap_or(1);

    results
        .into_iter()
        .enumerate()
        .map(|(index, item)| {
            format!(
                "{:>3}. {:<name_width$}  {:>score_width$}  [{}A] {}",
                index + 1,
                item.school,
                item.score,
                item.conference,
                item.school,
            )
        })
        .collect()
}

fn apply_individual_filter(
    results: Vec<Individual>,
    positions: usize,
    find: &Option<String>,
) -> Vec<(usize, Individual)> {
    let mut filtered = Vec::new();
    let mut previous_score = results.first().map(|item| item.score).unwrap_or(0);
    let mut previous_place = 0usize;

    for (index, item) in results.into_iter().enumerate() {
        let place = if index == 0 || item.score == previous_score {
            previous_place
        } else {
            index
        };
        previous_score = item.score;
        previous_place = place;

        if positions != 0 && find.is_none() && place >= positions {
            break;
        }

        if let Some(find_name) = find {
            if !item.name.contains(find_name) && !item.school.contains(find_name) {
                continue;
            }
        }

        filtered.push((place + 1, item));
    }

    filtered
}

fn apply_team_filter(results: Vec<Team>, positions: usize, find: &Option<String>) -> Vec<(usize, Team)> {
    let mut filtered = Vec::new();
    let mut previous_score = results.first().map(|item| item.score).unwrap_or(0);
    let mut previous_place = 0usize;

    for (index, item) in results.into_iter().enumerate() {
        let place = if index == 0 || item.score == previous_score {
            previous_place
        } else {
            index
        };
        previous_score = item.score;
        previous_place = place;

        if positions != 0 && find.is_none() && place >= positions {
            break;
        }

        if let Some(find_name) = find {
            if !item.school.contains(find_name) {
                continue;
            }
        }

        filtered.push((place + 1, item));
    }

    filtered
}

fn score_width(score: i16) -> usize {
    if score < 0 {
        ((-score) as u16).checked_ilog10().unwrap_or(0) as usize + 2
    } else {
        (score as u16).checked_ilog10().unwrap_or(0) as usize + 1
    }
}

pub fn resolve_level(cli: &mut Cli, show_messages: bool) {
    let subject = Subject::from_str(&cli.subject).unwrap_or(Subject::Mathematics);
    let year = cli
        .year
        .unwrap_or(chrono::Utc::now().year().try_into().unwrap_or(2004));

    while cli.district.is_none() && cli.region.is_none() && !cli.state {
        if show_messages {
            println!(
                "{}",
                "You must specify the level using --district, --region, or --state".red()
            );
        }

        let request = request::request(RequestFields {
            district: None,
            region: None,
            state: true,
            subject: subject.clone(),
            conference: 1,
            year,
        });

        if request.is_some() {
            cli.state = true;
            if show_messages {
                println!("Defaulting to state");
            }
            break;
        }

        let request = request::request(RequestFields {
            district: None,
            region: Some(1),
            state: false,
            subject: subject.clone(),
            conference: 1,
            year,
        });

        if request.is_some() {
            cli.region = Some(0);
            if show_messages {
                println!("Defaulting to region");
            }
            break;
        }

        let request = request::request(RequestFields {
            district: Some(1),
            region: None,
            state: false,
            subject: subject.clone(),
            conference: 1,
            year,
        });

        if request.is_some() {
            cli.district = Some(0);
            if show_messages {
                println!("Defaulting to district");
            }
            break;
        }
    }
}

pub fn find_level(cli: &mut Cli) {
    resolve_level(cli, true);
}
