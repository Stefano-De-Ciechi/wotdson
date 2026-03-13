use chrono::Datelike;
use dotenv::dotenv;
use rusqlite::{Connection, params};
use std::{collections::HashMap, env};
use reqwest::header::{HeaderName, HeaderValue, ACCEPT, ACCEPT_LANGUAGE, ORIGIN, REFERER, USER_AGENT};
use serde::Deserialize;

#[derive(Debug)]
struct WordOfTheDay {
    pub word: String,
    pub syllables: String,
    pub meaning: String,
    pub etymology: String,
    pub examples: String,
    pub publish_date: String,
}

impl WordOfTheDay {
    fn new(word: &str, syllables: &str, meaning: &str, etymology: &str, examples: &str, publish_date: &str) -> Self {
        Self {
            word: word.to_string(),
            syllables: syllables.to_string(),
            meaning: meaning.to_string(),
            etymology: etymology.to_string(),
            examples: examples.to_string(),
            publish_date: publish_date.to_string(),
        }
    }
    
    fn to_message(&self) -> String {
        format!("{}\n\n{}\n{}\n\n{}\n\n{}\n\nes. {}", self.publish_date, self.word, self.syllables, self.meaning, self.etymology, self.examples)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    // get today date in format yyyy-mm-dd (the date is the primary key in the words history db)
    let today = chrono::Utc::now();
    let today = format!("{}-{:0>2}-{:0>2}", today.year(), today.month(), today.day());
    println!("today is: {today}");

    print!("searching today record... ");
    // query the db; if a record with publish_date == today, exit the program
    if today_has_record(&today) {
        println!("FOUND");
        println!("stop procedure\n");
        return Ok(());
    }

    println!("NOT FOUND");
 
    // load credentials from .env file
    dotenv().ok();

    let telegram_bot_token = env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN not set");
    let telegram_chat_ids = env::var("TELEGRAM_CHAT_IDS").expect("TELEGRAM_CHAT_IDS not set");
    let telegram_chat_ids: Vec<&str> = telegram_chat_ids.split(", ").collect();

    print!("fetching data... ");
    let word = fetch_data().await;
    println!("DONE");

    if let Ok(word) = word {
        print!("storing today record... ");
        let stored = store_today_record(&word);
        if stored {
            println!("DONE");

            let message = word.to_message();
            // TODO should check if message was sended or not... if not there should be some logic
            // to retry? or just delete the record from DB? (it's the simpler solution)
            print!("sending telegram message... ");
            send_telegram_message(&telegram_bot_token, telegram_chat_ids, &message).await?;
            println!("DONE");
        }
        else {
            eprintln!("error, the record was not stored, abort sending message...");
        }
    } else {
        eprintln!("unable to scrape data...");
    }

    println!("finished\n");
    Ok(())
}

fn today_has_record(today: &str) -> bool {
    let db_path = "./words_history.db";

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("could not open sqlite connection to word_history.db...");
            return false;
        }
    };

    let mut stmt = match conn.prepare("SELECT * FROM history WHERE publish_date = :today;") {
        Ok(s) => s,
        Err(_) => {
            eprintln!("error while preparing sql SELECT statement...");
            return false;
        }
    };

    let res_iter = stmt.query_map(&[(":today", today)], |row| {
        Ok(
            WordOfTheDay {
                publish_date: row.get(0)?,
                word: row.get(1)?,
                syllables: row.get(2)?,
                meaning: row.get(3)?,
                etymology: row.get(4)?,
                examples: row.get(5)?,
            }
        )
    });

    match res_iter {
        Ok(res) => res.count() >= 1,
        Err(_) => {
            eprintln!("error with mapping SELECT result...");
            false
        }
    }

}

fn store_today_record(record: &WordOfTheDay) -> bool {

    let db_path = "./words_history.db";

    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => {
            eprintln!("could not open sqlite connection to word_history.db...");
            return false;
        }
    };

    let res = conn.execute(
        "INSERT INTO history (publish_date, word, syllables, meaning, etymology, examples) VALUES (?1, ?2, ?3, ?4, ?5, ?6);",
        params![record.publish_date, record.word, record.syllables, record.meaning, record.etymology, record.examples]
    );

    match res {
        Ok(rows) => rows == 1,
        Err(e) => {
            eprintln!("error during sql INSERT of today record...");
            eprintln!("{:?}", e);
            false
        }
    }
}

#[derive(Deserialize, Debug)]
struct TodayApiResponse {
    //#[serde(rename(deserialize = "data_pubblicazione"))]
    //publish_date: String,

    #[serde(rename(deserialize = "url_parola"))]
    word_url: String,
}

#[derive(Deserialize, Debug)]
struct WordApiResponse {

    #[serde(rename(deserialize = "data_pubblicazione"))]
    publish_date: String,

