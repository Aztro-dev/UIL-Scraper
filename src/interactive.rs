use std::{
    io::{self, Stdout, Write},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use chrono::Datelike;
use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers,
        MouseButton, MouseEvent, MouseEventKind,
    },
    execute, queue,
    style::{
        Attribute, Color, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{
        self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode,
        enable_raw_mode,
    },
};

use crate::{
    cli::{Cli, Commands},
    execute_cli,
    progress::{self, ProgressSnapshot},
    request::RequestFields,
    OutputSection, RunOutput,
};

const SUBJECT_OPTIONS: [(&str, &str); 11] = [
    ("accounting", "Accounting"),
    ("comp_apps", "Computer Applications"),
    ("current_events", "Current Events"),
    ("social_studies", "Social Studies"),
    ("spelling", "Spelling"),
    ("calculator", "Calculator"),
    ("comp_sci", "Computer Science"),
    ("mathematics", "Mathematics"),
    ("number_sense", "Number Sense"),
    ("science", "Science"),
    ("sweepstakes", "Sweepstakes / Overall"),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Search,
    Compare,
    Rankings,
}

impl Mode {
    const ALL: [Mode; 3] = [Mode::Search, Mode::Compare, Mode::Rankings];

    const fn label(self) -> &'static str {
        match self {
            Self::Search => "Search subject results",
            Self::Compare => "Compare two people / schools",
            Self::Rankings => "Custom rankings",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let idx = Self::ALL.iter().position(|mode| *mode == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SearchLevel {
    Auto,
    District,
    Region,
    State,
}

impl SearchLevel {
    const ALL: [SearchLevel; 4] = [
        SearchLevel::Auto,
        SearchLevel::District,
        SearchLevel::Region,
        SearchLevel::State,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Auto => "Auto-detect from available postings",
            Self::District => "District",
            Self::Region => "Region",
            Self::State => "State",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|level| *level == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let idx = Self::ALL.iter().position(|level| *level == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompareLevel {
    District,
    Region,
    State,
}

impl CompareLevel {
    const ALL: [CompareLevel; 3] = [
        CompareLevel::District,
        CompareLevel::Region,
        CompareLevel::State,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::District => "District",
            Self::Region => "Region",
            Self::State => "State",
        }
    }

    fn next(self) -> Self {
        let idx = Self::ALL.iter().position(|level| *level == self).unwrap_or(0);
        Self::ALL[(idx + 1) % Self::ALL.len()]
    }

    fn previous(self) -> Self {
        let idx = Self::ALL.iter().position(|level| *level == self).unwrap_or(0);
        Self::ALL[(idx + Self::ALL.len() - 1) % Self::ALL.len()]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FieldId {
    Mode,
    Subject,
    SearchLevel,
    District,
    DistrictRegionFilter,
    Region,
    Conference,
    Year,
    Find,
    IndividualPositions,
    TeamPositions,
    Mute,
    Highscores,
    PersonA,
    PersonB,
    CompareConferences,
    CompareLevel,
    Run,
    Cancel,
}

#[derive(Clone, Debug)]
struct FieldView {
    id: FieldId,
    label: String,
    value: String,
    help: String,
}

#[derive(Clone, Debug)]
enum SidebarAction {
    SetMode(Mode),
    SetSubject(usize),
    SetSearchLevel(SearchLevel),
    SetCompareLevel(CompareLevel),
    SetText(FieldId, String),
    SetBool(FieldId, bool),
}

#[derive(Clone, Debug)]
enum SidebarItemKind {
    Heading,
    Info,
    Option(SidebarAction),
}

#[derive(Clone, Debug)]
struct SidebarItem {
    text: String,
    kind: SidebarItemKind,
    color: Color,
    bold: bool,
}

struct App {
    mode: Mode,
    subject_index: usize,
    search_level: SearchLevel,
    compare_level: CompareLevel,
    conference: String,
    district: String,
    district_region_filter: String,
    region: String,
    year: String,
    find: String,
    individual_positions: String,
    team_positions: String,
    mute: bool,
    highscores: bool,
    person_a: String,
    person_b: String,
    compare_conferences: String,
    focus: usize,
    form_scroll: usize,
    sidebar_scroll: usize,
    sidebar_x_scroll: usize,
    message: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            mode: Mode::Search,
            subject_index: SUBJECT_OPTIONS
                .iter()
                .position(|(value, _)| *value == "mathematics")
                .unwrap_or(0),
            search_level: SearchLevel::Auto,
            compare_level: CompareLevel::District,
            conference: String::from("1-6"),
            district: String::from("0"),
            district_region_filter: String::new(),
            region: String::from("0"),
            year: chrono::Utc::now().year().to_string(),
            find: String::new(),
            individual_positions: String::from("0"),
            team_positions: String::from("0"),
            mute: false,
            highscores: false,
            person_a: String::new(),
            person_b: String::new(),
            compare_conferences: String::from("1,2"),
            focus: 0,
            form_scroll: 0,
            sidebar_scroll: 0,
            sidebar_x_scroll: 0,
            message: String::from(
                "Ready. Use arrows like a terminal editor, Enter to run, and the mouse wheel in results.",
            ),
        }
    }
}

impl App {
    fn current_subject_value(&self) -> &'static str {
        SUBJECT_OPTIONS[self.subject_index].0
    }

    fn current_subject_label(&self) -> &'static str {
        SUBJECT_OPTIONS[self.subject_index].1
    }

    fn visible_fields(&self) -> Vec<FieldView> {
        let mut fields = vec![FieldView {
            id: FieldId::Mode,
            label: String::from("Mode"),
            value: self.mode.label().to_string(),
            help: String::from("Switch between search, compare, and rankings modes."),
        }];

        match self.mode {
            Mode::Search => {
                fields.push(FieldView {
                    id: FieldId::Subject,
                    label: String::from("Subject"),
                    value: self.current_subject_label().to_string(),
                    help: String::from("Academic event to scrape."),
                });
                fields.push(FieldView {
                    id: FieldId::SearchLevel,
                    label: String::from("Level"),
                    value: self.search_level.label().to_string(),
                    help: String::from("Auto uses the app's existing fallback logic."),
                });
                self.push_search_level_fields(&mut fields);
                self.push_common_search_fields(&mut fields, true);
            }
            Mode::Rankings => {
                fields.push(FieldView {
                    id: FieldId::SearchLevel,
                    label: String::from("Level"),
                    value: self.search_level.label().to_string(),
                    help: String::from("Rankings mode uses district, region, or state scope too."),
                });
                self.push_search_level_fields(&mut fields);
                self.push_common_search_fields(&mut fields, true);
            }
            Mode::Compare => {
                fields.push(FieldView {
                    id: FieldId::Subject,
                    label: String::from("Subject"),
                    value: self.current_subject_label().to_string(),
                    help: String::from("Academic event to compare inside."),
                });
                fields.push(FieldView {
                    id: FieldId::PersonA,
                    label: String::from("Person / school A"),
                    value: display_text_value(&self.person_a, true),
                    help: String::from("Exact person name or school name to keep in the output."),
                });
                fields.push(FieldView {
                    id: FieldId::PersonB,
                    label: String::from("Person / school B"),
                    value: display_text_value(&self.person_b, true),
                    help: String::from("Second exact person name or school name to keep in the output."),
                });
                fields.push(FieldView {
                    id: FieldId::CompareConferences,
                    label: String::from("Conferences"),
                    value: display_text_value(&self.compare_conferences, true),
                    help: String::from("Examples: 1,2 or 1A,2A or 1-4."),
                });
                fields.push(FieldView {
                    id: FieldId::CompareLevel,
                    label: String::from("Comparison level"),
                    value: self.compare_level.label().to_string(),
                    help: String::from("Choose whether to compare district, region, or state postings."),
                });
                self.push_common_tail(&mut fields, false);
            }
        }

        fields.push(FieldView {
            id: FieldId::Run,
            label: String::from("Run"),
            value: String::from("Start scrape"),
            help: String::from("Run the scraper. The timer starts here, not while you edit options."),
        });
        fields.push(FieldView {
            id: FieldId::Cancel,
            label: String::from("Cancel"),
            value: String::from("Quit"),
            help: String::from("Exit without running anything."),
        });

        fields
    }

    fn push_search_level_fields(&self, fields: &mut Vec<FieldView>) {
        match self.search_level {
            SearchLevel::Auto => {}
            SearchLevel::District => {
                fields.push(FieldView {
                    id: FieldId::District,
                    label: String::from("District"),
                    value: display_text_value(&self.district, true),
                    help: String::from("Use 0 for all districts, otherwise 1 through 32."),
                });
                fields.push(FieldView {
                    id: FieldId::DistrictRegionFilter,
                    label: String::from("District region filter"),
                    value: display_text_value(&self.district_region_filter, false),
                    help: String::from("Optional: limit districts to a region, 1 through 4. Leave blank or 0 for all."),
                });
            }
            SearchLevel::Region => {
                fields.push(FieldView {
                    id: FieldId::Region,
                    label: String::from("Region"),
                    value: display_text_value(&self.region, true),
                    help: String::from("Use 0 for all regions, otherwise 1 through 4."),
                });
            }
            SearchLevel::State => {}
        }
    }

    fn push_common_search_fields(&self, fields: &mut Vec<FieldView>, include_highscores: bool) {
        fields.push(FieldView {
            id: FieldId::Conference,
            label: String::from("Conference"),
            value: display_text_value(&self.conference, true),
            help: String::from("Examples: 1-6, 3, or 1A,4A."),
        });
        if include_highscores {
            fields.push(FieldView {
                id: FieldId::Highscores,
                label: String::from("Highscores"),
                value: bool_label(self.highscores),
                help: String::from("Show highest scores across conferences instead of standard results."),
            });
        }
        self.push_common_tail(fields, true);
    }

    fn push_common_tail(&self, fields: &mut Vec<FieldView>, include_find: bool) {
        fields.push(FieldView {
            id: FieldId::Year,
            label: String::from("Year"),
            value: display_text_value(&self.year, true),
            help: String::from("Leave blank to use the current year."),
        });
        if include_find || self.mode == Mode::Compare {
            fields.push(FieldView {
                id: FieldId::Find,
                label: String::from("Find filter"),
                value: display_text_value(&self.find, false),
                help: String::from("Optional substring filter applied to the rendered results."),
            });
        }
        fields.push(FieldView {
            id: FieldId::IndividualPositions,
            label: String::from("Individual positions"),
            value: display_text_value(&self.individual_positions, true),
            help: String::from("How many individual places to show. Use 0 for all."),
        });
        fields.push(FieldView {
            id: FieldId::TeamPositions,
            label: String::from("Team positions"),
            value: display_text_value(&self.team_positions, true),
            help: String::from("How many team places to show. Use 0 for all."),
        });
        fields.push(FieldView {
            id: FieldId::Mute,
            label: String::from("Mute progress"),
            value: bool_label(self.mute),
            help: String::from("Suppress completed / unavailable status lines in non-TUI runs."),
        });
    }

    fn visible_field_ids(&self) -> Vec<FieldId> {
        self.visible_fields().into_iter().map(|field| field.id).collect()
    }

    fn keep_focus_valid(&mut self) {
        let count = self.visible_field_ids().len();
        if count == 0 {
            self.focus = 0;
        } else if self.focus >= count {
            self.focus = count - 1;
        }
    }

    fn current_field_id(&self) -> FieldId {
        self.visible_field_ids()
            .get(self.focus)
            .copied()
            .unwrap_or(FieldId::Mode)
    }

    fn reset_sidebar_view(&mut self) {
        self.sidebar_scroll = 0;
        self.sidebar_x_scroll = 0;
    }

    fn focus_next(&mut self) {
        let count = self.visible_field_ids().len();
        if count == 0 {
            self.focus = 0;
        } else {
            self.focus = (self.focus + 1) % count;
        }
        self.reset_sidebar_view();
    }

    fn focus_previous(&mut self) {
        let count = self.visible_field_ids().len();
        if count == 0 {
            self.focus = 0;
        } else {
            self.focus = (self.focus + count - 1) % count;
        }
        self.reset_sidebar_view();
    }

    fn ensure_focus_visible(&mut self, form_rows: usize) {
        if form_rows == 0 {
            self.form_scroll = 0;
            return;
        }
        if self.focus < self.form_scroll {
            self.form_scroll = self.focus;
        } else if self.focus >= self.form_scroll + form_rows {
            self.form_scroll = self.focus + 1 - form_rows;
        }
    }

    fn set_focus_from_row(&mut self, relative_row: usize, form_rows: usize) {
        let total = self.visible_field_ids().len();
        if total == 0 {
            self.focus = 0;
            return;
        }
        let max_index = total - 1;
        self.focus = (self.form_scroll + relative_row).min(max_index);
        self.ensure_focus_visible(form_rows);
        self.reset_sidebar_view();
    }

    fn handle_key(&mut self, key: KeyEvent) -> AppAction {
        match key.code {
            KeyCode::Esc => return AppAction::Cancel,
            KeyCode::Up => {
                self.focus_previous();
                self.message = String::from("Moved to previous field.");
            }
            KeyCode::Down => {
                self.focus_next();
                self.message = String::from("Moved to next field.");
            }
            KeyCode::Tab => {
                self.focus_next();
                self.message = String::from("Moved to next field.");
            }
            KeyCode::BackTab => {
                self.focus_previous();
                self.message = String::from("Moved to previous field.");
            }
            KeyCode::Left if key.modifiers.contains(KeyModifiers::SHIFT) => self.cycle_current(false),
            KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => self.cycle_current(true),
            KeyCode::Left => self.cycle_current(false),
            KeyCode::Right => self.cycle_current(true),
            KeyCode::Enter => return self.activate_current(),
            KeyCode::Backspace => self.backspace_current(),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return AppAction::Cancel
            }
            KeyCode::Char(c) => self.insert_char_current(c),
            _ => {}
        }

        self.keep_focus_valid();
        AppAction::Continue
    }

    fn activate_current(&mut self) -> AppAction {
        match self.current_field_id() {
            FieldId::Run => AppAction::Submit,
            FieldId::Cancel => AppAction::Cancel,
            FieldId::Mute | FieldId::Highscores => {
                self.cycle_current(true);
                AppAction::Continue
            }
            _ => {
                self.focus_next();
                self.message = String::from("Moved to next field.");
                AppAction::Continue
            }
        }
    }

    fn cycle_current(&mut self, forward: bool) {
        match self.current_field_id() {
            FieldId::Mode => {
                self.mode = if forward { self.mode.next() } else { self.mode.previous() };
                self.keep_focus_valid();
                self.message = format!("Mode: {}", self.mode.label());
            }
            FieldId::Subject => {
                if forward {
                    self.subject_index = (self.subject_index + 1) % SUBJECT_OPTIONS.len();
                } else if self.subject_index == 0 {
                    self.subject_index = SUBJECT_OPTIONS.len() - 1;
                } else {
                    self.subject_index -= 1;
                }
                self.message = format!("Subject: {}", self.current_subject_label());
            }
            FieldId::SearchLevel => {
                self.search_level = if forward {
                    self.search_level.next()
                } else {
                    self.search_level.previous()
                };
                self.keep_focus_valid();
                self.message = format!("Level: {}", self.search_level.label());
            }
            FieldId::CompareLevel => {
                self.compare_level = if forward {
                    self.compare_level.next()
                } else {
                    self.compare_level.previous()
                };
                self.message = format!("Comparison level: {}", self.compare_level.label());
            }
            FieldId::Mute => {
                self.mute = !self.mute;
                self.message = format!("Mute progress {}.", on_off(self.mute));
            }
            FieldId::Highscores => {
                self.highscores = !self.highscores;
                self.message = format!("Highscores {}.", on_off(self.highscores));
            }
            _ => {
                self.message = String::from("Type into this field or press Enter.");
            }
        }
    }

    fn backspace_current(&mut self) {
        let field = self.current_field_id();
        let target = match field {
            FieldId::Conference => Some(&mut self.conference),
            FieldId::District => Some(&mut self.district),
            FieldId::DistrictRegionFilter => Some(&mut self.district_region_filter),
            FieldId::Region => Some(&mut self.region),
            FieldId::Year => Some(&mut self.year),
            FieldId::Find => Some(&mut self.find),
            FieldId::IndividualPositions => Some(&mut self.individual_positions),
            FieldId::TeamPositions => Some(&mut self.team_positions),
            FieldId::PersonA => Some(&mut self.person_a),
            FieldId::PersonB => Some(&mut self.person_b),
            FieldId::CompareConferences => Some(&mut self.compare_conferences),
            _ => None,
        };

        if let Some(text) = target {
            text.pop();
            self.message = String::from("Edited field.");
        }
    }

    fn insert_char_current(&mut self, c: char) {
        let field = self.current_field_id();
        match field {
            FieldId::Conference => {
                if is_conference_char(c) {
                    self.conference.push(c);
                    self.message = String::from("Edited conference.");
                }
            }
            FieldId::District => {
                if c.is_ascii_digit() {
                    self.district.push(c);
                    self.message = String::from("Edited district.");
                }
            }
            FieldId::DistrictRegionFilter => {
                if c.is_ascii_digit() {
                    self.district_region_filter.push(c);
                    self.message = String::from("Edited district region filter.");
                }
            }
            FieldId::Region => {
                if c.is_ascii_digit() {
                    self.region.push(c);
                    self.message = String::from("Edited region.");
                }
            }
            FieldId::Year => {
                if c.is_ascii_digit() {
                    self.year.push(c);
                    self.message = String::from("Edited year.");
                }
            }
            FieldId::Find => {
                if !c.is_control() {
                    self.find.push(c);
                    self.message = String::from("Edited find filter.");
                }
            }
            FieldId::IndividualPositions => {
                if c.is_ascii_digit() {
                    self.individual_positions.push(c);
                    self.message = String::from("Edited individual positions.");
                }
            }
            FieldId::TeamPositions => {
                if c.is_ascii_digit() {
                    self.team_positions.push(c);
                    self.message = String::from("Edited team positions.");
                }
            }
            FieldId::PersonA => {
                if !c.is_control() {
                    self.person_a.push(c);
                    self.message = String::from("Edited first person / school.");
                }
            }
            FieldId::PersonB => {
                if !c.is_control() {
                    self.person_b.push(c);
                    self.message = String::from("Edited second person / school.");
                }
            }
            FieldId::CompareConferences => {
                if is_conference_char(c) {
                    self.compare_conferences.push(c);
                    self.message = String::from("Edited compare conferences.");
                }
            }
            _ => {}
        }
    }

    fn build_cli(&self) -> Result<Cli, String> {
        let year = parse_optional_u16(&self.year, "year")?;
        let find = string_or_none(&self.find);
        let individual_positions = parse_optional_usize(&self.individual_positions, "individual positions")?;
        let team_positions = parse_optional_usize(&self.team_positions, "team positions")?;

        match self.mode {
            Mode::Search => {
                let conference = Some(validate_conference(&self.conference)?);
                let (district, region, state) = self.build_search_level()?;
                Ok(Cli {
                    subject: self.current_subject_value().to_string(),
                    conference,
                    district,
                    region,
                    state,
                    year,
                    find,
                    individual_positions,
                    team_positions,
                    mute: self.mute,
                    highscores: self.highscores,
                    command: None,
                })
            }
            Mode::Rankings => {
                let conference = Some(validate_conference(&self.conference)?);
                let (district, region, state) = self.build_search_level()?;
                Ok(Cli {
                    subject: String::from("rankings"),
                    conference,
                    district,
                    region,
                    state,
                    year,
                    find,
                    individual_positions,
                    team_positions,
                    mute: self.mute,
                    highscores: self.highscores,
                    command: None,
                })
            }
            Mode::Compare => {
                let person_a = required_text(&self.person_a, "person / school A")?;
                let person_b = required_text(&self.person_b, "person / school B")?;
                let conferences = validate_conference(&self.compare_conferences)?;
                Ok(Cli {
                    subject: self.current_subject_value().to_string(),
                    conference: None,
                    district: None,
                    region: None,
                    state: false,
                    year,
                    find,
                    individual_positions,
                    team_positions,
                    mute: self.mute,
                    highscores: false,
                    command: Some(Commands::Compare {
                        person_a,
                        person_b,
                        conferences,
                        district: self.compare_level == CompareLevel::District,
                        region: self.compare_level == CompareLevel::Region,
                        state: self.compare_level == CompareLevel::State,
                    }),
                })
            }
        }
    }

    fn build_search_level(&self) -> Result<(Option<u8>, Option<u8>, bool), String> {
        match self.search_level {
            SearchLevel::Auto => Ok((None, None, false)),
            SearchLevel::District => {
                let district = parse_bounded_u8(&self.district, "district", 0, 32)?.unwrap_or(0);
                let region_filter = parse_bounded_u8(&self.district_region_filter, "district region filter", 0, 4)?;
                Ok((Some(district), region_filter, false))
            }
            SearchLevel::Region => {
                let region = parse_bounded_u8(&self.region, "region", 0, 4)?.unwrap_or(0);
                Ok((None, Some(region), false))
            }
            SearchLevel::State => Ok((None, None, true)),
        }
    }

    fn preview_command(&self) -> String {
        match self.build_cli() {
            Ok(cli) => build_preview_from_cli(&cli),
            Err(error) => format!("Validation pending: {error}"),
        }
    }

    fn sidebar_items(&self) -> Vec<SidebarItem> {
        let mut items = Vec::new();
        let current_year = chrono::Utc::now().year().max(2004) as u16;

        match self.current_field_id() {
            FieldId::Mode => {
                sidebar_heading(&mut items, "Modes");
                for mode in Mode::ALL {
                    sidebar_option(&mut items, mode.label().to_string(), SidebarAction::SetMode(mode));
                }
            }
            FieldId::Subject => {
                sidebar_heading(&mut items, "Subjects (UIL order)");
                for (index, (_, label)) in SUBJECT_OPTIONS.iter().enumerate() {
                    sidebar_option(&mut items, (*label).to_string(), SidebarAction::SetSubject(index));
                }
                sidebar_heading(&mut items, "Subjects (alphabetical)");
                for (value, label) in subject_options_alphabetical() {
                    if let Some(index) = SUBJECT_OPTIONS.iter().position(|(candidate, _)| *candidate == value) {
                        sidebar_option(&mut items, label.to_string(), SidebarAction::SetSubject(index));
                    }
                }
            }
            FieldId::SearchLevel => {
                sidebar_heading(&mut items, "Levels");
                for level in SearchLevel::ALL {
                    sidebar_option(&mut items, level.label().to_string(), SidebarAction::SetSearchLevel(level));
                }
            }
            FieldId::CompareLevel => {
                sidebar_heading(&mut items, "Comparison levels");
                for level in CompareLevel::ALL {
                    sidebar_option(&mut items, level.label().to_string(), SidebarAction::SetCompareLevel(level));
                }
            }
            FieldId::Conference => {
                sidebar_heading(&mut items, "Conference presets");
                sidebar_option(&mut items, "All conferences (1-6)".to_string(), SidebarAction::SetText(FieldId::Conference, "1-6".to_string()));
                for conference in 1..=6 {
                    sidebar_option(&mut items, format!("{conference}A only"), SidebarAction::SetText(FieldId::Conference, conference.to_string()));
                }
                sidebar_option(&mut items, "Small schools (1-3)".to_string(), SidebarAction::SetText(FieldId::Conference, "1-3".to_string()));
                sidebar_option(&mut items, "Large schools (4-6)".to_string(), SidebarAction::SetText(FieldId::Conference, "4-6".to_string()));
            }
            FieldId::CompareConferences => {
                sidebar_heading(&mut items, "Conference presets");
                sidebar_option(&mut items, "All conferences (1-6)".to_string(), SidebarAction::SetText(FieldId::CompareConferences, "1-6".to_string()));
                sidebar_option(&mut items, "1A and 2A".to_string(), SidebarAction::SetText(FieldId::CompareConferences, "1,2".to_string()));
                sidebar_option(&mut items, "3A and 4A".to_string(), SidebarAction::SetText(FieldId::CompareConferences, "3,4".to_string()));
                sidebar_option(&mut items, "5A and 6A".to_string(), SidebarAction::SetText(FieldId::CompareConferences, "5,6".to_string()));
                for conference in 1..=6 {
                    sidebar_option(&mut items, format!("{conference}A only"), SidebarAction::SetText(FieldId::CompareConferences, conference.to_string()));
                }
            }
            FieldId::District => {
                sidebar_heading(&mut items, "Districts");
                sidebar_option(&mut items, "0 = all districts".to_string(), SidebarAction::SetText(FieldId::District, "0".to_string()));
                for district in 1..=32 {
                    sidebar_option(&mut items, format!("District {district}"), SidebarAction::SetText(FieldId::District, district.to_string()));
                }
            }
            FieldId::DistrictRegionFilter => {
                sidebar_heading(&mut items, "District region filter");
                sidebar_option(&mut items, "Blank = any region".to_string(), SidebarAction::SetText(FieldId::DistrictRegionFilter, String::new()));
                sidebar_option(&mut items, "0 = all regions".to_string(), SidebarAction::SetText(FieldId::DistrictRegionFilter, "0".to_string()));
                for region in 1..=4 {
                    sidebar_option(&mut items, format!("Region {region}"), SidebarAction::SetText(FieldId::DistrictRegionFilter, region.to_string()));
                }
            }
            FieldId::Region => {
                sidebar_heading(&mut items, "Regions");
                sidebar_option(&mut items, "0 = all regions".to_string(), SidebarAction::SetText(FieldId::Region, "0".to_string()));
                for region in 1..=4 {
                    sidebar_option(&mut items, format!("Region {region}"), SidebarAction::SetText(FieldId::Region, region.to_string()));
                }
            }
            FieldId::Year => {
                sidebar_heading(&mut items, "Recent years");
                sidebar_option(&mut items, "Blank = current year".to_string(), SidebarAction::SetText(FieldId::Year, String::new()));
                for year in (current_year.saturating_sub(10)..=current_year).rev() {
                    sidebar_option(&mut items, year.to_string(), SidebarAction::SetText(FieldId::Year, year.to_string()));
                }
            }
            FieldId::Find => {
                sidebar_heading(&mut items, "Find filter examples");
                sidebar_info(&mut items, "Type any competitor or school substring.");
                sidebar_option(&mut items, "Clear filter".to_string(), SidebarAction::SetText(FieldId::Find, String::new()));
            }
            FieldId::IndividualPositions => {
                sidebar_heading(&mut items, "Individual position presets");
                for value in [0usize, 10, 25, 50, 100] {
                    let label = if value == 0 { "0 = show all".to_string() } else { value.to_string() };
                    sidebar_option(&mut items, label, SidebarAction::SetText(FieldId::IndividualPositions, value.to_string()));
                }
            }
            FieldId::TeamPositions => {
                sidebar_heading(&mut items, "Team position presets");
                for value in [0usize, 10, 25, 50, 100] {
                    let label = if value == 0 { "0 = show all".to_string() } else { value.to_string() };
                    sidebar_option(&mut items, label, SidebarAction::SetText(FieldId::TeamPositions, value.to_string()));
                }
            }
            FieldId::Mute => {
                sidebar_heading(&mut items, "Mute progress");
                sidebar_option(&mut items, "No".to_string(), SidebarAction::SetBool(FieldId::Mute, false));
                sidebar_option(&mut items, "Yes".to_string(), SidebarAction::SetBool(FieldId::Mute, true));
            }
            FieldId::Highscores => {
                sidebar_heading(&mut items, "Highscores mode");
                sidebar_option(&mut items, "No".to_string(), SidebarAction::SetBool(FieldId::Highscores, false));
                sidebar_option(&mut items, "Yes".to_string(), SidebarAction::SetBool(FieldId::Highscores, true));
            }
            FieldId::PersonA => {
                sidebar_heading(&mut items, "Person / school A");
                sidebar_info(&mut items, "Type an exact person name or school name.");
                sidebar_option(&mut items, "Clear".to_string(), SidebarAction::SetText(FieldId::PersonA, String::new()));
            }
            FieldId::PersonB => {
                sidebar_heading(&mut items, "Person / school B");
                sidebar_info(&mut items, "Type an exact person name or school name.");
                sidebar_option(&mut items, "Clear".to_string(), SidebarAction::SetText(FieldId::PersonB, String::new()));
            }
            FieldId::Run => {
                sidebar_heading(&mut items, "Run");
                sidebar_info(&mut items, "Press Enter here to start scraping with the current settings.");
            }
            FieldId::Cancel => {
                sidebar_heading(&mut items, "Quit");
                sidebar_info(&mut items, "Press Enter here to leave without running anything.");
            }
        }

        items
    }

    fn apply_sidebar_action(&mut self, action: SidebarAction) {
        match action {
            SidebarAction::SetMode(mode) => {
                self.mode = mode;
                self.keep_focus_valid();
                self.message = format!("Mode: {}", self.mode.label());
            }
            SidebarAction::SetSubject(index) => {
                self.subject_index = index.min(SUBJECT_OPTIONS.len().saturating_sub(1));
                self.message = format!("Subject: {}", self.current_subject_label());
            }
            SidebarAction::SetSearchLevel(level) => {
                self.search_level = level;
                self.keep_focus_valid();
                self.message = format!("Level: {}", self.search_level.label());
            }
            SidebarAction::SetCompareLevel(level) => {
                self.compare_level = level;
                self.message = format!("Comparison level: {}", self.compare_level.label());
            }
            SidebarAction::SetText(field, value) => {
                match field {
                    FieldId::Conference => self.conference = value,
                    FieldId::District => self.district = value,
                    FieldId::DistrictRegionFilter => self.district_region_filter = value,
                    FieldId::Region => self.region = value,
                    FieldId::Year => self.year = value,
                    FieldId::Find => self.find = value,
                    FieldId::IndividualPositions => self.individual_positions = value,
                    FieldId::TeamPositions => self.team_positions = value,
                    FieldId::PersonA => self.person_a = value,
                    FieldId::PersonB => self.person_b = value,
                    FieldId::CompareConferences => self.compare_conferences = value,
                    _ => {}
                }
                self.message = String::from("Applied option from the right panel.");
            }
            SidebarAction::SetBool(field, value) => {
                match field {
                    FieldId::Mute => self.mute = value,
                    FieldId::Highscores => self.highscores = value,
                    _ => {}
                }
                self.message = String::from("Applied option from the right panel.");
            }
        }
    }
}

fn sidebar_heading(items: &mut Vec<SidebarItem>, text: &str) {
    items.push(SidebarItem {
        text: text.to_string(),
        kind: SidebarItemKind::Heading,
        color: Color::Yellow,
        bold: true,
    });
}

fn sidebar_info(items: &mut Vec<SidebarItem>, text: &str) {
    items.push(SidebarItem {
        text: text.to_string(),
        kind: SidebarItemKind::Info,
        color: Color::Grey,
        bold: false,
    });
}

fn sidebar_option(items: &mut Vec<SidebarItem>, text: String, action: SidebarAction) {
    items.push(SidebarItem {
        text,
        kind: SidebarItemKind::Option(action),
        color: Color::White,
        bold: false,
    });
}

fn sidebar_action_selected(app: &App, action: &SidebarAction) -> bool {
    match action {
        SidebarAction::SetMode(mode) => app.mode == *mode,
        SidebarAction::SetSubject(index) => app.subject_index == *index,
        SidebarAction::SetSearchLevel(level) => app.search_level == *level,
        SidebarAction::SetCompareLevel(level) => app.compare_level == *level,
        SidebarAction::SetText(field, value) => match field {
            FieldId::Conference => app.conference.trim() == value.trim(),
            FieldId::District => app.district.trim() == value.trim(),
            FieldId::DistrictRegionFilter => app.district_region_filter.trim() == value.trim(),
            FieldId::Region => app.region.trim() == value.trim(),
            FieldId::Year => app.year.trim() == value.trim(),
            FieldId::Find => app.find.trim() == value.trim(),
            FieldId::IndividualPositions => app.individual_positions.trim() == value.trim(),
            FieldId::TeamPositions => app.team_positions.trim() == value.trim(),
            FieldId::PersonA => app.person_a.trim() == value.trim(),
            FieldId::PersonB => app.person_b.trim() == value.trim(),
            FieldId::CompareConferences => app.compare_conferences.trim() == value.trim(),
            _ => false,
        },
        SidebarAction::SetBool(field, value) => match field {
            FieldId::Mute => app.mute == *value,
            FieldId::Highscores => app.highscores == *value,
            _ => false,
        },
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AppAction {
    Continue,
    Submit,
    Cancel,
}

enum RunningOutcome {
    Completed(RunOutput),
    Cancelled,
}

struct TerminalGuard;

impl TerminalGuard {
    fn enter() -> io::Result<Self> {
        enable_raw_mode()?;
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            cursor::Hide,
            EnableMouseCapture
        )?;
        Ok(Self)
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            LeaveAlternateScreen,
            cursor::Show
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultsPane {
    Individuals,
    Teams,
}

impl ResultsPane {
    fn label(self) -> &'static str {
        match self {
            Self::Individuals => "Individuals",
            Self::Teams => "Teams",
        }
    }

    fn opposite(self) -> Self {
        match self {
            Self::Individuals => Self::Teams,
            Self::Teams => Self::Individuals,
        }
    }
}

#[derive(Clone, Debug)]
struct StyledLine {
    text: String,
    color: Color,
    bold: bool,
}

#[derive(Clone, Debug)]
struct PaneSection {
    title: String,
    lines: Vec<StyledLine>,
}

#[derive(Clone, Debug)]
struct ResultPaneData {
    title: String,
    sections: Vec<PaneSection>,
}

#[derive(Clone, Debug)]
struct RenderedPane {
    title: String,
    lines: Vec<StyledLine>,
    match_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ResultsEditMode {
    Normal,
    Search,
}

#[derive(Clone, Debug)]
struct ResultsView {
    active: ResultsPane,
    individual_scroll: usize,
    team_scroll: usize,
    individual_x: usize,
    team_x: usize,
    individuals: ResultPaneData,
    teams: ResultPaneData,
    conference_filter: Option<u8>,
    search_query: String,
    edit_mode: ResultsEditMode,
    stats: ProgressSnapshot,
}

impl ResultsView {
    fn from_output(output: &RunOutput, stats: ProgressSnapshot) -> Self {
        let (individual_sections, team_sections) = split_output_sections(&output.sections);
        Self {
            active: ResultsPane::Individuals,
            individual_scroll: 0,
            team_scroll: 0,
            individual_x: 0,
            team_x: 0,
            individuals: build_result_pane("Individuals", &individual_sections, Color::Cyan),
            teams: build_result_pane("Teams", &team_sections, Color::Blue),
            conference_filter: None,
            search_query: String::new(),
            edit_mode: ResultsEditMode::Normal,
            stats,
        }
    }

    fn pane_scroll(&self, pane: ResultsPane) -> usize {
        match pane {
            ResultsPane::Individuals => self.individual_scroll,
            ResultsPane::Teams => self.team_scroll,
        }
    }

    fn pane_data(&self, pane: ResultsPane) -> &ResultPaneData {
        match pane {
            ResultsPane::Individuals => &self.individuals,
            ResultsPane::Teams => &self.teams,
        }
    }

    fn rendered_pane(&self, pane: ResultsPane) -> RenderedPane {
        self.pane_data(pane)
            .render(self.conference_filter, &self.search_query)
    }

    fn set_pane_scroll(&mut self, pane: ResultsPane, value: usize) {
        match pane {
            ResultsPane::Individuals => self.individual_scroll = value,
            ResultsPane::Teams => self.team_scroll = value,
        }
    }

    fn pane_x_scroll(&self, pane: ResultsPane) -> usize {
        match pane {
            ResultsPane::Individuals => self.individual_x,
            ResultsPane::Teams => self.team_x,
        }
    }

    fn set_pane_x_scroll(&mut self, pane: ResultsPane, value: usize) {
        match pane {
            ResultsPane::Individuals => self.individual_x = value,
            ResultsPane::Teams => self.team_x = value,
        }
    }

    fn scroll_active_x_by(&mut self, delta: isize, visible_width: usize) {
        let pane = self.active;
        let current = self.pane_x_scroll(pane);
        let max_scroll = self.max_x_scroll(pane, visible_width);
        let new_value = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(max_scroll)
        };
        self.set_pane_x_scroll(pane, new_value.min(max_scroll));
    }

    fn max_x_scroll(&self, pane: ResultsPane, visible_width: usize) -> usize {
        let longest = self
            .rendered_pane(pane)
            .lines
            .iter()
            .map(|line| line.text.chars().count())
            .max()
            .unwrap_or(0);
        longest.saturating_sub(visible_width)
    }

    fn normalize_x_scrolls(&mut self, visible_width: usize) {
        let indiv_max = self.max_x_scroll(ResultsPane::Individuals, visible_width);
        let team_max = self.max_x_scroll(ResultsPane::Teams, visible_width);
        self.individual_x = self.individual_x.min(indiv_max);
        self.team_x = self.team_x.min(team_max);
    }

    fn activate(&mut self, pane: ResultsPane) {
        self.active = pane;
    }

    fn switch_active(&mut self) {
        self.active = self.active.opposite();
    }

    fn scroll_active_by(&mut self, delta: isize, visible_rows: usize) {
        self.scroll_pane_by(self.active, delta, visible_rows);
    }

    fn scroll_pane_by(&mut self, pane: ResultsPane, delta: isize, visible_rows: usize) {
        let total = self.rendered_pane(pane).lines.len();
        let max_scroll = total.saturating_sub(visible_rows);
        let current = self.pane_scroll(pane);
        let new_value = if delta < 0 {
            current.saturating_sub(delta.unsigned_abs())
        } else {
            current.saturating_add(delta as usize).min(max_scroll)
        };
        self.set_pane_scroll(pane, new_value.min(max_scroll));
    }

    fn set_active_scroll(&mut self, value: usize, visible_rows: usize) {
        let total = self.rendered_pane(self.active).lines.len();
        let max_scroll = total.saturating_sub(visible_rows);
        self.set_pane_scroll(self.active, value.min(max_scroll));
    }

    fn normalize_scrolls(&mut self, visible_rows: usize) {
        let indiv_max = self
            .rendered_pane(ResultsPane::Individuals)
            .lines
            .len()
            .saturating_sub(visible_rows);
        let team_max = self
            .rendered_pane(ResultsPane::Teams)
            .lines
            .len()
            .saturating_sub(visible_rows);
        self.individual_scroll = self.individual_scroll.min(indiv_max);
        self.team_scroll = self.team_scroll.min(team_max);
    }

    fn cycle_conference_filter(&mut self) {
        self.conference_filter = match self.conference_filter {
            None => Some(1),
            Some(value) if value >= 6 => None,
            Some(value) => Some(value + 1),
        };
        self.normalize_scrolls(usize::MAX / 4);
    }

    fn clear_filters(&mut self) {
        self.conference_filter = None;
        self.search_query.clear();
        self.edit_mode = ResultsEditMode::Normal;
        self.normalize_scrolls(usize::MAX / 4);
    }

    fn filters_active(&self) -> bool {
        self.conference_filter.is_some() || !self.search_query.trim().is_empty()
    }

    fn conference_filter_label(&self) -> String {
        match self.conference_filter {
            Some(value) => format!("{}A", value),
            None => String::from("All conferences"),
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent, width: u16, height: u16) {
        let pane_top = results_pane_top();
        let pane_height = results_pane_height(height);
        let content_rows = pane_content_rows(pane_height);
        if pane_height == 0 {
            return;
        }
        let in_panes = mouse.row >= pane_top && mouse.row < pane_top.saturating_add(pane_height);
        if !in_panes {
            return;
        }
        let pane = pane_from_column(mouse.column, width);
        let horizontal_to_right = matches!(mouse.kind, MouseEventKind::ScrollRight)
            || (matches!(mouse.kind, MouseEventKind::ScrollDown)
                && mouse.modifiers.contains(KeyModifiers::SHIFT));
        let horizontal_to_left = matches!(mouse.kind, MouseEventKind::ScrollLeft)
            || (matches!(mouse.kind, MouseEventKind::ScrollUp)
                && mouse.modifiers.contains(KeyModifiers::SHIFT));

        if horizontal_to_left {
            self.activate(ResultsPane::Individuals);
            return;
        }
        if horizontal_to_right {
            self.activate(ResultsPane::Teams);
            return;
        }

        match mouse.kind {
            MouseEventKind::ScrollUp => {
                self.activate(pane);
                self.scroll_pane_by(pane, -3, content_rows);
            }
            MouseEventKind::ScrollDown => {
                self.activate(pane);
                self.scroll_pane_by(pane, 3, content_rows);
            }
            MouseEventKind::Down(MouseButton::Left) => {
                self.activate(pane);
            }
            _ => {}
        }
    }
}

impl ResultPaneData {
    fn render(&self, conference_filter: Option<u8>, search_query: &str) -> RenderedPane {
        let mut lines = Vec::new();
        let mut match_count = 0usize;
        let search = search_query.trim().to_ascii_lowercase();
        let filters_active = conference_filter.is_some() || !search.is_empty();

        for (section_index, section) in self.sections.iter().enumerate() {
            let mut section_lines = Vec::new();
            let mut filtered_place = 0usize;

            for line in &section.lines {
                if !line_matches_filters(&line.text, conference_filter, &search) {
                    continue;
                }
                let mut styled = line.clone();
                if filters_active {
                    if extract_place(&line.text).is_some() {
                        filtered_place += 1;
                        styled.text = format!("{:>3} │ {}", filtered_place, line.text);
                    }
                }
                if extract_place(&line.text).is_some() {
                    match_count += 1;
                }
                section_lines.push(styled);
            }

            if !section_lines.is_empty() || !filters_active {
                lines.push(StyledLine {
                    text: section.title.clone(),
                    color: Color::Cyan,
                    bold: true,
                });
                if filters_active && section_lines.is_empty() {
                    lines.push(StyledLine {
                        text: String::from("No matches in this section."),
                        color: Color::DarkGrey,
                        bold: false,
                    });
                } else {
                    lines.extend(section_lines);
                }
                if section_index + 1 < self.sections.len() {
                    lines.push(StyledLine {
                        text: String::new(),
                        color: Color::White,
                        bold: false,
                    });
                }
            }
        }

        if lines.is_empty() {
            lines.push(StyledLine {
                text: String::from("No matching results."),
                color: Color::DarkGrey,
                bold: false,
            });
        }

        RenderedPane {
            title: self.title.clone(),
            lines,
            match_count,
        }
    }
}

pub fn run_tui() {
    let _guard = TerminalGuard::enter().expect("failed to initialize interactive terminal UI");
    let mut stdout = io::stdout();
    let mut app = App::default();

    loop {
        let (_, height) = terminal::size().unwrap_or((120, 30));
        let form_rows = form_visible_rows(height);
        app.keep_focus_valid();
        app.ensure_focus_visible(form_rows);
        draw(&mut stdout, &app).expect("failed to draw terminal UI");

        match event::read().expect("failed to read terminal event") {
            Event::Key(key) => match app.handle_key(key) {
                AppAction::Continue => {}
                AppAction::Submit => match app.build_cli() {
                    Ok(cli) => {
                        let shared_progress = Arc::new(Mutex::new(ProgressSnapshot {
                            last_message: String::from("Starting scrape..."),
                            current_label: String::from("Preparing requests..."),
                            ..ProgressSnapshot::default()
                        }));
                        let progress_for_thread = shared_progress.clone();
                        let (tx, rx) = mpsc::channel();
                        thread::spawn(move || {
                            let _progress_guard = progress::install(progress_for_thread.clone());
                            progress::set_message("Starting scrape...");
                            let result = execute_cli(cli, false);
                            progress::mark_finished();
                            let _ = tx.send(result);
                        });

                        match running_loop(&mut stdout, &app, shared_progress.clone(), rx) {
                            Ok(RunningOutcome::Completed(output)) => {
                                let final_stats = snapshot_copy(&shared_progress);
                                if results_loop(&mut stdout, &output, final_stats)
                                    .expect("failed to show results")
                                {
                                    app.message = String::from("Returned to the menu.");
                                    continue;
                                }
                                return;
                            }
                            Ok(RunningOutcome::Cancelled) => {
                                app.message = String::from("Scrape cancelled. Back at the menu.");
                                continue;
                            }
                            Err(error) => {
                                app.message = error.to_string();
                                continue;
                            }
                        }
                    }
                    Err(error) => app.message = error,
                },
                AppAction::Cancel => return,
            },
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    handle_form_mouse_scroll(&mut app, mouse, height, form_rows);
                }
                MouseEventKind::Down(MouseButton::Left) => {
                    handle_form_click(&mut app, mouse, height);
                }
                _ => {}
            },
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn handle_form_click(app: &mut App, mouse: MouseEvent, height: u16) {
    let (width, _) = terminal::size().unwrap_or((120, height));
    let top = form_top();
    let rows = form_visible_rows(height);
    let (left_width, right_x, right_width) = form_layout(width);
    if rows == 0 || mouse.row < top {
        return;
    }

    if mouse.column < left_width {
        if mouse.row >= top && mouse.row < top.saturating_add(rows as u16) {
            let relative_row = (mouse.row - top) as usize;
            app.set_focus_from_row(relative_row, rows);
            app.message = String::from("Focused field.");
        }
        return;
    }

    if right_width == 0 || mouse.column < right_x {
        return;
    }

    let option_top = sidebar_options_top();
    let option_rows = sidebar_visible_rows(height);
    if mouse.row < option_top || mouse.row >= option_top.saturating_add(option_rows as u16) {
        return;
    }

    let index = app.sidebar_scroll + (mouse.row - option_top) as usize;
    let items = app.sidebar_items();
    if let Some(item) = items.get(index) {
        if let SidebarItemKind::Option(action) = &item.kind {
            app.apply_sidebar_action(action.clone());
        }
    }
}

fn handle_form_mouse_scroll(app: &mut App, mouse: MouseEvent, height: u16, form_rows: usize) {
    let (width, _) = terminal::size().unwrap_or((120, height));
    let (left_width, _right_x, _right_width) = form_layout(width);

    let horizontal_forward = matches!(mouse.kind, MouseEventKind::ScrollRight)
        || (matches!(mouse.kind, MouseEventKind::ScrollDown)
            && mouse.modifiers.contains(KeyModifiers::SHIFT));
    let horizontal_backward = matches!(mouse.kind, MouseEventKind::ScrollLeft)
        || (matches!(mouse.kind, MouseEventKind::ScrollUp)
            && mouse.modifiers.contains(KeyModifiers::SHIFT));

    if horizontal_forward {
        app.cycle_current(true);
        return;
    }
    if horizontal_backward {
        app.cycle_current(false);
        return;
    }

    if mouse.column < left_width {
        match mouse.kind {
            MouseEventKind::ScrollUp => {
                app.focus_previous();
                app.ensure_focus_visible(form_rows);
            }
            MouseEventKind::ScrollDown => {
                app.focus_next();
                app.ensure_focus_visible(form_rows);
            }
            _ => {}
        }
        return;
    }

    let option_rows = sidebar_visible_rows(height);
    let item_count = app.sidebar_items().len();
    let max_scroll = item_count.saturating_sub(option_rows);
    match mouse.kind {
        MouseEventKind::ScrollUp => {
            app.sidebar_scroll = app.sidebar_scroll.saturating_sub(2);
        }
        MouseEventKind::ScrollDown => {
            app.sidebar_scroll = app.sidebar_scroll.saturating_add(2).min(max_scroll);
        }
        _ => {}
    }
}

fn running_loop(
    stdout: &mut Stdout,
    app: &App,
    shared_progress: Arc<Mutex<ProgressSnapshot>>,
    rx: mpsc::Receiver<Result<RunOutput, String>>,
) -> io::Result<RunningOutcome> {
    let started_at = Instant::now();
    let mut cancelling = false;

    loop {
        let snapshot = snapshot_copy(&shared_progress);
        draw_running(stdout, app, &snapshot, started_at.elapsed())?;

        match rx.try_recv() {
            Ok(result) => match result {
                Ok(output) => {
                    if cancelling || snapshot.cancel_requested {
                        return Ok(RunningOutcome::Cancelled);
                    }
                    return Ok(RunningOutcome::Completed(output));
                }
                Err(error) => {
                    if cancelling || snapshot.cancel_requested {
                        return Ok(RunningOutcome::Cancelled);
                    }
                    return Err(io::Error::other(error));
                }
            },
            Err(mpsc::TryRecvError::Disconnected) => {
return Err(io::Error::other("Scrape thread disconnected unexpectedly."))
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }

        if event::poll(Duration::from_millis(120))? {
            match event::read()? {
                Event::Resize(_, _) => {}
                Event::Key(key) => match key.code {
                    KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('b') => {
                        progress::request_cancel();
                        cancelling = true;
                    }
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        progress::request_cancel();
                        cancelling = true;
                    }
                    _ => {}
                },
                Event::Mouse(mouse) => {
                    if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left))
                        && mouse.row >= terminal::size()?.1.saturating_sub(1)
                    {
                        progress::request_cancel();
                        cancelling = true;
                    }
                }
                _ => {}
            }
        }
    }
}

fn results_loop(
    stdout: &mut Stdout,
    output: &RunOutput,
    stats: ProgressSnapshot,
) -> io::Result<bool> {
    let mut view = ResultsView::from_output(output, stats);

    loop {
        let height = terminal::size()?.1;
        let visible_rows = pane_content_rows(results_pane_height(height));
        let visible_width = pane_content_width(results_divider_x(terminal::size()?.0).max(1));
        view.normalize_scrolls(visible_rows);
        view.normalize_x_scrolls(visible_width);
        draw_results(stdout, output, &view)?;
        match event::read()? {
            Event::Key(key) => {
                if view.edit_mode == ResultsEditMode::Search {
                    match key.code {
                        KeyCode::Esc => view.edit_mode = ResultsEditMode::Normal,
                        KeyCode::Enter => {
                            view.edit_mode = ResultsEditMode::Normal;
                            view.normalize_scrolls(visible_rows);
                        }
                        KeyCode::Backspace => {
                            view.search_query.pop();
                            view.normalize_scrolls(visible_rows);
                        }
                        KeyCode::Char(c) if !c.is_control() => {
                            view.search_query.push(c);
                            view.normalize_scrolls(visible_rows);
                        }
                        _ => {}
                    }
                    continue;
                }

                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc | KeyCode::Char('b') => return Ok(true),
                    KeyCode::Left | KeyCode::Right if key.modifiers.contains(KeyModifiers::SHIFT) => {
                        let delta = if key.code == KeyCode::Left { -6 } else { 6 };
                        view.scroll_active_x_by(delta, visible_width);
                    }
                    KeyCode::Left | KeyCode::Right => view.switch_active(),
                    KeyCode::Up => view.scroll_active_by(-1, visible_rows),
                    KeyCode::Down => view.scroll_active_by(1, visible_rows),
                    KeyCode::PageUp => view.scroll_active_by(-10, visible_rows),
                    KeyCode::PageDown => view.scroll_active_by(10, visible_rows),
                    KeyCode::Home => view.set_active_scroll(0, visible_rows),
                    KeyCode::End => view.set_active_scroll(usize::MAX / 2, visible_rows),
                    KeyCode::Char('c') => view.cycle_conference_filter(),
                    KeyCode::Char('/') => view.edit_mode = ResultsEditMode::Search,
                    KeyCode::Char('r') => view.clear_filters(),
                    _ => {}
                }
            }
            Event::Mouse(mouse) => {
                let (width, height) = terminal::size()?;
                view.handle_mouse(mouse, width, height);
            }
            Event::Resize(_, _) => {}
            _ => {}
        }
    }
}

fn draw(stdout: &mut Stdout, app: &App) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    queue!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    draw_line(
        stdout,
        0,
        width,
        "UIL Scraper TUI",
        Some(Color::Green),
        Some(Attribute::Bold),
    )?;
    draw_line(
        stdout,
        1,
        width,
        "Arrow keys move fields. ←/→ changes values. Horizontal trackpad scroll or Shift+wheel also changes the focused value. Click options on the right to apply them.",
        Some(Color::DarkGrey),
        None,
    )?;

    let (left_width, right_x, right_width) = form_layout(width);
    let fields = app.visible_fields();
    let form_top = form_top();
    let visible_rows = form_visible_rows(height);

    draw_region_line(
        stdout,
        0,
        form_top.saturating_sub(1),
        left_width,
        " Form fields ",
        Some(Color::Black),
        Some(Color::Cyan),
        true,
    )?;

    for row in 0..visible_rows {
        let index = app.form_scroll + row;
        let y = form_top + row as u16;
        if let Some(field) = fields.get(index) {
            let active = index == app.focus;
            let label_width = 20usize.min(left_width as usize);
            let value_width = left_width.saturating_sub(label_width as u16 + 3) as usize;
            let label = pad_to_width(&field.label, label_width);
            let value = pad_to_width(&field.value, value_width);
            let fg = if active { Color::Black } else { Color::White };
            let bg = if active { Color::Yellow } else { Color::DarkGrey };
            let row_text = format!(" {} {}", label, value);
            draw_region_line(
                stdout,
                0,
                y,
                left_width,
                &row_text,
                Some(fg),
                if active { Some(bg) } else { None },
                active,
            )?;
        } else {
            draw_region_line(stdout, 0, y, left_width, "", None, None, false)?;
        }
    }

    if right_width > 0 {
        draw_region_line(
            stdout,
            right_x,
            form_top.saturating_sub(1),
            right_width,
            " Options / help ",
            Some(Color::Black),
            Some(Color::Blue),
            true,
        )?;

        let selected = fields.get(app.focus);
        let selected_label = selected.map(|f| f.label.as_str()).unwrap_or("Mode");
        let selected_value = selected.map(|f| f.value.as_str()).unwrap_or("");
        let help_lines = wrap_text(selected.map(|f| f.help.as_str()).unwrap_or(""), right_width.saturating_sub(2) as usize);
        draw_region_line(
            stdout,
            right_x,
            form_top,
            right_width,
            &format!("Selected: {selected_label}"),
            Some(Color::Cyan),
            None,
            true,
        )?;
        draw_region_line_offset(
            stdout,
            right_x,
            form_top + 1,
            right_width,
            &format!("Current: {selected_value}"),
            Some(Color::White),
            None,
            false,
            app.sidebar_x_scroll,
        )?;
        draw_region_line_offset(
            stdout,
            right_x,
            form_top + 2,
            right_width,
            help_lines.get(0).map(String::as_str).unwrap_or(""),
            Some(Color::Grey),
            None,
            false,
            app.sidebar_x_scroll,
        )?;
        draw_region_line_offset(
            stdout,
            right_x,
            form_top + 3,
            right_width,
            help_lines.get(1).map(String::as_str).unwrap_or("Wheel scrolls this list. Horizontal trackpad scroll changes the focused value. Click an option to apply it."),
            Some(Color::Grey),
            None,
            false,
            app.sidebar_x_scroll,
        )?;
        draw_region_line(
            stdout,
            right_x,
            form_top + 4,
            right_width,
            "Options:",
            Some(Color::Magenta),
            None,
            true,
        )?;

        let option_top = sidebar_options_top();
        let option_rows = sidebar_visible_rows(height);
        let items = app.sidebar_items();
        let max_scroll = items.len().saturating_sub(option_rows);
        let scroll = app.sidebar_scroll.min(max_scroll);

        for row in 0..option_rows {
            let y = option_top + row as u16;
            if let Some(item) = items.get(scroll + row) {
                let selected_option = match &item.kind {
                    SidebarItemKind::Option(action) => sidebar_action_selected(app, action),
                    _ => false,
                };
                let (fg, bg, bold) = match &item.kind {
                    SidebarItemKind::Heading => (Color::Yellow, None, true),
                    SidebarItemKind::Info => (Color::Grey, None, false),
                    SidebarItemKind::Option(_) if selected_option => (Color::Black, Some(Color::Green), true),
                    SidebarItemKind::Option(_) => (item.color, None, false),
                };
                let prefix = match &item.kind {
                    SidebarItemKind::Heading => "• ",
                    SidebarItemKind::Info => "  ",
                    SidebarItemKind::Option(_) if selected_option => "▶ ",
                    SidebarItemKind::Option(_) => "  ",
                };
                draw_region_line_offset(
                    stdout,
                    right_x,
                    y,
                    right_width,
                    &format!("{prefix}{}", item.text),
                    Some(fg),
                    bg,
                    bold || item.bold,
                    app.sidebar_x_scroll,
                )?;
            } else {
                draw_region_line(stdout, right_x, y, right_width, "", None, None, false)?;
            }
        }

        draw_region_line_offset(
            stdout,
            right_x,
            height.saturating_sub(3),
            right_width,
            &format!("Preview: {}", app.preview_command()),
            Some(Color::DarkGrey),
            None,
            false,
            app.sidebar_x_scroll,
        )?;
    }

    draw_line(
        stdout,
        height.saturating_sub(2),
        width,
        &format!("Status: {}", app.message),
        Some(Color::Yellow),
        None,
    )?;
    draw_line(
        stdout,
        height.saturating_sub(1),
        width,
        "Keys: ↑/↓ fields • ←/→ values • horizontal trackpad / Shift+wheel changes values • wheel scrolls lists • Esc quit",
        Some(Color::DarkGrey),
        None,
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_running(
    stdout: &mut Stdout,
    app: &App,
    snapshot: &ProgressSnapshot,
    elapsed: Duration,
) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    queue!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;
    draw_line(
        stdout,
        0,
        width,
        "UIL Scraper",
        Some(Color::Green),
        Some(Attribute::Bold),
    )?;
    draw_line(
        stdout,
        1,
        width,
        "Scraping in progress • timer starts when the scrape starts • this page updates live",
        Some(Color::Yellow),
        Some(Attribute::Bold),
    )?;
    draw_line(
        stdout,
        3,
        width,
        &format!("Mode: {} • Subject: {}", app.mode.label(), if app.mode == Mode::Rankings { "Rankings" } else { app.current_subject_label() }),
        Some(Color::White),
        None,
    )?;
    draw_line(
        stdout,
        4,
        width,
        &format!(
            "Progress: {}/{} attempted • {} with data • {} unavailable • elapsed {}",
            snapshot.completed_steps,
            snapshot.total_steps.max(snapshot.completed_steps),
            snapshot.successful_steps,
            snapshot.unavailable_steps,
            format_duration(elapsed)
        ),
        Some(Color::Cyan),
        Some(Attribute::Bold),
    )?;
    draw_progress_bar(stdout, 5, width, snapshot)?;
    draw_line(
        stdout,
        7,
        width,
        &format!("Current: {}", snapshot.current_label),
        Some(Color::Magenta),
        None,
    )?;
    draw_line(
        stdout,
        8,
        width,
        &format!("Last update: {}", snapshot.last_message),
        Some(Color::White),
        None,
    )?;
    draw_line(
        stdout,
        10,
        width,
        &format!(
            "Rows collected: {} individual • {} team",
            snapshot.individual_rows, snapshot.team_rows
        ),
        Some(Color::Yellow),
        None,
    )?;
    draw_line(
        stdout,
        11,
        width,
        &format!(
            "Availability: districts {} • regions {} • conferences {} • years {}{}",
            snapshot.districts_with_data.len(),
            snapshot.regions_with_data.len(),
            snapshot.conferences_with_data.len(),
            snapshot.years_with_data.len(),
            if snapshot.state_has_data { " • state yes" } else { "" }
        ),
        Some(Color::Blue),
        None,
    )?;

    let detail_y = 13u16;
    if detail_y < height.saturating_sub(4) {
        draw_line(
            stdout,
            detail_y,
            width,
            &format!("Districts with data: {}", join_numbers(&snapshot.districts_with_data, "none yet")),
            Some(Color::Grey),
            None,
        )?;
    }
    if detail_y + 1 < height.saturating_sub(3) {
        draw_line(
            stdout,
            detail_y + 1,
            width,
            &format!("Regions with data: {}", join_numbers(&snapshot.regions_with_data, "none yet")),
            Some(Color::Grey),
            None,
        )?;
    }
    draw_line(
        stdout,
        height.saturating_sub(1),
        width,
        if snapshot.cancel_requested {
            "Cancelling... waiting for in-flight requests to finish so the app can return to the menu."
        } else {
            "Press q, Esc, or b to cancel this scrape and return to the menu."
        },
        Some(Color::DarkGrey),
        None,
    )?;
    stdout.flush()?;
    Ok(())
}

fn draw_results(stdout: &mut Stdout, output: &RunOutput, view: &ResultsView) -> io::Result<()> {
    let (width, height) = terminal::size()?;
    queue!(stdout, cursor::MoveTo(0, 0), Clear(ClearType::All))?;

    let individuals = view.rendered_pane(ResultsPane::Individuals);
    let teams = view.rendered_pane(ResultsPane::Teams);

    draw_line(
        stdout,
        0,
        width,
        &output.title,
        Some(Color::Green),
        Some(Attribute::Bold),
    )?;
    draw_line(
        stdout,
        1,
        width,
        &format!(
            "Elapsed: {} • Active pane: {} • Conference filter: {} • Search: {}",
            format_duration(output.elapsed),
            view.active.label(),
            view.conference_filter_label(),
            if view.search_query.trim().is_empty() { "<none>" } else { view.search_query.trim() }
        ),
        Some(Color::Yellow),
        Some(Attribute::Bold),
    )?;
    draw_line(
        stdout,
        2,
        width,
        &format!(
            "Fetch stats: {} / {} postings had data • districts {} • regions {} • conferences {}",
            view.stats.successful_steps,
            view.stats.total_steps.max(view.stats.completed_steps),
            view.stats.districts_with_data.len(),
            view.stats.regions_with_data.len(),
            view.stats.conferences_with_data.len(),
        ),
        Some(Color::Magenta),
        None,
    )?;
    draw_line(
        stdout,
        3,
        width,
        if view.edit_mode == ResultsEditMode::Search {
            "Search mode: type to filter competitor or school names, Enter/Esc to finish. c cycles conferences. r resets filters."
        } else {
            "Wheel scrolls vertically • horizontal trackpad or Shift+wheel switches panes • click a pane to focus • ←/→ switches panes • / search • c conference • r reset • b/q/Esc menu"
        },
        Some(Color::DarkGrey),
        None,
    )?;

    let pane_top = results_pane_top();
    let pane_height = results_pane_height(height);
    let divider_x = results_divider_x(width);
    let left_width = divider_x.max(1);
    let right_x = divider_x.saturating_add(1);
    let right_width = width.saturating_sub(right_x).max(1);

    if pane_height > 0 {
        draw_pane(
            stdout,
            0,
            pane_top,
            left_width,
            pane_height,
            &individuals,
            view.pane_scroll(ResultsPane::Individuals),
            view.pane_x_scroll(ResultsPane::Individuals),
            view.active == ResultsPane::Individuals,
            Color::Cyan,
            view.filters_active(),
        )?;

        if divider_x < width {
            for row in pane_top..pane_top.saturating_add(pane_height) {
                queue!(
                    stdout,
                    cursor::MoveTo(divider_x, row),
                    SetForegroundColor(Color::DarkGrey),
                    Print("│"),
                    ResetColor
                )?;
            }
        }

        draw_pane(
            stdout,
            right_x,
            pane_top,
            right_width,
            pane_height,
            &teams,
            view.pane_scroll(ResultsPane::Teams),
            view.pane_x_scroll(ResultsPane::Teams),
            view.active == ResultsPane::Teams,
            Color::Blue,
            view.filters_active(),
        )?;
    }

    draw_line(
        stdout,
        height.saturating_sub(2),
        width,
        &format!(
            "Individuals: line {} of {} • matches {} • Teams: line {} of {} • matches {}",
            line_counter(view.pane_scroll(ResultsPane::Individuals), &individuals),
            individuals.lines.len().max(1),
            individuals.match_count,
            line_counter(view.pane_scroll(ResultsPane::Teams), &teams),
            teams.lines.len().max(1),
            teams.match_count,
        ),
        Some(Color::Cyan),
        None,
    )?;
    draw_line(
        stdout,
        height.saturating_sub(1),
        width,
        if view.filters_active() {
            "When filters are active, each result row shows filtered place on the left and original place in the row."
        } else {
            "Tip: use / to search names, c to isolate a conference, wheel for vertical scroll, and horizontal trackpad / Shift+wheel to switch panes."
        },
        Some(Color::Magenta),
        None,
    )?;

    stdout.flush()?;
    Ok(())
}

fn draw_pane(
    stdout: &mut Stdout,
    x: u16,
    top: u16,
    width: u16,
    height: u16,
    pane: &RenderedPane,
    scroll: usize,
    x_scroll: usize,
    active: bool,
    accent: Color,
    filters_active: bool,
) -> io::Result<()> {
    if width == 0 || height == 0 {
        return Ok(());
    }

    let title_fg = if active { Color::Black } else { Color::White };
    let title_bg = if active { accent } else { Color::DarkGrey };
    let border_color = if active { accent } else { Color::DarkGrey };

    draw_region_line(
        stdout,
        x,
        top,
        width,
        &format!(" {}{} ", pane.title, if filters_active { " (filtered)" } else { "" }),
        Some(title_fg),
        Some(title_bg),
        true,
    )?;

    if height == 1 {
        return Ok(());
    }

    draw_region_line(
        stdout,
        x,
        top + 1,
        width,
        &"─".repeat(width as usize),
        Some(border_color),
        None,
        false,
    )?;

    if height == 2 {
        return Ok(());
    }

    let content_rows = pane_content_rows(height);
    let max_scroll = pane.lines.len().saturating_sub(content_rows);
    let scroll = scroll.min(max_scroll);

    for row_offset in 0..content_rows {
        let row = top + 2 + row_offset as u16;
        if let Some(line) = pane.lines.get(scroll + row_offset) {
            draw_region_line_offset(
                stdout,
                x,
                row,
                width,
                &line.text,
                Some(line.color),
                None,
                line.bold,
                x_scroll,
            )?;
        } else {
            draw_region_line(stdout, x, row, width, "", None, None, false)?;
        }
    }

    let visible_end = if pane.lines.is_empty() {
        0
    } else {
        (scroll + content_rows).min(pane.lines.len())
    };
    let footer = if pane.lines.is_empty() {
        String::from(" No results ")
    } else {
        format!(" Showing {}-{} of {} ", scroll + 1, visible_end, pane.lines.len())
    };
    draw_region_line(
        stdout,
        x,
        top + height - 1,
        width,
        &footer,
        Some(border_color),
        None,
        active,
    )?;

    Ok(())
}

fn draw_progress_bar(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    snapshot: &ProgressSnapshot,
) -> io::Result<()> {
    let usable = width.saturating_sub(10) as usize;
    let total = snapshot.total_steps.max(snapshot.completed_steps).max(1);
    let filled = ((snapshot.completed_steps as f32 / total as f32) * usable as f32).round() as usize;
    let filled = filled.min(usable);
    let bar = format!(
        "[{}{}] {:>3}%",
        "█".repeat(filled),
        "░".repeat(usable.saturating_sub(filled)),
        ((snapshot.completed_steps as f32 / total as f32) * 100.0).round() as usize
    );
    draw_line(stdout, row, width, &bar, Some(Color::Green), None)
}

fn snapshot_copy(shared: &Arc<Mutex<ProgressSnapshot>>) -> ProgressSnapshot {
    shared.lock().map(|snapshot| snapshot.clone()).unwrap_or_default()
}

fn display_text_value(value: &str, show_blank_hint: bool) -> String {

    let trimmed = value.trim();
    if trimmed.is_empty() {
        if show_blank_hint {
            String::from("<blank>")
        } else {
            String::from("<optional>")
        }
    } else {
        trimmed.to_string()
    }
}

fn bool_label(value: bool) -> String {
    if value {
        String::from("Yes")
    } else {
        String::from("No")
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "ON" } else { "OFF" }
}

fn is_conference_char(c: char) -> bool {
    c.is_ascii_digit() || matches!(c, 'a' | 'A' | ',' | '-' | ' ')
}

fn string_or_none(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn required_text(value: &str, label: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Err(format!("Please enter {label}."))
    } else {
        Ok(trimmed.to_string())
    }
}

fn parse_optional_u16(value: &str, label: &str) -> Result<Option<u16>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<u16>()
            .map(Some)
            .map_err(|_| format!("Invalid {label}."))
    }
}

fn parse_optional_usize(value: &str, label: &str) -> Result<Option<usize>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else {
        trimmed
            .parse::<usize>()
            .map(Some)
            .map_err(|_| format!("Invalid {label}."))
    }
}

