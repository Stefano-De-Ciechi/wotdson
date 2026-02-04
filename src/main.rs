use dotenv::dotenv;
use std::{collections::HashMap, env};
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

    // load credentials from .env file
    dotenv().ok();

    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN")?;
    let telegram_chat_ids = env::var("TELEGRAM_CHAT_IDS")?;
    let telegram_chat_ids: Vec<&str> = telegram_chat_ids.split(", ").collect();

    println!("{telegram_bot_token}");
    for id in telegram_chat_ids.clone() {
        println!("{id}");
    }

    let url = "https://unaparolaalgiorno.it/";

    let html = fetch_html(url).await?;
    let word = extract_data(&html);

    if let Ok(word) = word {
        let message = format!("{}\n\n{}\n\n{}\n\nes. {}", word.word, word.meaning, word.etimo, word.examples);
        send_telegram_message(&telegram_bot_token, telegram_chat_ids, &message).await?;
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

async fn send_telegram_message(bot_token: &str, chat_ids: Vec<&str>, message: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

    for id in chat_ids {

        let mut data = HashMap::new();
        data.insert("chat_id", id);
        data.insert("text", message);
        data.insert("parse_mode", "HTML");

        let res = client.post(&url)
            .json(&data)
            .send()
            .await?;

        println!("{:?}", res);
    }

    Ok(())
}
