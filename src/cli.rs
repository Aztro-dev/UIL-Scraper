use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Subject to find
    pub subject: String,

    /// Find a specific conference, a range of conferences (ex. 3-5), or ignore for all conferences
    #[arg(short, long, value_name = "CONFERENCE")]
    pub conference: Option<String>,

    /// Find a specific district, or blank/0 for all districts
    #[arg(short, long, value_name = "DISTRICT", num_args = 0..=1, default_missing_value = "0")]
    pub district: Option<u8>,

    /// Find a specific region, or blank/0 for all regions
    #[arg(short, long, value_name = "REGION", num_args = 0..=1, default_missing_value = "0")]
    pub region: Option<u8>,

    /// Find the state results
    #[arg(short, long)]
    pub state: bool,

    /// Find a past/current year, or leave blank for the current year
    #[arg(short, long, value_name = "YEAR")]
    pub year: Option<u16>,

    /// Describes how many positions to show for the individual results
    /// Defaults to 25, with 0 being all teams
    #[arg(short, long, value_name = "INDIVIDUAL POSITIONS")]
    pub individual_positions: Option<usize>,

    /// Describes how many positions to show for the team results
    /// Defaults to 25, with 0 being all teams
    #[arg(short, long, value_name = "TEAM POSITIONS")]
    pub team_positions: Option<usize>,

    /// Mutes the district/region/state "completed" output lines
    #[arg(short, long)]
    pub mute: bool,
}
