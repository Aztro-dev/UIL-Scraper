use crate::{
    Individual,
    individual::IndividualMisc,
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
        // Subject::ComputerApplications,
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
            for i in 0..std::cmp::min(indiv_ties.len(), 6) {
                let group = indiv_ties[i].clone();
                if group.contains(&&copy) {
                    let mut sum = 0.0;
                    for ii in i..std::cmp::min(i + group.len(), 6) {
                        sum += INDIV_POINTS[ii];
                    }
                    indiv.points = sum / group.len() as f32;
                }
            }
            let mut found = false;
            for result in &mut individual_results.iter_mut() {
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
            for i in 0..std::cmp::min(team_ties.len(), positions) {
                let group = team_ties[i].clone();
                if group.contains(&&copy) {
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
            for result in &mut team_results {
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
    Some((individual_results, team_results))
}