fn parse_bounded_u8(value: &str, label: &str, min: u8, max: u8) -> Result<Option<u8>, String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed = trimmed
        .parse::<u8>()
        .map_err(|_| format!("Invalid {label}."))?;
    if parsed < min || parsed > max {
        return Err(format!("{label} must be between {min} and {max}."));
    }
    Ok(Some(parsed))
}

fn normalize_conference(value: &str) -> String {
    let mut normalized = String::new();
    let mut previous_separator = false;
    for ch in value.chars() {
        if ch.is_ascii_digit() {
            normalized.push(ch);
            previous_separator = false;
        } else if ch == ',' || ch == '-' {
            if !normalized.is_empty() && !previous_separator {
                normalized.push(ch);
                previous_separator = true;
            }
        }
    }
    normalized.trim_matches(&[',', '-'][..]).to_string()
}

fn validate_conference(value: &str) -> Result<String, String> {
    let normalized = normalize_conference(value);
    if normalized.is_empty() {
        return Err(String::from("Please enter a conference value."));
    }
    if RequestFields::parse_range(normalized.clone()).is_none() {
        return Err(String::from("Invalid conference value."));
    }
    Ok(normalized)
}

fn build_preview_from_cli(cli: &Cli) -> String {
    let mut parts = vec![String::from("cargo run --"), shell_quote(&cli.subject)];

    if let Some(conference) = &cli.conference {
        parts.push(String::from("--conference"));
        parts.push(shell_quote(conference));
    }
    if let Some(district) = cli.district {
        parts.push(String::from("--district"));
        parts.push(district.to_string());
    }
    if let Some(region) = cli.region {
        parts.push(String::from("--region"));
        parts.push(region.to_string());
    }
    if cli.state {
        parts.push(String::from("--state"));
    }
    if let Some(year) = cli.year {
        parts.push(String::from("--year"));
        parts.push(year.to_string());
    }
    if let Some(find) = &cli.find {
        parts.push(String::from("--find"));
        parts.push(shell_quote(find));
    }
    if let Some(count) = cli.individual_positions {
        parts.push(String::from("--individual-positions"));
        parts.push(count.to_string());
    }
    if let Some(count) = cli.team_positions {
        parts.push(String::from("--team-positions"));
        parts.push(count.to_string());
    }
    if cli.mute {
        parts.push(String::from("--mute"));
    }
    if cli.highscores {
        parts.push(String::from("--highscores"));
    }

    if let Some(command) = &cli.command {
        match command {
            Commands::Compare {
                person_a,
                person_b,
                conferences,
                district,
                region,
                state,
            } => {
                parts.push(String::from("compare"));
                parts.push(shell_quote(person_a));
                parts.push(shell_quote(person_b));
                parts.push(String::from("--conferences"));
                parts.push(shell_quote(conferences));
                if *district {
                    parts.push(String::from("--district"));
                }
                if *region {
                    parts.push(String::from("--region"));
                }
                if *state {
                    parts.push(String::from("--state"));
                }
            }
        }
    }

    parts.join(" ")
}

