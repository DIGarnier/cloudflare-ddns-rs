mod cloudflare;
use std::env;
use std::time::Duration;

#[derive(Debug, Clone)]
struct EnvVars {
    api_key: String,
    zones_of_interest: Vec<String>,
    delay: u64,
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

    EnvVars {
        zones_of_interest,
        api_key,
        delay,
    }
}

fn refresh_records(cloudflare: &cloudflare::Cloudflare, zone: &cloudflare::Zone, ip: &str) -> () {
    cloudflare.clear_dead_records(zone);
    cloudflare.add_record(zone, ip);
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

fn run(cloudflare: &cloudflare::Cloudflare, env_vars: &EnvVars) {
    let ip = aws_ip(&cloudflare.http_client()).expect("Couldn't fetch aws ip.");
    let zones = cloudflare
        .api::<Vec<cloudflare::Zone>>(cloudflare::Method::Get, "zones")
        .unwrap_or_default()
        .iter()
        .filter(|elem| env_vars.zones_of_interest.contains(&elem.name))
        .cloned()
        .collect::<Vec<_>>();

    for zone in &zones {
        refresh_records(&cloudflare, zone, &ip);
    }
}

fn main() {
    let env_vars = load_env_vars();
    let cloudflare = cloudflare::Cloudflare::new(&env_vars.api_key);
    let delay_in_sec = Duration::from_secs(env_vars.delay);

    loop {
        run(&cloudflare, &env_vars);
        std::thread::sleep(delay_in_sec);
    }
}
