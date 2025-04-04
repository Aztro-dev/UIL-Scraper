A web scraper for the most of the Academic UIL events, including:

- Accounting
- Computer Applications
- Social Studies
- Spelling
- Calculator
- Computer Science
- Mathematics
- Number Sense
- Science
- Sweepstakes (overall)

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

- If you want to use this command whenever you open up your terminal, I would recommend adding the produced binary to your path

```sh
# Linux/MacOS:
export PATH="$PATH:/path/to/UIL-Scraper/target/build"
# Windows:
Lmao google it
```

- I would also recommend moving the binary to another folder in case you want to delete the `UIL-Scraper` folder but not the binary

```sh
# Linux/MacOS:
mv /path/to/UIL-Scraper/target/build/uil_scraper /path/to/folder
# Windows:
move /path/to/UIL-Scraper/target/build/uil_scraper /path/to/folder
```

# Supported commands/options

- SUBJECT:

  - You must specify a subject that is from the subject list, but in a certain manner. Here are the supported strings you can pass in:

  ```rust
  "accounting" => Accounting,
  "current_events" => Current Events,
  "comp_sci" or "cs" => Computer Science,
  "calculator" or "calc" => Calculator,
  "spelling" or "spell" => Spelling,
  "social_studies" => Social Studies,
  "mathematics" or "math" => Mathematics,
  "number_sense" or "ns" => Number Sense,
  "science" or "sci" => Science),
  "sweepstakes" or "overall" => Sweepstakes,
  ```

- LEVEL:

  - While this technically isn't a single variable, it is still required if you want results

  - district:

    - Included by passing in `--district <district>`
    - Put any number 1-32 for a specific district, or 0 for all districts
    - Example:

    ```sh
    uil_scraper mathematics --district 11
    ```

  - region:

    - Included by passing in `--region <region>`
    - Put any number 1-4 for a specific region, or 0 for all regions
    - Example:

    ```sh
    uil_scraper mathematics --region 2
    ```

  - state:

    - Included by passing in `--state <state>`
    - You can put any number to check the state results.
    - Example:

    ```sh
    uil_scraper mathematics --state 0
    ```

- CONFERENCE (optional):

  - Included by passing in `--conference <conference>`
  - The `conference` can be in the form of a single number (`4`), a number and letter (`4a`) or number and uppercase letter (`4A`)
  - The `conference` can also be a range in the form of `<num>A-<num>A`, `<num>-<num>`, `<num><num>`, etc.
    - Basically it just needs two numbers both in the range 1-6
  - Example:

  ```sh
  uil_scraper mathematics --district 0 --conference 1-4
  ```

- YEAR (optional):

  - Included by passing in `--year <year>`
  - Only supported values are 2023-2025, because this utility scrapes SpeechWire specifically.
  - The default value is 2025, which would be the year if you didn't include this field at all.
  - Example:

  ```sh
  uil_scraper mathematics --state 0 --year 2024
  ```