fn shell_quote(value: &str) -> String {
    if value.contains(' ') || value.contains(',') || value.contains('-') {
        format!("\"{}\"", value.replace('"', "\\\""))
    } else {
        value.to_string()
    }
}

fn split_output_sections(sections: &[OutputSection]) -> (Vec<OutputSection>, Vec<OutputSection>) {
    let mut individuals = Vec::new();
    let mut teams = Vec::new();
    for section in sections {
        if section.title.to_lowercase().contains("team") {
            teams.push(section.clone());
        } else {
            individuals.push(section.clone());
        }
    }
    (individuals, teams)
}

fn build_result_pane(title: &str, sections: &[OutputSection], _accent: Color) -> ResultPaneData {
    let mut pane_sections = Vec::new();
    if sections.is_empty() {
        pane_sections.push(PaneSection {
            title: String::from("No results"),
            lines: vec![StyledLine {
                text: String::from("No results."),
                color: Color::DarkGrey,
                bold: false,
            }],
        });
    } else {
        for section in sections {
            pane_sections.push(PaneSection {
                title: section.title.clone(),
                lines: section.lines.iter().map(|line| style_result_line(line)).collect(),
            });
        }
    }
    ResultPaneData {
        title: title.to_string(),
        sections: pane_sections,
    }
}

