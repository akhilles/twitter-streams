use circular_queue::CircularQueue;
use serde::{Deserialize, Serialize};
use warp::Filter;

use crate::twitter::{BackPressureEntry, ProcessedTweet};
use crate::SharedData;
use crate::KEYWORDS;

#[derive(Serialize, Deserialize, Debug)]
pub struct BackPressureData {
    data: Vec<(u128, u128)>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeywordFrequencyData {
    labels: Vec<String>,
    data: Vec<f32>,
    emoji_data: Vec<f32>,
}

pub fn back_pressure_data(queue: &CircularQueue<BackPressureEntry>) -> String {
    let back_pressure_data = BackPressureData {
        data: queue.iter().map(|(a, b)| (*a, *b)).collect(),
    };
    serde_json::to_string(&back_pressure_data).unwrap()
}

pub fn keyword_frequency_data(queue: &CircularQueue<ProcessedTweet>) -> String {
    let mut occurrence = [0; KEYWORDS.len()];
    let mut emoji_occurrence = [0; KEYWORDS.len()];
    let mut total = 0;

    for pt in queue.iter() {
        for i in 0..KEYWORDS.len() {
            if pt.keywords[i] {
                if pt.emoji_encountered {
                    emoji_occurrence[i] += 1;
                }
                occurrence[i] += 1;
                total += 1;
            }
        }
    }

    let mut kfqd = KeywordFrequencyData {
        labels: Vec::new(),
        data: Vec::new(),
        emoji_data: Vec::new(),
    };
    for i in 0..KEYWORDS.len() {
        let frequency = occurrence[i] as f32 / total as f32;
        let safe_occurrence = if occurrence[i] == 0 { 1 } else { occurrence[i] };
        let emoji_frequency = emoji_occurrence[i] as f32 / safe_occurrence as f32;
        kfqd.labels.push(KEYWORDS[i].to_string());
        kfqd.data.push(frequency);
        kfqd.emoji_data.push(emoji_frequency);
    }
    serde_json::to_string(&kfqd).unwrap()
}

pub async fn serve_graph_data(shared_data: &'static SharedData) {
    let readme = warp::path("readme").and(warp::fs::file("./README.md"));
    let back_pressure = warp::path("back_pressure")
        .map(move || back_pressure_data(&shared_data.back_pressure_entries.read().unwrap()));
    let keyword_frequency = warp::path("keyword_frequency")
        .map(move || keyword_frequency_data(&shared_data.processed_tweets.read().unwrap()));
    let web = warp::fs::dir("./web/");

    let routes = readme.or(back_pressure).or(keyword_frequency).or(web);
    warp::serve(routes).run(([0, 0, 0, 0], 8080)).await;
}
