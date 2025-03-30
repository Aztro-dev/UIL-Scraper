use crate::request::Subject;

#[allow(dead_code)]
pub struct Result {
    pub school: String,
    pub conference: u8,
    pub individual_scores: Vec<Individual>,
    pub team_score: Team,
}

impl Result {
    fn new(school: String, conference: u8, subject: Subject) -> Self {
        Self {
            school,
            conference,
            individual_scores: Vec::new(),
            team_score: match subject {
                Subject::ComputerScience => Team::ComputerScience {
                    score: 0,
                    prog: None,
                },
                _ => Team::Normal { score: 0 },
            },
        }
    }

    fn add_individual(&mut self, score: Individual) {
        self.individual_scores.push(score);
    }

    fn set_team_score(&mut self, score: Team) {
        self.team_score = score;
    }
}

#[allow(dead_code)]
pub enum Individual {
    Normal {
        name: String,
        score: i16,
    },
    Science {
        name: String,
        score: i16,
        biology: i16,
        chemistry: i16,
        physics: i16,
    },
}

pub enum Team {
    Normal { score: i16 },
    ComputerScience { score: i16, prog: Option<i16> },
}
