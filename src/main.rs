use colored::{ColoredString, Colorize};

mod request;
use request::*;

mod individual;
use individual::*;

mod team;
use team::*;

mod cli;
use cli::*;

use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();

    let subject = Subject::from_str(&cli.subject).unwrap();
    let year = cli.year.unwrap_or(2025);
    let conferences =
        RequestFields::parse_conference(cli.conference.unwrap_or(String::from("16"))).unwrap();
    for conference in conferences {
        if cli.state.is_some() {
            let fields = RequestFields {
                subject: subject.clone(),
                district: None,
                region: None,
                state: cli.state,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields) {
                individual_results.append(&mut individual);
                team_results.append(&mut team);
            }
            continue;
        }
        if cli.region.is_some() && cli.region.unwrap() != 0 {
            let fields = RequestFields {
                subject: subject.clone(),
                district: None,
                region: cli.region,
                state: None,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields) {
                individual_results.append(&mut individual);
                team_results.append(&mut team);
            }
            continue;
        } else if cli.region.is_some() {
            for region in 1..=4 {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: None,
                    region: Some(region),
                    state: None,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields) {
                    individual_results.append(&mut individual);
                    team_results.append(&mut team);
                }
                continue;
            }
        }
        if cli.district.is_some() && cli.district.unwrap() != 0 {
            let fields = RequestFields {
                subject: subject.clone(),
                district: cli.district,
                region: None,
                state: None,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields) {
                individual_results.append(&mut individual);
                team_results.append(&mut team);
            }
            continue;
        } else if cli.district.is_some() {
            for district in 1..=32 {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: Some(district),
                    region: None,
                    state: None,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields) {
                    individual_results.append(&mut individual);
                    team_results.append(&mut team);
                }
                continue;
            }
        }
    }

    match subject {
        Subject::Science => {
            let mut biology = individual_results.clone();
            biology.retain_mut(|x| {
                x.to_biology();
                true
            });
            let mut chemistry = individual_results.clone();
            chemistry.retain_mut(|x| {
                x.to_chemistry();
                true
            });
            let mut physics = individual_results.clone();
            physics.retain_mut(|x| {
                x.to_physics();
                true
            });
            println!("Individual Total Scores:");
            Individual::display_results(individual_results);
            println!();
            println!("Individual Biology Scores:");
            Individual::display_results(biology);
            println!();
            println!("Individual Chemistry Scores:");
            Individual::display_results(chemistry);
            println!();
            println!("Individual Physics Scores:");
            Individual::display_results(physics);
        }
        _ => {
            println!("Individual Scores:");
            Individual::display_results(individual_results);
        }
    }
    println!();
    println!("Team Scores:");
    Team::display_results(team_results, subject);
}

fn scrape(fields: RequestFields) -> Option<(Vec<Individual>, Vec<Team>)> {
    let conference = fields.conference;
    let level;
    if fields.state.is_some() {
        level = String::from("States");
    } else if fields.region.is_some() {
        level = format!("Region {}", fields.region.unwrap());
    } else if fields.district.is_some() {
        level = format!("District {}", fields.district.unwrap());
    } else {
        return None;
    }
    let unavailable: ColoredString = format!("{conference}A {level} unavailable").red();
    let completed: ColoredString = format!("{conference}A {level} completed").green();

    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();

    if let Some((mut individual, mut team)) = request::perform_scrape(fields) {
        individual_results.append(&mut individual);
        team_results.append(&mut team);
        println!("{completed}");
    } else {
        println!("{unavailable}");
    }

    Some((individual_results, team_results))
}
