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
        let results = scrape_subject(fields, conferences.clone(), mute);
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
        use std::{thread, time};

        println!("Pausing to (hopefully) prevent rate limiting");
        let second = time::Duration::from_millis(1000);

        thread::sleep(second);
    }
    Some((individual_results, team_results))
}
