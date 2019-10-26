use std::collections::BTreeMap;
use std::env;

use ring::hmac::{sign, Key, HMAC_SHA1_FOR_LEGACY_USE_ONLY};

use crate::util::{generate_nonce, timestamp_as_secs, url_encode};

pub struct Credentials {
    pub consumer_key: String,
    pub consumer_key_secret: String,
    pub token: String,
    pub token_secret: String,
}

impl Credentials {
    pub fn from_env() -> Result<Credentials, env::VarError> {
        let consumer_key = env::var("TWITTER_API_KEY")?;
        let consumer_key_secret = env::var("TWITTER_API_SECRET_KEY")?;
        let token = env::var("TWITTER_API_ACCESS_TOKEN")?;
        let token_secret = env::var("TWITTER_API_ACCESS_TOKEN_SECRET")?;

        Ok(Self {
            consumer_key,
            consumer_key_secret,
            token,
            token_secret,
        })
    }
}

fn sign_base_string<B: AsRef<[u8]>>(
    consumer_key_secret: &str,
    token_secret: &str,
    base_string: B,
) -> String {
    let signing_key = format!("{}&{}", consumer_key_secret, token_secret);
    let signing_key = Key::new(HMAC_SHA1_FOR_LEGACY_USE_ONLY, signing_key.as_bytes());

    let signature = sign(&signing_key, base_string.as_ref());
    let signature = url_encode(base64::encode(signature.as_ref()));

    signature
}

pub(crate) fn header(
    credentials: &Credentials,
    method: &str,
    url: &str,
    query: (&str, &str),
) -> String {
    let query_key = query.0;
    let query_value = url_encode(query.1);
    let nonce = generate_nonce();
    let timestamp = timestamp_as_secs().to_string();
    let mut params: BTreeMap<&str, &str> = [
        ("oauth_consumer_key", credentials.consumer_key.as_str()),
        ("oauth_nonce", nonce.as_str()),
        ("oauth_signature_method", "HMAC-SHA1"),
        ("oauth_timestamp", timestamp.as_str()),
        ("oauth_token", credentials.token.as_str()),
        ("oauth_version", "1.0"),
        (query_key, query_value.as_str()),
    ]
    .iter()
    .cloned()
    .collect();

    let parameter_string = params
        .keys()
        .map(|k| format!("{}={}", k, params[k]))
        .collect::<Vec<String>>()
        .join("&");
    println!("oauth parameter string: {}", parameter_string);
    let signature_base_string = format!(
        "{}&{}&{}",
        method,
        url_encode(url),
        url_encode(parameter_string),
    );
    println!("oauth signature base string: {}", signature_base_string);

    let signature = sign_base_string(
        credentials.consumer_key_secret.as_str(),
        credentials.token_secret.as_str(),
        signature_base_string,
    );
    println!("oauth signature: {}", signature);

    params.insert("oauth_signature", signature.as_str());
    params.remove(query_key);
    // println!("oauth header: {:#?}", params);

    let authorization_header = params
        .keys()
        .map(|k| format!("{}=\"{}\"", k, params[k]))
        .collect::<Vec<String>>()
        .join(", ");
    let authorization_header = format!("OAuth {}", authorization_header);
    println!("oauth header: {}", authorization_header);

    authorization_header
}