fn line_matches_filters(line: &str, conference_filter: Option<u8>, search_query: &str) -> bool {
    if let Some(conference) = conference_filter {
        if extract_line_conference(line) != Some(conference) {
            return false;
        }
    }
    if !search_query.is_empty() {
        let lowered = line.to_ascii_lowercase();
        if !lowered.contains(search_query) {
            return false;
        }
    }
    true
}

fn extract_line_conference(line: &str) -> Option<u8> {
    let start = line.find('[')?;
    let end = line[start..].find(']')? + start;
    let inner = &line[start + 1..end];
    inner.trim_end_matches('A').parse::<u8>().ok()
}

fn extract_place(line: &str) -> Option<usize> {
    let trimmed = line.trim_start();
    let digits: String = trimmed.chars().take_while(|c| c.is_ascii_digit()).collect();
    if digits.is_empty() {
        return None;
    }
    let after = trimmed.chars().nth(digits.len())?;
    if after != '.' {
        return None;
    }
    digits.parse::<usize>().ok()
}

fn join_numbers<T>(values: &std::collections::BTreeSet<T>, empty: &str) -> String
where
    T: std::fmt::Display + Ord,
{
    if values.is_empty() {
        empty.to_string()
    } else {
        values.iter().map(|value| value.to_string()).collect::<Vec<_>>().join(", ")
    }
}

