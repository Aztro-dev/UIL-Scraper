use crate::Individual;
use crate::progress;
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

    let base_fields = RequestFields {
        district,
        region,
        state,
        subject: subject.clone(),
        conference: 0,
        year,
    };

    conferences.dedup();
    if progress::is_cancelled() {
        return Some((Vec::new(), Vec::new()));
    }
    progress::add_total_steps(expected_attempts(&base_fields, &conferences));

    if district.is_some() && district.unwrap_or(0) == 0 {
        for conference in conferences.clone() {
            let range = match region {
                Some(0) => 1..=32,
                Some(region) => (region * 8 - 7)..=(region * 8),
                None => 1..=32,
            };
            range.into_par_iter().for_each(|district| {
                if progress::is_cancelled() {
                    return;
                }
                let fields = RequestFields {
                    subject: subject.clone(),
                    district: Some(district),
                    region: None,
                    state: false,
                    conference,
                    year,
                };

                if let Some((mut individual, mut team)) = scrape(fields, mute) {
                    individual_results.lock().unwrap().append(&mut individual);
                    team_results.lock().unwrap().append(&mut team);
                }
            });
        }
    } else {
        conferences.into_par_iter().for_each(|conference| {
            if progress::is_cancelled() {
                return;
            }
            let fields = RequestFields {
                conference,
                ..base_fields.clone()
            };
            if district.is_some() || region.is_none() || region.is_some() && region.unwrap_or(0) != 0 {
                if let Some((mut individual, mut team)) = scrape(fields, mute) {
                    individual_results.lock().unwrap().append(&mut individual);
                    team_results.lock().unwrap().append(&mut team);
                }
            } else if region.is_some() && region.unwrap_or(0) == 0 {
                (1..=4).into_par_iter().for_each(|region| {
                    if progress::is_cancelled() {
                        return;
                    }
                    let fields = RequestFields {
                        subject: subject.clone(),
                        district: None,
                        region: Some(region),
                        state: false,
                        conference,
                        year,
                    };

                    if let Some((mut individual, mut team)) = scrape(fields, mute) {
                        individual_results.lock().unwrap().append(&mut individual);
                        team_results.lock().unwrap().append(&mut team);
                    }
                });
            }
        });
    }

    let individual_results: Vec<Individual> = individual_results.lock().ok()?.to_vec();
    let team_results: Vec<Team> = team_results.lock().ok()?.to_vec();

    Some((individual_results, team_results))
}

fn expected_attempts(fields: &RequestFields, conferences: &[u8]) -> usize {
    if fields.state {
        return conferences.len();
    }
    if let Some(district) = fields.district {
        if district == 0 {
            let district_count = match fields.region {
                Some(region) if (1..=4).contains(&region) => 8,
                _ => 32,
            };
            return conferences.len() * district_count;
        }
        return conferences.len();
    }
    if let Some(region) = fields.region {
        if region == 0 {
            return conferences.len() * 4;
        }
        return conferences.len();
    }
    conferences.len()
}

pub fn scrape(fields: RequestFields, mute: bool) -> Option<(Vec<Individual>, Vec<Team>)> {
    if progress::is_cancelled() {
        return None;
    }
    let conference = fields.conference;
    let level;
    let year = fields.year;
    if fields.state {
        level = String::from("States");
    } else if fields.region.is_some() {
        level = format!("Region {}", fields.region.unwrap_or(0));
    } else if fields.district.is_some() {
        level = format!("District {}", fields.district.unwrap_or(0));
    } else {
        return None;
    }
    let subject = fields.subject.to_string();
    let support = supports_color::on(Stream::Stdout);
    let mut unavailable = format!("{year} {conference}A {subject} {level} unavailable").red();
    let mut completed = format!("{year} {conference}A {subject} {level} completed").green();
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

    progress::set_current_label(format!("Fetching {year} {conference}A {subject} {level}"));

    if progress::is_cancelled() {
        return None;
    }

    if let Some((mut individual, mut team)) = request::perform_scrape(fields.clone()) {
        individual_results.append(&mut individual);
        team_results.append(&mut team);
        progress::record_attempt(&fields, individual_results.len(), team_results.len());
        if !mute {
            println!("{completed}");
        }
    } else {
        progress::record_attempt(&fields, 0, 0);
        if !mute {
            println!("{unavailable}");
        }
    }

    Some((individual_results, team_results))
}
