use minreq::Response;

#[derive(Clone)]
pub struct UILFields {
    pub district: Option<u8>,
    pub region: Option<u8>,
    pub state: Option<u8>,
    pub subject: Subject,
    pub conference: u8,
    pub year: u16,
}

impl UILFields {
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
            self.state.unwrap().to_string()
        }
    }
}

pub fn request(fields: UILFields) -> Option<String> {
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
            Subject::Accounting => 0,
            Subject::ComputerApplications => 1,
            Subject::CurrentEvents => 2,
            Subject::SocialStudies => 6,
            Subject::Spelling => 7,
            Subject::Calculator => 8,
            Subject::ComputerScience => 9,
            Subject::Mathematics => 10,
            Subject::NumberSense => 11,
            Subject::Science => 12,
        }
    }
}
