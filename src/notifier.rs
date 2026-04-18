use serde::Serialize;

#[derive(Serialize)]
struct PushoverPayload<'a> {
    token: &'a str,
    user: &'a str,
    message: &'a str,
}

pub fn send_pushover(token: &str, user: &str, message: &str) -> Result<(), String> {
    let payload = PushoverPayload { token, user, message };
    let body = serde_json::to_string(&payload).map_err(|e| e.to_string())?;

    match ureq::post("https://api.pushover.net/1/messages.json")
        .set("Content-Type", "application/json")
        .send_string(&body)
    {
        Ok(_) => Ok(()),
        Err(e) => Err(format!("Pushover request failed: {}", e)),
    }
}
