use minreq::Response;
use scraper::{Html, Selector};

use crate::{individual::Individual, team::Team};

#[derive(Clone)]
pub struct RequestFields {
    pub district: Option<u8>,
    pub region: Option<u8>,
    pub state: Option<u8>,
    pub subject: Subject,
    pub conference: u8,
    pub year: u16,
}

impl RequestFields {
    pub fn parse_conference(string: String) -> Option<u8> {
        match string.as_str() {
            "1A" | "1a" | "1" => Some(1),
            "2A" | "2a" | "2" => Some(2),
            "3A" | "3a" | "3" => Some(3),
            "4A" | "4a" | "4" => Some(4),
            "5A" | "5a" | "5" => Some(5),
            "6A" | "6a" | "6" => Some(6),
            _ => None,
        }
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
        if self.state.is_none() {
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
    let subject = fields.subject.to_u8();
    let conference = fields.conference;
    let year = fields.year - 2008;
    let url: String = format!(
        "https://postings.speechwire.com/r-uil-academics.php?groupingid={subject}&Submit=View+postings&region={region}&district={district}&state={state}&conference={conference}&seasonid={year}"
    );
    let response: Response = minreq::get(url).send().ok()?;

    if response.status_code >= 400 {
        return None;
    }

    Some(response.as_str().ok()?.to_string())
}

pub fn perform_scrape(fields: RequestFields) -> Option<(Vec<Individual>, Vec<Team>)> {
    let mut individual_results: Vec<Individual> = Vec::new();
    let mut team_results: Vec<Team> = Vec::new();

    let request = request(fields.clone())?;

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
}

#[allow(dead_code)]
#[derive(Clone)]
pub enum Subject {
    Accounting,
    /// No longer competed in
    ComputerApplications,
    CurrentEvents,
    SocialStudies,
    Spelling,
    Calculator,
    ComputerScience,
    Mathematics,
    NumberSense,
    Science,
}

impl Subject {
    fn to_u8(&self) -> u8 {
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
        }
    }

    pub fn from_str(string: &str) -> Option<Self> {
        match string {
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
            _ => None,
        }
    }
}
