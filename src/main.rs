mod cloudflare;
use cloudflare::{Cloudflare, DnsRecord, DnsRecordDelete, Method, Zone};

use std::env;
use std::process;
use std::time::Duration;

#[derive(Debug, Clone)]
struct EnvVars {
    api_key: String,
    zones_of_interest: Vec<String>,
    delay: u64,
    unique: bool,
}

fn load_env_vars() -> EnvVars {
    let zones_of_interest = env::var("ZONES")
        .expect("Zones were not found in the environment variables.")
        .split(",")
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
    let api_key = env::var("CF_API_KEY")
        .expect("Cloudflare api key was not found in the environment variables.");
    let delay = env::var("DELAY")
        .unwrap_or("300".to_string())
        .parse::<u64>()
        .expect("DELAY environment variable should be an integer.");
    let unique = env::var("UNIQUE").unwrap_or("no".to_string()) == "yes";

    EnvVars {
        zones_of_interest,
        api_key,
        delay,
        unique,
    }
}

fn aws_ip(client: &reqwest::blocking::Client) -> Option<String> {
    let ip = client
        .get("https://checkip.amazonaws.com/")
        .send()
        .ok()?
        .text()
        .ok()?
        .trim()
        .to_owned();

    Some(ip)
}

fn dns_records_type_a(cf: &Cloudflare, zone: &Zone) -> Vec<DnsRecord> {
    let dns_records = cf
        .api::<Vec<DnsRecord>>(
            Method::Get,
            &format!("zones/{}/dns_records?type=A", zone.id),
        )
        .unwrap_or_default()
        .iter()
        .filter(|elem| elem.name == zone.name)
        .cloned()
        .collect::<Vec<_>>();

    dns_records
}

fn clear_other_records(cf: &Cloudflare, zone: &Zone, ip: &str, dns_records: Vec<DnsRecord>) -> () {
    for record in &dns_records {
        let record_ip = &record.content;

        if record_ip != ip {
            let deleted_record = cf.api::<DnsRecordDelete>(
                Method::Delete,
                &format!(
                    "zones/{}/dns_records/{}",
                    zone.id,
                    record.id.clone().unwrap()
                ),
            );
            println!("Deleted other record: {:?}.", deleted_record);
        }
    }
}

fn clear_dead_records(cf: &Cloudflare, zone: &Zone, dns_records: Vec<DnsRecord>) -> () {
    for record in &dns_records {
        let ip = &record.content;
        let output = process::Command::new("ping")
            .args(["-W", "1", "-c", "1", ip].iter())
            .output()
            .expect("Failed to execute ping.");

        if !output.status.success() {
            let deleted_record = cf.api::<DnsRecordDelete>(
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

pub fn add_record(cf: &Cloudflare, zone: &Zone, ip: &str, dns_records: Vec<DnsRecord>) -> () {
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
        let _ = cf.api::<DnsRecord>(
            Method::Post {
                data: serde_json::to_string(&new_record).unwrap(),
            },
            &format!("zones/{}/dns_records?type=A", zone.id),
        );
        println!("Added {:?} in zone {:?}.", new_record, zone.name);
    } else {
        println!("{:?} already exists in zone {:?}.", new_record, zone.name);
    }
}

fn run(cf: &Cloudflare, env_vars: &EnvVars) {
    let ip = aws_ip(&cf.http_client()).expect("Couldn't fetch aws ip.");
    let zones = cf
        .api::<Vec<Zone>>(Method::Get, "zones")
        .unwrap_or_default()
        .iter()
        .filter(|elem| env_vars.zones_of_interest.contains(&elem.name))
        .cloned()
        .collect::<Vec<_>>();

    for zone in &zones {
        let dns_records = dns_records_type_a(&cf, &zone);
        if !dns_records.is_empty() {
            if env_vars.unique {
                clear_other_records(&cf, zone, &ip, dns_records);
            } else {
                clear_dead_records(&cf, zone, dns_records);
            }
        } else {
            println!("No DNS record were found for zone {}.", zone.id)
        }

        let dns_records = dns_records_type_a(&cf, &zone);
        add_record(&cf, zone, &ip, dns_records);
    }
}

fn main() {
    let env_vars = load_env_vars();
    let cloudflare = Cloudflare::new(&env_vars.api_key);
    let delay_in_sec = Duration::from_secs(env_vars.delay);

    loop {
        run(&cloudflare, &env_vars);
        std::thread::sleep(delay_in_sec);
    }
}