fn subject_options_alphabetical() -> Vec<(&'static str, &'static str)> {
    let mut options = SUBJECT_OPTIONS.to_vec();
    options.sort_by(|a, b| a.1.cmp(b.1));
    options
}

fn style_result_line(line: &str) -> StyledLine {

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return StyledLine {
            text: String::new(),
            color: Color::White,
            bold: false,
        };
    }
    if trimmed.starts_with("1.") || trimmed.starts_with("1 .") || trimmed.starts_with("1 ") || trimmed.starts_with("1\t") {
        return StyledLine {
            text: line.to_string(),
            color: Color::Green,
            bold: true,
        };
    }
    if trimmed.starts_with("2.") || trimmed.starts_with("3.") {
        return StyledLine {
            text: line.to_string(),
            color: Color::Yellow,
            bold: true,
        };
    }
    if line.contains("[Advanced]") || line.contains("[Indv]") || line.contains("[Team]") {
        return StyledLine {
            text: line.to_string(),
            color: Color::Cyan,
            bold: false,
        };
    }
    if line.contains("[Wildcard]") || line.contains("[Wild]") {
        return StyledLine {
            text: line.to_string(),
            color: Color::Magenta,
            bold: false,
        };
    }
    if line.to_lowercase().contains("no matching") || line.to_lowercase().contains("no results") {
        return StyledLine {
            text: line.to_string(),
            color: Color::DarkGrey,
            bold: false,
        };
    }
    StyledLine {
        text: line.to_string(),
        color: Color::White,
        bold: false,
    }
}

