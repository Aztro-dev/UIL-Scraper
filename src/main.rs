use scraper::*;

mod request;
use request::*;

mod results;
use results::*;

fn main() {
    let mut results: Vec<Result> = Vec::new();

    let conference = 4;
    for district in 1..=32 {
        let fields = UILFields {
            district: Some(district),
            region: None,
            state: None,
            conference,
            subject: Subject::Science,
            year: 2025,
        };

        let request = request(fields.clone());
        if request.is_none() {
            println!("District {district} unavailable");
            continue;
        }

        let request = request.unwrap();

        let document = Html::parse_document(request.as_str());
        let table_selector = Selector::parse("table.ddprint").ok();
        if table_selector.is_none() {
            println!("District {district} unavailable");
            continue;
        }
        let table_selector = table_selector.unwrap();
        let mut table = document.select(&table_selector);
        let individual_table = table.next();
        if individual_table.is_none() {
            println!("District {district} unavailable");
            continue;
        }
        let individual_table = individual_table.unwrap();
        let team_table = table.next();
        if team_table.is_none() {
            println!("District {district} unavailable");
            continue;
        }
        let team_table = team_table.unwrap();
        let row_selector = Selector::parse("tr").unwrap();
        let cell_selector = Selector::parse("td").unwrap();
        let span_selector = Selector::parse("span").unwrap();

        for row in individual_table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();

            let place = &cells[0];
            if place == "Place" {
                // We continue because this row doesn't contain any data
                continue;
            }
            let school = cells[1].clone();
            let name = &cells[2];
            let individual: Individual = match fields.clone().subject {
                Subject::Science => Individual::Science {
                    name: name.clone(),
                    school: school.clone(),
                    biology: cells[4].parse::<i16>().unwrap_or(0),
                    chemistry: cells[5].parse::<i16>().unwrap_or(0),
                    physics: cells[6].parse::<i16>().unwrap_or(0),
                    score: cells[7].parse::<i16>().unwrap_or(0),
                },
                _ => Individual::Normal {
                    name: name.clone(),
                    school: school.clone(),
                    score: cells[4].parse::<i16>().unwrap_or(0),
                },
            };
            let mut index = usize::MAX;
            for (i, result) in results.iter().enumerate() {
                if result.school == school {
                    index = i;
                    break;
                }
            }

            if index == usize::MAX {
                let mut result = Result::new(school, conference, fields.clone().subject);
                result.add_individual(individual);
                results.push(result);
            } else {
                let result = results.get_mut(index).unwrap();
                result.add_individual(individual);
            }
        }
        for row in team_table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();

            let place = &cells[0];
            if place == "Place" {
                // We continue because this row doesn't contain any data
                continue;
            }
            // Extract the direct text (e.g., "#name" or "#address")

            let mut span_text = String::new();
            // Separate span text from direct text
            for cell in row.select(&cell_selector) {
                // Extract the span text if it exists
                let text = cell
                    .select(&span_selector)
                    .next()
                    .map(|span| span.text().collect::<String>())
                    .unwrap_or_default();
                if text.is_empty() {
                    continue;
                }
                span_text = text;
            }
            let mut school = cells[1].clone();
            let _ = school.split_off(school.find(&span_text).unwrap());
            let mut index = usize::MAX;
            for (i, result) in results.iter().enumerate() {
                if result.school == school {
                    index = i;
                    break;
                }
            }

            let team: Team = match fields.clone().subject {
                Subject::ComputerScience => Team::ComputerScience {
                    score: cells[3].parse::<i16>().unwrap_or(0),
                    prog: cells[2].parse::<i16>().ok(),
                    name: school.clone(),
                },
                _ => Team::Normal {
                    score: cells[2].parse::<i16>().unwrap_or(0),
                    name: school.clone(),
                },
            };

            if index == usize::MAX {
                let mut result = Result::new(school, conference, fields.clone().subject);
                result.set_team_score(team);
                results.push(result);
            } else {
                let result = results.get_mut(index).unwrap();
                result.set_team_score(team);
            }
        }
        println!("District {district} completed");
    }
    let mut individuals: Vec<Individual> = Vec::new();
    let mut teams: Vec<Team> = Vec::new();
    let mut longest_individual_name = 0;
    let mut longest_team_name = 0;
    for result in results.iter_mut() {
        individuals.append(&mut result.individual_scores);
        teams.push(result.team_score.clone());
    }

    individuals.sort_by(|a, b| {
        let a_score = a.get_score();
        let b_score = b.get_score();
        b_score.cmp(&a_score)
    });
    individuals.resize(
        25,
        Individual::Normal {
            score: 0,
            school: String::new(),
            name: String::new(),
        },
    );
    for individual in individuals.iter() {
        if individual.get_name().len() > longest_individual_name {
            longest_individual_name = individual.get_name().len();
        }
    }
    teams.sort_by(|a, b| {
        let a_score = a.get_score();
        let b_score = b.get_score();
        b_score.cmp(&a_score)
    });
    teams.resize(
        10,
        Team::Normal {
            score: 0,
            name: String::new(),
        },
    );
    for team in teams.iter() {
        if team.get_name().len() > longest_team_name {
            longest_team_name = team.get_name().len();
        }
    }

    println!("{conference}A Individual Scores:");
    for (place, individual) in individuals.iter().enumerate() {
        println!(
            "{:2} {:longest_individual_name$} => {}",
            place + 1,
            individual.get_name(),
            individual.get_score()
        );
    }
    println!();
    println!("{conference}A Team Scores:");
    for (place, team) in teams.iter().enumerate() {
        println!(
            "{:2} {:longest_team_name$} => {}",
            place + 1,
            team.get_name(),
            team.get_score()
        );
    }
}
