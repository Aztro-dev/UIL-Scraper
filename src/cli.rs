use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    /// Subject to find
    pub subject: String,

    /// Find a specific conference, or ignore for all conferences
    #[arg(short, long, value_name = "CONFERENCE")]
    pub conference: Option<String>,

    /// Find a specific district, or 0 for all districts
    #[arg(short, long, value_name = "DISTRICT")]
    pub district: Option<u8>,

    /// Find a specific region, or 0 for all regions
    #[arg(short, long, value_name = "REGION")]
    pub region: Option<u8>,

    /// Find the state results, can be any number
    #[arg(short, long, value_name = "STATE")]
    pub state: Option<u8>,

    /// Find a past/current year, or leave blank for the current year
    #[arg(short, long, value_name = "YEAR")]
    pub year: Option<u16>,
}