fn draw_line(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    text: &str,
    fg: Option<Color>,
    attr: Option<Attribute>,
) -> io::Result<()> {
    draw_region_line(stdout, 0, row, width, text, fg, None, attr == Some(Attribute::Bold))
}

fn draw_region_line(
    stdout: &mut Stdout,
    x: u16,
    row: u16,
    width: u16,
    text: &str,
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
) -> io::Result<()> {
    draw_region_line_offset(stdout, x, row, width, text, fg, bg, bold, 0)
}

fn draw_region_line_offset(
    stdout: &mut Stdout,
    x: u16,
    row: u16,
    width: u16,
    text: &str,
    fg: Option<Color>,
    bg: Option<Color>,
    bold: bool,
    offset: usize,
) -> io::Result<()> {
    queue!(stdout, cursor::MoveTo(x, row))?;
    if let Some(color) = fg {
        queue!(stdout, SetForegroundColor(color))?;
    }
    if let Some(color) = bg {
        queue!(stdout, SetBackgroundColor(color))?;
    }
    if bold {
        queue!(stdout, SetAttribute(Attribute::Bold))?;
    }
    queue!(stdout, Print(slice_and_pad(text, width as usize, offset)))?;
    queue!(stdout, ResetColor, SetAttribute(Attribute::Reset))?;
    Ok(())
}

