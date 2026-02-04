use scraper::{Html, Selector};

#[derive(Debug)]
struct WordOfTheDay {
    pub word: String,
    pub meaning: String,
    pub etimo: String,
    pub examples: String,
}

impl WordOfTheDay {
    fn new(word: &str, meaning: &str, etimo: &str, examples: &str) -> Self {
        Self {
            word: String::from(word),
            meaning: String::from(meaning),
            etimo: String::from(etimo),
            examples: String::from(examples),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    
    let url = "https://unaparolaalgiorno.it/";

    let html = fetch_html(url).await?;
    let word = extract_data(&html);

    if let Ok(word) = word {
        let res = format!("{}\n\n{}\n{}\nes. {}", word.word, word.meaning, word.etimo, word.examples);
        println!("{res}");
    } else {
        eprintln!("unable to scrape data...");
    }

    Ok(())
}

async fn fetch_html(url: &str) -> Result<String, reqwest::Error> {
    reqwest::get(url)
        .await?
        .error_for_status()?
        .text()
        .await
}

fn extract_data(html: &str) -> Result<WordOfTheDay, Box<dyn std::error::Error>> {
    let document = Html::parse_document(html);

    let word_sel = Selector::parse("#home-todays > h2")?;
    let meaning_sel = Selector::parse(".word-significato.with-sign")?;
    let etimo_sel = Selector::parse(".word-etimo-home")?;
    let examples_sel = Selector::parse("#word-esempi > li > span")?;

    let word: String = document
        .select(&word_sel)
        .flat_map(|el| el.text())
        .collect();

    let meaning: String = document
        .select(&meaning_sel)
        .flat_map(|el| el.text())
        .collect();

    let etimo: String = document
        .select(&etimo_sel)
        .flat_map(|el| el.text())
        .collect();

    let examples: String = document
        .select(&examples_sel)
        .flat_map(|el| el.text())
        .collect();

    let word = word.trim();
    let meaning = meaning.trim();
    let etimo = etimo.trim();
    let examples = examples.trim();

    Ok(WordOfTheDay::new(word, meaning, etimo, examples))
}
