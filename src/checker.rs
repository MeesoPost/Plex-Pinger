use std::time::Duration;

pub fn check_plex(url: &str, timeout_seconds: u64) -> bool {
    match ureq::get(url)
        .timeout(Duration::from_secs(timeout_seconds))
        .call()
    {
        Ok(response) => response.status() == 200,
        Err(_) => false,
    }
}

pub fn check_qbit(url: &str, timeout_seconds: u64) -> bool {
    let api_url = format!("{}/api/v2/app/version", url);
    match ureq::get(&api_url)
        .timeout(Duration::from_secs(timeout_seconds))
        .call()
    {
        Ok(response) => response.status() == 200,
        Err(_) => false,
    }
}
