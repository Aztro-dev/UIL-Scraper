use chrono::Datelike;
use std::sync::{Arc, Mutex};

use colored::{ColoredString, Colorize};

mod request;
use rayon::prelude::*;
use request::*;

mod individual;
use individual::*;

mod team;
use team::*;

mod cli;
use cli::*;

use clap::Parser;

fn main() {
    let mut cli = Cli::parse();

    let individual_results = Arc::new(Mutex::new(Vec::new()));
    let team_results = Arc::new(Mutex::new(Vec::new()));

    let subject = Subject::from_str(&cli.subject).unwrap();
    let year = cli
        .year
        .unwrap_or(chrono::Utc::now().year().try_into().unwrap());

    while cli.district.is_none() && cli.region.is_none() && !cli.state {
        println!(
            "{}",
            "You must specify the level using --district, --region, or --state".red()
        );

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
            println!("Defaulting to state");
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
            println!("Defaulting to region");
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
            println!("Defaulting to district");
            break;
        }
    }

    let conferences =
        RequestFields::parse_conference(cli.conference.unwrap_or(String::from("16"))).unwrap();
    for conference in conferences {
        if cli.state {
            let fields = RequestFields {
                subject: subject.clone(),
                district: None,
                region: None,
                state: cli.state,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields, cli.mute) {
                // Lock and modify safely
                {
                    let mut ind_lock = individual_results.lock().unwrap();
                    ind_lock.append(&mut individual);
                }
                {
                    let mut team_lock = team_results.lock().unwrap();
                    team_lock.append(&mut team);
                }
            }
            continue;
        }
        if cli.region.is_some() && cli.region.unwrap() != 0 {
            let fields = RequestFields {
                subject: subject.clone(),
                district: None,
                region: cli.region,
                state: false,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields, cli.mute) {
                // Lock and modify safely
                {
                    let mut ind_lock = individual_results.lock().unwrap();
                    ind_lock.append(&mut individual);
                }
                {
                    let mut team_lock = team_results.lock().unwrap();
                    team_lock.append(&mut team);
                }
            }
            continue;
        } else if cli.region.is_some() {
            (1..=4).into_par_iter().for_each(|region| {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: None,
                    region: Some(region),
                    state: false,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields, cli.mute) {
                    // Lock and modify safely
                    {
                        let mut ind_lock = individual_results.lock().unwrap();
                        ind_lock.append(&mut individual);
                    }
                    {
                        let mut team_lock = team_results.lock().unwrap();
                        team_lock.append(&mut team);
                    }
                }
            });
        }
        if cli.district.is_some() && cli.district.unwrap() != 0 {
            let fields = RequestFields {
                subject: subject.clone(),
                district: cli.district,
                region: None,
                state: false,
                conference,
                year,
            };

            if let Some((mut individual, mut team)) = scrape(fields, cli.mute) {
                // Lock and modify safely
                {
                    let mut ind_lock = individual_results.lock().unwrap();
                    ind_lock.append(&mut individual);
                }
                {
                    let mut team_lock = team_results.lock().unwrap();
                    team_lock.append(&mut team);
                }
            }
            continue;
        } else if cli.district.is_some() {
            (1..=32).into_par_iter().for_each(|district| {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: Some(district),
                    region: None,
                    state: false,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields, cli.mute) {
                    // Lock and modify safely
                    {
                        let mut ind_lock = individual_results.lock().unwrap();
                        ind_lock.append(&mut individual);
                    }
                    {
                        let mut team_lock = team_results.lock().unwrap();
                        team_lock.append(&mut team);
                    }
                }
            });
        }
    }

    println!("Individual Total Scores:");
    Individual::display_results(
        individual_results.lock().unwrap().clone(),
        cli.individual_positions.unwrap_or(25),
    );
    println!();
    if let Subject::Science = subject {
        let mut biology = individual_results.lock().unwrap().clone();
        biology.retain_mut(|x| {
            x.score = x.get_biology().unwrap();
            true
        });
        let mut chemistry = individual_results.lock().unwrap().clone();
        chemistry.retain_mut(|x| {
            x.score = x.get_chemistry().unwrap();
            true
        });
        let mut physics = individual_results.lock().unwrap().clone();
        physics.retain_mut(|x| {
            x.score = x.get_physics().unwrap();
            true
        });
        println!("Individual Biology Scores:");
        Individual::display_results(biology, cli.individual_positions.unwrap_or(25));
        println!();
        println!("Individual Chemistry Scores:");
        Individual::display_results(chemistry, cli.individual_positions.unwrap_or(25));
        println!();
        println!("Individual Physics Scores:");
        Individual::display_results(physics, cli.individual_positions.unwrap_or(25));
    }
    println!("Team Scores:");
    Team::display_results(
        team_results.lock().unwrap().to_vec(),
        subject,
        cli.team_positions.unwrap_or(25),
    );
}

fn scrape(fields: RequestFields, mute: bool) -> Option<(Vec<Individual>, Vec<Team>)> {
    let conference = fields.conference;
    let level;
    if fields.state {
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
        if !mute {
            println!("{completed}");
        }
    } else if !mute {
        println!("{unavailable}");
    }

    Some((individual_results, team_results))
}
