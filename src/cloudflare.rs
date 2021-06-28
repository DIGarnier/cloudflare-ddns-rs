use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::process;

#[derive(Debug, Deserialize, Clone)]
pub struct Zone {
    pub name: String,
    id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct DnsRecord {
    name: String,
    content: String,
    id: Option<String>,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Deserialize, Clone)]
struct DnsRecordDelete {
    id: String,
}

#[derive(Debug, Deserialize, Clone)]
struct APIResponse<T> {
    result: T,
}
pub enum Method {
    Get,
    Post,
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
        self.api_with_data(method, api_route, "")
    }

    pub fn api_with_data<T: DeserializeOwned + Clone>(
        &self,
        method: Method,
        api_route: &str,
        data: &str,
    ) -> Option<T> {
        let response = {
            use reqwest::blocking::Client;
            let url = format!("https://api.cloudflare.com/client/v4/{}", api_route);
            match method {
                Method::Get => Client::get(self.http_client(), url),
                Method::Post => Client::post(self.http_client(), url),
                Method::Delete => Client::delete(self.http_client(), url),
            }
        }
        .bearer_auth(&self.api_key[..])
        .header("Content-Type", "application/json")
        .body(data.to_owned())
        .send()
        .ok()?
        .json::<APIResponse<T>>()
        .ok()?;

        Some(response.result)
    }

    pub fn clear_dead_records(&self, zone: &Zone) -> () {
        let dns_records = self
            .api::<Vec<DnsRecord>>(Method::Get, &format!("zones/{}/dns_records?type=A", zone.id))
            .unwrap_or_default()
            .iter()
            .filter(|elem| elem.name == zone.name)
            .cloned()
            .collect::<Vec<_>>();

        if dns_records.len() == 0 {
            println!("No DNS record were found for zone {}.", zone.id)
        }

        for record in &dns_records {
            let ip = &record.content;
            let output = process::Command::new("ping")
                .args(["-W", "1", "-c", "1", ip].iter())
                .output()
                .expect("Failed to execute ping.");

            if !output.status.success() {
                let deleted_record = self.api::<DnsRecordDelete>(
                    Method::Delete,
                    &format!(
                        "zones/{}/dns_records/{}",
                        zone.id,
                        record.id.clone().unwrap()
                    ),
                );
                println!("Deleted stale record: {:?}.", deleted_record);
            }
        }
    }

    pub fn add_record(&self, zone: &Zone, ip: &str) -> () {
        let dns_record_api_route = &format!("zones/{}/dns_records?type=A", zone.id);
        let dns_records = self
            .api::<Vec<DnsRecord>>(Method::Get, dns_record_api_route)
            .unwrap_or_default()
            .iter()
            .filter(|elem| elem.name == zone.name)
            .cloned()
            .collect::<Vec<_>>();

        if dns_records.len() == 0 {
            println!("No DNS record were found for zone {}.", zone.id)
        }

        let new_record = DnsRecord {
            name: zone.name.clone(),
            type_: "A".to_string(),
            content: ip.to_owned(),
            id: None,
        };
        if !dns_records
            .iter()
            .any(|rec| rec.name == new_record.name && rec.content == new_record.content)
        {
            let _ = self.api_with_data::<DnsRecord>(
                Method::Post,
                dns_record_api_route,
                &serde_json::to_string(&new_record).unwrap(),
            );
            println!("Added {:?} in zone {:?}.", new_record, zone.name);
        } else {
            println!("{:?} already exists in zone {:?}.", new_record, zone.name);
        }
    }
}