    #[serde(rename(deserialize = "esempi"))]
    examples: String,

    #[serde(rename(deserialize = "etimo"))]
    etymology: String,

    #[serde(rename(deserialize = "parola"))]
    word: String,

    #[serde(rename(deserialize = "significato"))]
    meaning: String,

    //#[serde(rename(deserialize = "preview"))]
    //preview: String,

    #[serde(rename(deserialize = "sillabe"))]
    syllables: String,
}

async fn fetch_data() -> Result<WordOfTheDay, reqwest::Error> {
    let url = "https://v3.unaparolaalgiorno.it/api/words/home";

    // ===== home request =====
    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("it-IT,it;q=0.5"));
    headers.insert(ORIGIN, HeaderValue::from_static("https://unaparolaalgiorno.it"));
    headers.insert(HeaderName::from_static("priority"), HeaderValue::from_static("u=1, i"));
    headers.insert(REFERER, HeaderValue::from_static("https://unaparolaalgiorno.it/"));
    headers.insert(HeaderName::from_static("sec-ch-ua"), HeaderValue::from_static("Not(A:Brand\";v=\"8\", \"Chromium\";v=\"144\", \"Brave\";v=\"144\")"));
    headers.insert(HeaderName::from_static("sec-ch-ua-mobile"), HeaderValue::from_static("?0"));
    headers.insert(HeaderName::from_static("sec-ch-ua-platform"), HeaderValue::from_static("Linux"));
    headers.insert(HeaderName::from_static("sec-fetch-dest"), HeaderValue::from_static("empty"));
    headers.insert(HeaderName::from_static("sec-fetch-mode"), HeaderValue::from_static("cors"));
    headers.insert(HeaderName::from_static("sec-fetch-site"), HeaderValue::from_static("same-site"));
    headers.insert(HeaderName::from_static("sec-gpc"), HeaderValue::from_static("1"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36"));

    let client = reqwest::Client::new();
    let json = client
        .get(url)
        .headers(headers)
        .send()
        .await?
        .text()
        .await?;
    
    let home_data = parse_wrapper(&json);
    //println!("{:?}", home_data);
    
    // ===== word data =====
    let url = &format!("https://v3.unaparolaalgiorno.it/api/words/view/{}", home_data.word_url);
    let mut headers = reqwest::header::HeaderMap::new();

    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("it-IT,it;q=0.5"));
    headers.insert(ORIGIN, HeaderValue::from_static("https://unaparolaalgiorno.it"));
    headers.insert(HeaderName::from_static("priority"), HeaderValue::from_static("u=1, i"));
    headers.insert(REFERER, HeaderValue::from_static("https://unaparolaalgiorno.it/"));
    headers.insert(HeaderName::from_static("sec-ch-ua"), HeaderValue::from_static("Not(A:Brand\";v=\"8\", \"Chromium\";v=\"144\", \"Brave\";v=\"144\")"));
    headers.insert(HeaderName::from_static("sec-ch-ua-mobile"), HeaderValue::from_static("?0"));
    headers.insert(HeaderName::from_static("sec-ch-ua-platform"), HeaderValue::from_static("Linux"));
    headers.insert(HeaderName::from_static("sec-fetch-dest"), HeaderValue::from_static("empty"));
    headers.insert(HeaderName::from_static("sec-fetch-mode"), HeaderValue::from_static("cors"));
    headers.insert(HeaderName::from_static("sec-fetch-site"), HeaderValue::from_static("same-site"));
    headers.insert(HeaderName::from_static("sec-gpc"), HeaderValue::from_static("1"));
    headers.insert(USER_AGENT, HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36"));

    let client = reqwest::Client::new();
    let word_data: WordApiResponse = client
        .get(url)
        .headers(headers)
        .send()
        .await?
        .json()
        .await?;
    
    //println!("{:?}", word_data);
    
    Ok(WordOfTheDay::new(&word_data.word, &word_data.syllables, &word_data.meaning, &word_data.etymology, &word_data.examples, &word_data.publish_date))
}

fn parse_wrapper(json_data: &str) -> TodayApiResponse {

    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(rename = "oggi")]
        data: TodayApiResponse
    }
    
    let wrapper: Wrapper = serde_json::from_str(json_data).unwrap();
    wrapper.data
}

async fn send_telegram_message(bot_token: &str, chat_ids: Vec<&str>, message: &str) -> Result<(), reqwest::Error> {
    let client = reqwest::Client::new();

    let url = format!("https://api.telegram.org/bot{bot_token}/sendMessage");

    for id in chat_ids {

        let mut data = HashMap::new();
        data.insert("chat_id", id);
        data.insert("text", message);
        data.insert("parse_mode", "HTML");

        let _ = client.post(&url)
            .json(&data)
            .send()
            .await?;

        //println!("{:?}", res);
    }

    Ok(())
}
