use serde::{de::DeserializeOwned, Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone)]
pub struct Zone {
    pub name: String,
    pub id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DnsRecord {
    pub name: String,
    pub content: String,
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub type_: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DnsRecordDelete {
    pub id: String,
}

#[derive(Debug, Deserialize, Clone)]
struct APIResponse<T> {
    result: T,
}
pub enum Method {
    Get,
    Post { data: String },
    Delete,
}

#[derive(Debug)]
pub struct Cloudflare {
    client: reqwest::blocking::Client,
    api_key: String,
}

impl Cloudflare {
    pub fn new(api_key: &str) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api_key: api_key.to_owned(),
        }
    }

    pub fn http_client(&self) -> &reqwest::blocking::Client {
        &self.client
    }

    pub fn api<T: DeserializeOwned + Clone>(&self, method: Method, api_route: &str) -> Option<T> {
        let response = {
            use reqwest::blocking::Client;
            let url = format!("https://api.cloudflare.com/client/v4/{}", api_route);
            match method {
                Method::Get => Client::get(self.http_client(), url),
                Method::Post { data } => Client::post(self.http_client(), url).body(data),
                Method::Delete => Client::delete(self.http_client(), url),
            }
        }
        .bearer_auth(&self.api_key[..])
        .header("Content-Type", "application/json")
        .send()
        .ok()?
        .json::<APIResponse<T>>()
        .ok()?;

        Some(response.result)
    }
}
