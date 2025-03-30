use scraper::*;

mod request;
use request::*;

mod results;
use results::*;

fn main() {
    for district in 1..=1 {
        let request = request(UILFields {
            district: Some(district),
            region: None,
            state: None,
            conference: 4,
            subject: Subject::Mathematics,
            year: 2025,
        });
        if request.is_none() {
            println!("Could not find results from District {district}");
            continue;
        }

        let request = request.unwrap();

        let document = Html::parse_document(request.as_str());
        let table_selector = Selector::parse("table.ddprint").unwrap();
        let table = document
            .select(&table_selector)
            .next()
            .expect("Couldn't select table");
        let row_selector = Selector::parse("tr").unwrap();
        let cell_selector = Selector::parse("td").unwrap();

        let results: Vec<Result> = Vec::new();

        for row in table.select(&row_selector) {
            let cells: Vec<String> = row
                .select(&cell_selector)
                .map(|cell| cell.text().collect::<String>())
                .collect();

            let school = cells.[1];
        }
    }
}
