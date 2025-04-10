use colored::Colorize;
use minreq::Response;
use scraper::{Html, Selector};

use crate::{individual::Individual, team::Team};

#[derive(Clone)]
pub struct RequestFields {
    pub district: Option<u8>,
    pub region: Option<u8>,
    pub state: bool,
    pub subject: Subject,
    pub conference: u8,
    pub year: u16,
}

impl RequestFields {
    pub fn parse_range(mut string: String) -> Option<Vec<u8>> {
        if string.is_empty() {
            return None;
        }
        string = string.to_lowercase();
        if string.contains(",") {
            let mut split = string.split(",");
            let left_num = split.next().unwrap().parse::<u8>().ok()?;
            let right_num = split.next().unwrap().parse::<u8>().ok()?;
            let vec = vec![left_num, right_num];
            return Some(vec);
        }
        string.retain(|c| c.is_ascii_digit());
        let bytes = string.as_bytes();
        // char to u8
        let left_digit = bytes[0] - 48;
        if bytes.len() == 1 {
            if left_digit < 1 {
                return None;
            }
            if left_digit > 6 {
                return None;
            }

            let vec = vec![left_digit];
            return Some(vec);
        }
        let right_digit = bytes[1] - 48;
        let start = std::cmp::min(left_digit, right_digit);
        let end = std::cmp::max(left_digit, right_digit);

        if start < 1 {
            return None;
        }
        if end > 6 {
            return None;
        }

        let mut vec = Vec::new();
        for i in start..=end {
            vec.push(i);
        }
        Some(vec)
    }
    fn get_district(&self) -> String {
        if self.district.is_none() {
            String::new()
        } else {
            self.district.unwrap().to_string()
        }
    }
    fn get_region(&self) -> String {
        if self.region.is_none() {
            String::new()
        } else {
            self.region.unwrap().to_string()
        }
    }
    fn get_state(&self) -> String {
        if !self.state {
            String::new()
        } else {
            String::from("1")
        }
    }
}

pub fn request(fields: RequestFields) -> Option<String> {
    let district = fields.get_district();
    let region = fields.get_region();
    let state = fields.get_state();
    let subject: i8 = fields.subject.to_i8();
    let conference = fields.conference;
    let year = fields.year - 2008;
    let url: String = if fields.year > 2022 {
        format!(
            "https://postings.speechwire.com/r-uil-academics.php?groupingid={subject}&Submit=View+postings&region={region}&district={district}&state={state}&conference={conference}&seasonid={year}"
        )
    } else {
        old_school(fields)
    };
    let response: Response = minreq::get(url).with_timeout(1000).send().ok()?;

    if response.status_code >= 400 {
        return None;
    }
    // Results viewing for this season is not open.
    if response
        .as_str()
        .ok()?
        .contains("Please click a District to view results for.")
    {
        return None;
    }

    Some(response.as_str().ok()?.to_string())
}

pub fn perform_scrape(fields: RequestFields) -> Option<(Vec<Individual>, Vec<Team>)> {
    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();

    let request = request(fields.clone())?;

    if fields.year > 2022 {
        let document = Html::parse_document(request.as_str());
        let table_selector = Selector::parse("table.ddprint").ok()?;
        let mut table = document.select(&table_selector);
        let individual_table = table.next()?;

        let team_table = table.next()?;

        let mut individuals = Individual::parse_table(individual_table, &fields)?;

        individual_results.append(&mut individuals);

        let mut teams = Team::parse_table(team_table, &fields)?;

        team_results.append(&mut teams);

        Some((individual_results, team_results))
    } else {
        let document = Html::parse_document(request.as_str());
        let table_selector = Selector::parse("table").ok()?;
        let mut table = document.select(&table_selector);
        let individual_table = table.next()?;

        let team_table = table.next()?;

        let mut individuals = Individual::parse_table(individual_table, &fields)?;

        individual_results.append(&mut individuals);

        let mut teams = Team::parse_table(team_table, &fields)?;

        team_results.append(&mut teams);

        Some((individual_results, team_results))
    }
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Subject {
    Accounting,
    // NOTE: computer applications isn't fully supported
    ComputerApplications,
    // NOTE: current events isn't fully supported
    CurrentEvents,
    // NOTE: social studies isn't fully supported
    SocialStudies,
    Spelling,
    Calculator,
    ComputerScience,
    Mathematics,
    NumberSense,
    Science,
    // NOTE: sweepstakes isn't fully supported
    Sweepstakes,
}

impl Subject {
    fn to_i8(&self) -> i8 {
        match *self {
            Subject::Accounting => 1,
            Subject::ComputerApplications => 2,
            Subject::CurrentEvents => 3,
            Subject::SocialStudies => 6,
            Subject::Spelling => 7,
            Subject::Calculator => 8,
            Subject::ComputerScience => 9,
            Subject::Mathematics => 10,
            Subject::NumberSense => 11,
            Subject::Science => 12,
            Subject::Sweepstakes => -1,
        }
    }

    pub fn from_str(string: &str) -> Option<Self> {
        match string.to_lowercase().as_str() {
            "accounting" => Some(Self::Accounting),
            "comp_apps" => Some(Self::ComputerApplications),
            "current_events" => Some(Self::CurrentEvents),
            "comp_sci" | "cs" => Some(Self::ComputerScience),
            "calculator" | "calc" => Some(Self::Calculator),
            "spelling" | "spell" => Some(Self::Spelling),
            "social_studies" => Some(Self::SocialStudies),
            "mathematics" | "math" => Some(Self::Mathematics),
            "number_sense" | "ns" => Some(Self::NumberSense),
            "science" | "sci" => Some(Self::Science),
            "sweepstakes" | "overall" => Some(Self::Sweepstakes),
            _ => None,
        }
    }

    pub fn _list_options() {
        println!("Subjects listed in {} are not fully supported", "red".red());
        // let accounting
    }
}

pub fn district_as_region(district: Option<u8>) -> Option<u8> {
    district?;
    let region = match district.unwrap() {
        1..=8 => 1,
        9..=16 => 2,
        17..=24 => 3,
        25..=32 => 4,
        _ => 0,
    };

    if region == 0 {
        return None;
    }

    Some(region)
}

pub fn old_school(fields: RequestFields) -> String {
    let level = if fields.district.is_some() {
        "D"
    } else if fields.region.is_some() {
        "R"
    } else {
        "S"
    };

    let first = "https://utdirect.utexas.edu/nlogon/uil/vlcp_pub_arch.WBX?".to_string();
    let second = format!(
        "s_year={}&s_conference={}A&s_level_id={level}&s_level_nbr={}&",
        fields.year,
        fields.conference,
        if fields.district.is_some() {
            fields.district.unwrap().to_string()
        } else if fields.region.is_some() {
            fields.region.unwrap().to_string()
        } else {
            "".to_string()
        }
    )
    .to_string();

    let abbr = match fields.subject {
        Subject::Accounting => "ACC",
        Subject::ComputerScience => "CSC",
        _ => "",
    };

    let third = format!(
        "s_event_abbr={abbr}&s_submit_sw=X&s_year={}&s_conference={}A&s_level_id=S&s_level_nbr=&s_gender=&s_round=&s_dept=C&s_area_zone=",
        fields.year, fields.conference
    );

    first + &second + &third
}
