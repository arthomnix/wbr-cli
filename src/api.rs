use std::fmt::{Display, Formatter};
use log::debug;
use color_eyre::eyre::Result;

const WBR_API_BASE: &str = "https://www.whatbeatsrock.com/api/";
const VS: &str = "vs";
const SCORES: &str = "scores";

pub(crate) fn endpoint_url(endpoint: &str) -> String {
    WBR_API_BASE.to_owned() + endpoint
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct WbrGameRequest {
    pub(crate) gid: String,
    pub(crate) guess: String,
    pub(crate) prev: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct WbrCustomGameRequest {
    pub(crate) oid: String,
    pub(crate) guess: String,
    pub(crate) prev: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct WbrLeaderboardRequest {
    pub(crate) gid: String,
    pub(crate) initials: String,
    pub(crate) score: u64,
    pub(crate) text: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct WbrAuthenticatedLeaderboardRequest {
    pub(crate) gid: String,
    pub(crate) score: u64,
    pub(crate) text: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct WbrGameResponseInner {
    pub(crate) guess_wins: bool,
    pub(crate) guess_emoji: String,
    pub(crate) reason: String,
    pub(crate) cache_count: Option<u64>,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct WbrGameResponse {
    data: WbrGameResponseInner,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WbrCustomGame {
    pub(crate) title: String,
    pub(crate) start_word: String,
    pub(crate) start_emoji: String,
    pub(crate) judging_criteria: String,
    pub(crate) judging_criteria_loss: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct WbrCustomResponseInner {
    attribute_data: WbrCustomGame,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct WbrCustomResponse {
    data: WbrCustomResponseInner,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct WbrErrorResponse {
    pub(crate) error: String,
}

impl Display for WbrErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for WbrErrorResponse {}

pub(crate) fn api_post(client: &reqwest::blocking::Client, endpoint: &str, payload: &str) -> Result<String> {
    Ok(client.post(endpoint_url(endpoint))
        .header("Content-Type", "application/jsoN")
        .body(payload.to_string())
        .send()?
        .text()?)
}

pub(crate) fn api_get(client: &reqwest::blocking::Client, endpoint: &str) -> Result<String> {
    Ok(client.get(endpoint_url(endpoint))
        .send()?
        .text()?)
}

fn do_guess_internal(client: &reqwest::blocking::Client, json: &str) -> Result<WbrGameResponseInner> {
    debug!("request {json}");
    let response = api_post(client, VS, json)?;
    debug!("response {response}");
    match serde_json::from_str::<WbrGameResponse>(&response) {
        Ok(resp) => Ok(resp.data),
        Err(_) => Err(serde_json::from_str::<WbrErrorResponse>(&response)?)?
    }
}

pub(crate) fn do_guess(client: &reqwest::blocking::Client, guess: WbrGameRequest) -> Result<WbrGameResponseInner> {
    let json = serde_json::to_string(&guess)?;
    do_guess_internal(client, &json)
}

pub(crate) fn do_custom_guess(client: &reqwest::blocking::Client, guess: WbrCustomGameRequest) -> Result<WbrGameResponseInner> {
    let json = serde_json::to_string(&guess)?;
    do_guess_internal(client, &json)
}

pub(crate) fn submit_score(client: &reqwest::blocking::Client, request: WbrLeaderboardRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    debug!("leaderboard request {json}");
    let response = api_post(client, SCORES, &json)?;
    debug!("leaderboard response {response}");
    Ok(())
}

pub(crate) fn submit_score_authenticated(client: &reqwest::blocking::Client, request: WbrAuthenticatedLeaderboardRequest) -> Result<()> {
    let json = serde_json::to_string(&request)?;
    debug!("leaderboard request {json}");
    let response = api_post(client, SCORES, &json)?;
    debug!("leaderboard response {response}");
    Ok(())
}

pub(crate) fn get_custom_game(client: &reqwest::blocking::Client, oid: &str) -> Result<WbrCustomGame> {
    let response = api_get(client, &format!("users/{oid}/custom"))?;
    debug!("custom game response {response}");
    let game = serde_json::from_str::<WbrCustomResponse>(&response)?;
    Ok(game.data.attribute_data)
}