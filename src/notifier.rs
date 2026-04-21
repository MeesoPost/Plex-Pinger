use serde::Serialize;
use ureq::Agent;

const PUSHOVER_URL: &str = "https://api.pushover.net/1/messages.json";

#[derive(Serialize)]
struct PushoverPayload<'a> {
    token: &'a str,
    user: &'a str,
    message: &'a str,
}

pub fn send_pushover(
    agent: &Agent,
    token: &str,
    user: &str,
    message: &str,
) -> Result<(), Box<ureq::Error>> {
    agent
        .post(PUSHOVER_URL)
        .send_json(PushoverPayload { token, user, message })
        .map_err(Box::new)?;
    Ok(())
}
