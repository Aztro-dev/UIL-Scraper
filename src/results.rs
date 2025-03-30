use crate::request::Subject;

#[allow(dead_code)]
#[derive(Clone)]
pub struct Result {
    pub school: String,
    pub conference: u8,
    pub individual_scores: Vec<Individual>,
    pub team_score: Team,
}

impl Result {
    pub fn new(school: String, conference: u8, subject: Subject) -> Self {
        Self {
            school: school.clone(),
            conference,
            individual_scores: Vec::new(),
            team_score: match subject {
                Subject::ComputerScience => Team::ComputerScience {
                    score: 0,
                    prog: None,
                    name: school,
                },
                _ => Team::Normal {
                    score: 0,
                    name: school,
                },
            },
        }
    }

    pub fn add_individual(&mut self, score: Individual) {
        self.individual_scores.push(score);
    }

    pub fn set_team_score(&mut self, score: Team) {
        self.team_score = score;
    }
}

#[derive(Clone, Eq, PartialEq, PartialOrd)]
#[allow(dead_code)]
pub enum Individual {
    Normal {
        name: String,
        school: String,
        score: i16,
    },
    Science {
        name: String,
        school: String,
        score: i16,
        biology: i16,
        chemistry: i16,
        physics: i16,
    },
}

impl Individual {
    pub fn get_score(&self) -> i16 {
        match *self {
            Individual::Normal {
                name: _,
                school: _,
                score,
            } => score,
            Individual::Science {
                name: _,
                school: _,
                score,
                biology: _,
                chemistry: _,
                physics: _,
            } => score,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            Individual::Normal {
                name,
                school: _,
                score: _,
            } => name.clone(),
            Individual::Science {
                name,
                school: _,
                score: _,
                biology: _,
                chemistry: _,
                physics: _,
            } => name.clone(),
        }
    }
}
#[allow(dead_code)]
#[derive(Clone, Eq, PartialEq, PartialOrd)]
pub enum Team {
    Normal {
        score: i16,
        name: String,
    },
    ComputerScience {
        score: i16,
        prog: Option<i16>,
        name: String,
    },
}

impl Team {
    pub fn get_score(&self) -> i16 {
        match *self {
            Team::Normal { score, name: _ } => score,
            Team::ComputerScience {
                score,
                prog: _,
                name: _,
            } => score,
        }
    }
    pub fn get_name(&self) -> String {
        match self {
            Team::Normal { score: _, name } => name.clone(),
            Team::ComputerScience {
                score: _,
                prog: _,
                name,
            } => name.clone(),
        }
    }
}
