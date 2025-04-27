use chrono::Datelike;

use colored::Colorize;

mod request;
use request::*;

mod individual;
use individual::*;

mod team;
use team::*;

mod cli;
use cli::*;

mod scrape;
use scrape::scrape_subject;

mod overall;

use clap::Parser;

fn main() {
    let mut cli = Cli::parse();

    let subject = Subject::from_str(&cli.subject).unwrap();
    let year = cli
        .year
        .unwrap_or(chrono::Utc::now().year().try_into().unwrap());

    if cli.command.is_none() {
        find_level(&mut cli);
    }

    let conferences =
        RequestFields::parse_range(cli.conference.unwrap_or(String::from("16"))).unwrap();

    let results = if cli.command.is_none() {
        let fields = RequestFields {
            district: cli.district,
            region: cli.region,
            state: cli.state,
            subject: subject.clone(),
            conference: 0,
            year,
        };
        match subject {
            Subject::Rankings => overall::rankings(fields, conferences.clone(), cli.mute),
            Subject::Sweepstakes => overall::sweepstakes(fields, conferences.clone(), cli.mute),
            _ => scrape_subject(fields, conferences.clone(), cli.mute),
        }
    } else {
        let Commands::Compare {
            person_a: _,
            person_b: _,
            conferences,
            district,
            region,
            state,
        } = cli.command.clone().unwrap();
        let conferences = RequestFields::parse_range(conferences)
            .expect("Conferences entered in the wrong order");

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

        let (individual_results, team_results) = match subject {
            Subject::Rankings => overall::rankings(fields, conferences.clone(), cli.mute),
            Subject::Sweepstakes => overall::sweepstakes(fields, conferences.clone(), cli.mute),
            _ => scrape_subject(fields.clone(), conferences.clone(), cli.mute),
        }
        .expect("No results found");

        if individual_results.is_empty() || team_results.is_empty() {
            None
        } else {
            Some((individual_results, team_results))
        }
    };
    if results.is_none() {
        println!("{}", "Didn't return any results".red());
        return;
    }

    let (mut individual_results, mut team_results) = results.unwrap();

    if cli.command.is_some() {
        let Commands::Compare {
            person_a,
            person_b,
            conferences: _,
            district: _,
            region: _,
            state: _,
        } = cli.command.clone().unwrap();
        {
            individual_results.retain(|x| x.name == person_a || x.name == person_b);
            team_results.retain(|x| x.school == person_a || x.school == person_b);
        }
    }

    if !individual_results.is_empty() {
        if subject == Subject::Sweepstakes {
            individual_results = individual_results
                .iter()
                .map(|individual| {
                    let mut copy = individual.clone();
                    copy.score = copy.points as i16;
                    copy
                })
                .collect();
        }
        println!("Individual Total Scores:");
        Individual::display_results(
            individual_results.clone(),
            cli.individual_positions.unwrap_or(25),
        );
        println!();
        if let Subject::Science = subject {
            let mut biology = individual_results.clone();
            biology.retain_mut(|x| {
                x.score = x.get_biology().unwrap();
                true
            });
            let mut chemistry = individual_results.clone();
            chemistry.retain_mut(|x| {
                x.score = x.get_chemistry().unwrap();
                true
            });
            let mut physics = individual_results.clone();
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
            println!();
        }
    }
    if !team_results.is_empty() {
        if subject == Subject::Sweepstakes {
            team_results = team_results
                .iter()
                .map(|team| {
                    let mut copy = team.clone();
                    for indiv in &individual_results {
                        if indiv.school == copy.school {
                            copy.points += indiv.points;
                        }
                    }
                    copy.score = copy.points as i16;
                    copy.misc = TeamMisc::Normal;
                    copy
                })
                .collect();
        }
        println!("Team Scores:");
        Team::display_results(
            team_results.to_vec(),
            subject,
            cli.team_positions.unwrap_or(25),
        );
    }
}

pub fn find_level(cli: &mut Cli) {
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
}
