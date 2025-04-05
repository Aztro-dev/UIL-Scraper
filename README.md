# uil_scraper

A web scraper for the most of the Academic UIL events, including:

- Accounting
- Calculator
- Computer Science
- Mathematics
- Number Sense
- Science
- Social Studies
- Spelling
- Sweepstakes (overall)

# Credits

This was originally inspired by a Python script written by Warith. While this Rust project builds off of the terminal-based UI concept, Warith's script was turned into a web design, which can be found [here](https://github.com/warithr621/uil-hub).

# Pre-requisites

- Rust 1.86.0+ (haven't checked other versions)
- Linux, Mac, or Windows machine

# Instructions/Installation

- Clone this repository by running

```sh
git clone https://github.com/Aztro-dev/UIL-Scraper.git
```

- Change into the directory and build

```sh
cd UIL-Scraper
cargo build
```

- If you want to use this command whenever you open up your terminal, I would recommend adding the produced binary to your PATH

```sh
# Linux/MacOS:
export PATH="$PATH:/path/to/UIL-Scraper/target/debug"
# Windows:
setx PATH "%PATH%;C:\path\to\UIL-Scraper\target\debug" # in CMD Prompt
```

- I would also recommend moving the binary to another folder in case you want to delete the `UIL-Scraper` folder but not the binary

```sh
# Linux/MacOS:
mv /path/to/UIL-Scraper/target/debug/uil_scraper /path/to/folder
# Windows:
move /path/to/UIL-Scraper/target/debug/uil_scraper /path/to/folder
```

You can now run the `uil_scraper` command! In case you did not add the binary into your PATH or have it in your current directory, `cd` into the produced `target/debug` to run it.

# Supported commands/options

## SUBJECT

You must specify a subject that is from the subject list, but in a certain manner. Here are the supported strings you can pass in:

  ```rust
  "accounting" => Accounting,
  "calculator" or "calc" => Calculator,
  "comp_sci" or "cs" => Computer Science,
  "current_events" => Current Events,
  "mathematics" or "math" => Mathematics,
  "number_sense" or "ns" => Number Sense,
  "science" or "sci" => Science,
  "social_studies" => Social Studies,
  "spelling" or "spell" => Spelling,
  "sweepstakes" or "overall" => Sweepstakes,
  ```

## LEVEL

While this technically isn't a single variable, it is still required if you want results

- district:

  - Included by passing in `--district <district>`
  - Put any number 1-32 for a specific district, or 0/leave blank for all districts
  - Examples:

    ```sh
    uil_scraper mathematics --district 11 # district 11 specifically
    uil_scraper mathematics --district 0  # all districts
    uil_scraper mathematics --district    # all districts
    ```

- region:

  - Included by passing in `--region <region>`
  - Put any number 1-4 for a specific region, or 0/leave blank for all regions
  - Examples:

    ```sh
    uil_scraper mathematics --region 2 # region 2 specifically
    uil_scraper mathematics --region 0 # all regions
    uil_scraper mathematics --region   # all regions 
    ```

- state:

  - Included by passing in `--state`
  - Example:

    ```sh
    uil_scraper mathematics --state
    ```

## CONFERENCE (optional):

  - Included by passing in `--conference <conference>`
  - The `conference` can be in the form of a single number (`4`), a number and letter (`4a`) or number and uppercase letter (`4A`)
  - The `conference` can also be a range in the form of `<num>A-<num>A`, `<num>-<num>`, `<num><num>`, etc.
    - Basically it just needs two numbers both in the range 1-6
  - By default, excluding this field will compile scores for all conferences with the given arguments (note putting `--conference` without any conference(s) will throw an error)
  - Examples:

  ```sh
  uil_scraper mathematics --district --conference 4    # District 4A results
  uil_scraper mathematics --district --conference 4A   # District 4A results
  uil_scraper mathematics --district --conference 4a   # District 4A results
  uil_scraper mathematics --district --conference 1-4  # District 1A to 4A results
  uil_scraper mathematics --district --conference 4-1  # District 1A to 4A results
  uil_scraper mathematics --district --conference 4A-1 # District 1A to 4A results
  uil_scraper mathematics --district --conference 41   # District 1A to 4A results
  ```

## YEAR (optional):

  - Included by passing in `--year <year>`
  - Only supported values are 2023-2025, because this utility scrapes SpeechWire specifically.
  - The default value is 2025, which would be the year if you didn't include this field at all.
  - Examples:

  ```sh
  uil_scraper mathematics --state --year 2024 # 2024 state results
  uil_scraper mathematics --state             # 2025 state results
  ```

## INDIVIDUAL POSITIONS (optional):

  - Included by passing in `--individual-positions <positions>` or `-i <positions>` for short
  - Defaults to 25, and changes the number of individual results that are shown
  - Example:

  ```sh
  uil_scraper mathematics --district --individual-positions 100 # show top 100 results
  ```

## TEAM POSITIONS (optional):

  - Included by passing in `--team-positions <positions>` or `-t <positions>` for short
  - Defaults to 25, and changes the number of team results that are shown
  - Example:

  ```sh
  uil_scraper mathematics --district --team-positions 100 # show top 100 results
  ```

## MUTE (optional):

  - Included by passing in `--mute`
  - Mutes the "completed" output lines when running so only the Individual and Team tables are displayed.
  - Example:

  ```sh
  uil_scraper mathematics --district --mute
  ```
