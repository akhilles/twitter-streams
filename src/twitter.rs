use std::str;

use bytes::Bytes;
use futures::channel::mpsc::channel;
use futures::{future::join, SinkExt, Stream, StreamExt};
use reqwest::{header, Client, Response};
use serde::{Deserialize, Serialize};

use crate::oauth;
use crate::util::{contains_emoji, find_cr_lf, timestamp_as_millis};
use crate::SharedData;
use crate::KEYWORDS;

pub const STREAM_API_URL: &str = "https://stream.twitter.com/1.1/statuses/filter.json";

pub struct FilteredTweets {
    stream: Response,
    partial_payload: Bytes,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    pub created_at: String,
    pub followers_count: i32,
    pub friends_count: i32,
    pub statuses_count: i32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct OriginalTweet {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Tweet {
    pub created_at: String,
    pub favorite_count: i32,
    pub retweet_count: i32,
    pub reply_count: i32,
    pub text: String,
    pub timestamp_ms: String,
    pub retweeted_status: Option<OriginalTweet>,
    pub user: User,
}

#[derive(Debug)]
pub struct ProcessedTweet {
    pub keywords: [bool; KEYWORDS.len()],
    len: usize,
    timestamp: u128,
    pub emoji_encountered: bool,
}

pub type BackPressureEntry = (u128, u128);

impl FilteredTweets {
    pub async fn new(credentials: &oauth::Credentials) -> Result<Self, Box<dyn std::error::Error>> {
        let keywords_concat = KEYWORDS.join(",");
        let query = ("track", keywords_concat.as_str());
        let oauth_header = oauth::header(credentials, "POST", STREAM_API_URL, query);

        let url = format!("{}?track={}", STREAM_API_URL, keywords_concat);

        let client = Client::builder().build()?;
        let request = client
            .post(url.as_str())
            .header(header::AUTHORIZATION, oauth_header)
            .build()?;
        // println!("request: {:?}", &request);

        let response = client.execute(request).await?.error_for_status()?;
        // println!("response: {:?}", &response);

        Ok(Self {
            stream: response,
            partial_payload: Bytes::new(),
        })
    }

    async fn payload(&mut self) -> Result<Bytes, reqwest::Error> {
        let mut cr_lf_index = find_cr_lf(self.partial_payload.as_ref());

        while cr_lf_index == None {
            // println!("fetching new chunk ...");
            let new_chunk = self.stream.chunk().await?.unwrap();
            // println!("new chunk: {}", str::from_utf8(new_chunk.as_ref()).unwrap());
            self.partial_payload.extend_from_slice(new_chunk.as_ref());

            cr_lf_index = find_cr_lf(self.partial_payload.as_ref());
            // println!("cr_lf_index: {:?}", cr_lf_index);
        }
        let payload = self.partial_payload.split_to(cr_lf_index.unwrap() + 2);
        Ok(payload)
    }

    pub async fn stream(&mut self, shared_data: &SharedData) {
        let (mut tx, rx) = channel(100);
        let tx = async move {
            loop {
                match self.payload().await {
                    Ok(payload) => {
                        // println!("sending payload {}, len: {}", i, payload.len());
                        let _ = tx.send(payload).await;
                    }
                    Err(_) => {}
                };
            }
        };

        let rx = rx.for_each_concurrent(10, |payload: Bytes| {
            if let Ok(tweet) = serde_json::from_slice::<Tweet>(payload.as_ref()) {
                // ignore retweets
                match process_tweet(tweet) {
                    Some((processed_tweet, back_pressure_entry)) => {
                        // println!("back pressure: {:?}", back_pressure_entry);
                        // println!("{:?}", processed_tweet);
                        shared_data
                            .processed_tweets
                            .write()
                            .unwrap()
                            .push(processed_tweet);
                        shared_data
                            .back_pressure_entries
                            .write()
                            .unwrap()
                            .push(back_pressure_entry);
                    }
                    None => {}
                }
            }
            // let tweet: Tweet = serde_json::from_slice(payload.as_ref()).unwrap();
            // println!("received payload, {:?}", tweet);
            futures::future::ready(())
        });
        join(tx, rx).await;
    }
}

pub(crate) fn process_tweet(tweet: Tweet) -> Option<(ProcessedTweet, BackPressureEntry)> {
    if tweet.retweeted_status.is_none() {
        return None;
    }
    let text = tweet.text.as_str();

    let mut encountered_keywords = [false; KEYWORDS.len()];
    let mut contains_keyword = false;
    for i in 0..KEYWORDS.len() {
        let keyword_found = text.contains(KEYWORDS[i]);
        encountered_keywords[i] = keyword_found;
        contains_keyword |= keyword_found;
    }
    if !contains_keyword {
        return None;
    };

    let timestamp = tweet.timestamp_ms.parse::<u128>().unwrap();
    let processed_tweet = ProcessedTweet {
        keywords: encountered_keywords,
        len: text.len(),
        timestamp,
        emoji_encountered: contains_emoji(text),
    };
    let back_pressure_entry = (timestamp, timestamp_as_millis() - timestamp);

    Some((processed_tweet, back_pressure_entry))
}
