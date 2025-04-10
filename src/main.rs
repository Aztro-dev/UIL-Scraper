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
        scrape_subject(
            RequestFields {
                district: cli.district,
                region: cli.region,
                state: cli.state,
                subject: subject.clone(),
                conference: 0,
                year,
            },
            conferences.clone(),
            cli.mute,
        )
    } else {
        let Commands::Compare {
            person_a: _,
            person_b: _,
            conferences,
            districts,
            regions,
            state,
        } = cli.command.clone().unwrap();
        let conferences = RequestFields::parse_range(conferences)
            .expect("Conferences entered in the wrong order");
        let mut fields = RequestFields {
            district: cli.district,
            region: cli.region,
            state: cli.state,
            subject: subject.clone(),
            conference: 0,
            year,
        };

        let districts_parsed = RequestFields::parse_range(districts.unwrap_or("".to_string()));
        let regions_parsed = RequestFields::parse_range(regions.unwrap_or("".to_string()));

        if districts_parsed.is_some() {
            let mut districts = districts_parsed.clone().unwrap();
            districts.sort();
            fields.district = Some(districts[0]);
        }
        if regions_parsed.is_some() {
            let mut regions = regions_parsed.clone().unwrap();
            regions.sort();
            fields.region = Some(regions[0]);
        }
        if state.unwrap_or(false) {
            fields.state = state.unwrap();
        }

        let (mut individual_results, mut team_results) =
            scrape_subject(fields.clone(), conferences.clone(), cli.mute)
                .expect("No results found");

        if districts_parsed.is_some() {
            let mut districts = districts_parsed.unwrap();
            districts.sort();
            fields.district = Some(districts[1]);
        }
        if regions_parsed.is_some() {
            let mut regions = regions_parsed.unwrap();
            regions.sort();
            fields.district = Some(regions[1]);
        }
        if state.unwrap_or(false) {
            fields.state = state.unwrap();
        }

        let mut results = scrape_subject(fields, conferences, cli.mute).expect("No results found");

        individual_results.append(&mut results.0);
        team_results.append(&mut results.1);

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

    let (mut individual_results, team_results) = results.unwrap();

    if cli.command.is_some() {
        let Commands::Compare {
            person_a,
            person_b,
            conferences: _,
            districts: _,
            regions: _,
            state: _,
        } = cli.command.clone().unwrap();
        {
            individual_results.retain(|x| x.name == person_a || x.name == person_b);
            Individual::display_results(individual_results.clone(), 2);
            return;
        }
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
    }
    println!("Team Scores:");
    Team::display_results(
        team_results.to_vec(),
        subject,
        cli.team_positions.unwrap_or(25),
    );
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
