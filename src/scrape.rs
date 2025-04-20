use crate::Individual;
use crate::request;
use crate::request::RequestFields;
use crate::team::Team;
use colored::Colorize;
use rayon::prelude::*;
use std::sync::{Arc, Mutex};
use supports_color::Stream;

pub fn scrape_subject(
    request_fields: RequestFields,
    mut conferences: Vec<u8>,
    mute: bool,
) -> Option<(Vec<Individual>, Vec<Team>)> {
    let district = request_fields.district;
    let region = request_fields.region;
    let state = request_fields.state;
    let subject = request_fields.subject;
    let year = request_fields.year;

    let individual_results = Arc::new(Mutex::new(Vec::new()));
    let team_results = Arc::new(Mutex::new(Vec::new()));

    let mut fields = RequestFields {
        district,
        region,
        state,
        subject: subject.clone(),
        conference: 0,
        year,
    };

    conferences.dedup();

    for conference in conferences {
        fields.conference = conference;
        if district.is_none() && region.is_some() && region.unwrap() == 0 {
            (1..=4).into_par_iter().for_each(|region| {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: None,
                    region: Some(region),
                    state: false,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields, mute) {
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
            continue;
        }
        if district.is_some() && district.unwrap() == 0 {
            let range = match region {
                Some(0) => 1..=32,
                Some(region) => (region * 8 - 7)..=(region * 8),
                None => 1..=32,
            };
            range.into_par_iter().for_each(|district| {
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: Some(district),
                    region: None,
                    state: false,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields, mute) {
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
            continue;
        }
        if let Some((mut individual, mut team)) = scrape(fields.clone(), mute) {
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
    }

    let individual_results: Vec<Individual> = individual_results.lock().ok()?.to_vec();
    let team_results: Vec<Team> = team_results.lock().ok()?.to_vec();

    Some((individual_results, team_results))
}

pub fn scrape(fields: RequestFields, mute: bool) -> Option<(Vec<Individual>, Vec<Team>)> {
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
    let subject = fields.subject.to_string();
    let support = supports_color::on(Stream::Stdout);
    let mut unavailable = format!("{conference}A {subject} {level} unavailable").red();
    let mut completed = format!("{conference}A {subject} {level} completed").green();
    match support {
        Some(support) => {
            if !support.has_basic {
                unavailable.fgcolor = None;
                unavailable.bgcolor = None;
                completed.fgcolor = None;
                completed.bgcolor = None;
            }
        }
        _ => {
            unavailable.fgcolor = None;
            unavailable.bgcolor = None;
            completed.fgcolor = None;
            completed.bgcolor = None;
        }
    };

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
