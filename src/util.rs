use std::time::{SystemTime, UNIX_EPOCH};

use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use regex::Regex;
use ring::rand::{SecureRandom, SystemRandom};

const URL_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'<')
    .add(b'>')
    .add(b'`')
    .add(b'#')
    .add(b'?')
    .add(b'/')
    .add(b':')
    .add(b'=')
    .add(b'@')
    .add(b'&')
    .add(b'%')
    .add(b',');

const CR: u8 = 0x0D;
const LF: u8 = 0x0A;

pub(crate) fn find_cr_lf(buf: &[u8]) -> Option<usize> {
    if buf.len() < 2 {
        return None;
    }

    for i in 0..buf.len() - 1 {
        if buf[i] == CR && buf[i + 1] == LF {
            return Some(i);
        }
    }
    None
}

pub(crate) fn url_encode<S: AsRef<str>>(url: S) -> String {
    utf8_percent_encode(url.as_ref(), URL_SET).to_string()
}

pub(crate) fn generate_nonce() -> String {
    let nonce = &mut [0u8; 16];
    SystemRandom::new().fill(nonce).unwrap();
    hex::encode(nonce)
}

pub(crate) fn timestamp_as_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

pub(crate) fn timestamp_as_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

pub(crate) fn contains_emoji(message: &str) -> bool {
    let re = Regex::new(r"\p{Emoji}").unwrap();
    re.is_match(message)
}