fn pad_to_width(text: &str, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut output: String = text.chars().take(width).collect();
    let len = output.chars().count();
    if len < width {
        output.push_str(&" ".repeat(width - len));
    }
    output
}

fn slice_and_pad(text: &str, width: usize, offset: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let mut output: String = text.chars().skip(offset).take(width).collect();
    let len = output.chars().count();
    if len < width {
        output.push_str(&" ".repeat(width - len));
    }
    output
}

fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return vec![String::new()];
    }
    let mut out = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let pending = if current.is_empty() {
            word.len()
        } else {
            current.len() + 1 + word.len()
        };
        if pending > width && !current.is_empty() {
            out.push(current);
            current = word.to_string();
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        out.push(current);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}

fn form_top() -> u16 {
    3
}

fn form_layout(width: u16) -> (u16, u16, u16) {
    let left_width = ((width as usize * 58) / 100) as u16;
    let left_width = left_width.max(40).min(width.saturating_sub(20));
    let right_x = left_width.saturating_add(1).min(width);
    let right_width = width.saturating_sub(right_x);
    (left_width, right_x, right_width)
}

fn form_visible_rows(height: u16) -> usize {
    height.saturating_sub(6) as usize
}

fn sidebar_options_top() -> u16 {
    form_top() + 5
}

fn sidebar_visible_rows(height: u16) -> usize {
    height.saturating_sub(sidebar_options_top()).saturating_sub(3) as usize
}

fn results_pane_top() -> u16 {
    4
}

fn results_pane_height(height: u16) -> u16 {
    height.saturating_sub(7)
}

fn results_divider_x(width: u16) -> u16 {
    width / 2
}

fn pane_content_rows(pane_height: u16) -> usize {
    pane_height.saturating_sub(3) as usize
}

fn pane_content_width(width: u16) -> usize {
    width.saturating_sub(1) as usize
}

fn pane_from_column(column: u16, width: u16) -> ResultsPane {
    if column < results_divider_x(width) {
        ResultsPane::Individuals
    } else {
        ResultsPane::Teams
    }
}

fn line_counter(scroll: usize, pane: &RenderedPane) -> usize {
    if pane.lines.is_empty() {
        0
    } else {
        scroll.min(pane.lines.len() - 1) + 1
    }
}

fn format_duration(duration: std::time::Duration) -> String {
    let total_seconds = duration.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let millis = duration.subsec_millis();
    if minutes > 0 {
        format!("{}m {:02}.{:03}s", minutes, seconds, millis)
    } else {
        format!("{}.{:03}s", seconds, millis)
    }
}
