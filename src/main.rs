use std::str;
use std::sync::RwLock;

use circular_queue::CircularQueue;
use futures::future::join;
use once_cell::sync::Lazy;

mod oauth;
mod server;
mod twitter;
mod util;

use crate::oauth::Credentials;
use crate::server::serve_graph_data;
use crate::twitter::*;

const NUM_PROCESSED_TWEETS_STORED: usize = 400;
const NUM_BACK_PRESSURE_ENTRIES_STORED: usize = 400;
const KEYWORDS: &[&str] = &[
    "twitter",
    "facebook",
    "google",
    "travel",
    "art",
    "music",
    "photography",
    "love",
    "fashion",
    "food",
];
static SHARED_DATA: Lazy<SharedData> = Lazy::new(|| SharedData::new());

pub struct SharedData {
    processed_tweets: RwLock<CircularQueue<ProcessedTweet>>,
    back_pressure_entries: RwLock<CircularQueue<BackPressureEntry>>,
}

impl SharedData {
    fn new() -> Self {
        let processed_tweets = CircularQueue::with_capacity(NUM_PROCESSED_TWEETS_STORED);
        let processed_tweets = RwLock::new(processed_tweets);
        let back_pressure_entries = CircularQueue::with_capacity(NUM_BACK_PRESSURE_ENTRIES_STORED);
        let back_pressure_entries = RwLock::new(back_pressure_entries);
        Self {
            processed_tweets,
            back_pressure_entries,
        }
    }
}

async fn process_tweets(credentials: &Credentials, shared_data: &SharedData) {
    loop {
        match FilteredTweets::new(&credentials).await {
            Ok(mut stream) => stream.stream(shared_data).await,
            Err(e) => println!("err: {:?}", e),
        };
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let credentials = Credentials::from_env()?;

    let server = serve_graph_data(&SHARED_DATA);
    let processor = process_tweets(&credentials, &SHARED_DATA);
    join(server, processor).await;

    Ok(())
}
