use std::fmt::{Display, Formatter};
use log::debug;
use color_eyre::eyre::Result;

const WBR_API_BASE: &str = "https://www.whatbeatsrock.com/api/";
const VS_ENDPOINT: &str = "vs";
const SCORES_ENDPOINT: &str = "scores";
const LIKE_ENDPOINT: &str = "me/custom/like";

pub(crate) fn endpoint_url(endpoint: &str) -> String {
    WBR_API_BASE.to_owned() + endpoint
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct GameRequest {
    pub(crate) gid: String,
    pub(crate) guess: String,
    pub(crate) prev: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct CustomGameRequest {
    pub(crate) oid: String,
    pub(crate) guess: String,
    pub(crate) prev: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct LeaderboardRequest {
    pub(crate) gid: String,
    pub(crate) initials: String,
    pub(crate) score: u64,
    pub(crate) text: String,
}

#[derive(serde::Serialize, Debug, Clone)]
pub(crate) struct AuthenticatedLeaderboardRequest {
    pub(crate) gid: String,
    pub(crate) score: u64,
    pub(crate) text: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct GameResponseInner {
    pub(crate) guess_wins: bool,
    pub(crate) guess_emoji: String,
    pub(crate) reason: String,
    pub(crate) cache_count: Option<u64>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct GameResponse {
    data: GameResponseInner,
}

#[derive(serde::Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CustomGameAttributes {
    pub(crate) title: String,
    pub(crate) start_word: String,
    pub(crate) start_emoji: String,
    pub(crate) judging_criteria: String,
    pub(crate) judging_criteria_loss: String,
}


#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct Vote {
    pub(crate) is_upvote: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct CustomGame {
    pub(crate) id: String,
    pub(crate) attribute_data: CustomGameAttributes,
    pub(crate) execution_count: u64,
    pub(crate) denormalized_vote_count: u64,
    pub(crate) vote: Vec<Vote>,
}

impl CustomGame {
    /// Returns whether the user has liked this game
    pub(crate) fn has_liked(&self) -> bool {
        self.vote.len() == 1 && self.vote[0].is_upvote
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
struct CustomResponse {
    data: CustomGame,
}

#[derive(serde::Serialize, Debug, Clone)]
struct LikeRequest {
    fid: String,
    is_upvote: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct SuccessResponse {
    success: bool,
}

#[derive(serde::Deserialize, Debug, Clone)]
pub(crate) struct ErrorResponse {
    pub(crate) error: String,
}

impl Display for ErrorResponse {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.error)
    }
}

impl std::error::Error for ErrorResponse {}

pub(crate) fn api_post(client: &reqwest::blocking::Client, endpoint: &str, payload: &str) -> Result<String> {
    debug!("request POST /api/{endpoint} {payload}");
    let text = client.post(endpoint_url(endpoint))
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()?
        .text()?;
    debug!("response {text}");
    Ok(text)
}

pub(crate) fn api_put(client: &reqwest::blocking::Client, endpoint: &str, payload: &str) -> Result<String> {
    debug!("request PUT /api/{endpoint} {payload}");
    let text = client.put(endpoint_url(endpoint))
        .header("Content-Type", "application/json")
        .body(payload.to_string())
        .send()?
        .text()?;
    debug!("response {text}");
    Ok(text)
}

pub(crate) fn api_get(client: &reqwest::blocking::Client, endpoint: &str) -> Result<String> {
    debug!("request GET /api/{endpoint}");
    let text = client.get(endpoint_url(endpoint))
        .send()?
        .text()?;
    debug!("response {text}");
    Ok(text)
}

fn do_guess_internal(client: &reqwest::blocking::Client, json: &str) -> Result<GameResponseInner> {
    let response = api_post(client, VS_ENDPOINT, json)?;
    match serde_json::from_str::<GameResponse>(&response) {
        Ok(resp) => Ok(resp.data),
        Err(_) => Err(serde_json::from_str::<ErrorResponse>(&response)?)?
    }
}

pub(crate) fn do_guess(client: &reqwest::blocking::Client, guess: GameRequest) -> Result<GameResponseInner> {
    let json = serde_json::to_string(&guess)?;
    do_guess_internal(client, &json)
}

pub(crate) fn do_custom_guess(client: &reqwest::blocking::Client, guess: CustomGameRequest) -> Result<GameResponseInner> {
    let json = serde_json::to_string(&guess)?;
    do_guess_internal(client, &json)
}

pub(crate) fn submit_score(client: &reqwest::blocking::Client, request: LeaderboardRequest) -> Result<bool> {
    let json = serde_json::to_string(&request)?;
    let response = api_post(client, SCORES_ENDPOINT, &json)?;
    let success = serde_json::from_str::<SuccessResponse>(&response)?;
    Ok(success.success)
}

pub(crate) fn submit_score_authenticated(client: &reqwest::blocking::Client, request: AuthenticatedLeaderboardRequest) -> Result<bool> {
    let json = serde_json::to_string(&request)?;
    let response = api_post(client, SCORES_ENDPOINT, &json)?;
    let success = serde_json::from_str::<SuccessResponse>(&response)?;
    Ok(success.success)
}

pub(crate) fn get_custom_game(client: &reqwest::blocking::Client, oid: &str) -> Result<CustomGame> {
    let response = api_get(client, &format!("users/{oid}/custom"))?;
    let game = serde_json::from_str::<CustomResponse>(&response)?;
    Ok(game.data)
}

pub(crate) fn like_custom_game(client: &reqwest::blocking::Client, fid: &str) -> Result<bool> {
    let request = LikeRequest {
        fid: fid.to_string(),
        is_upvote: true,
    };
    let json = serde_json::to_string(&request)?;
    let response = api_put(client, LIKE_ENDPOINT, &json)?;
    let success = serde_json::from_str::<SuccessResponse>(&response)?;
    Ok(success.success)
}