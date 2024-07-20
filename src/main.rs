use std::io::Write;
use colored::Colorize;
use log::{debug, LevelFilter};

const API: &str = "https://www.whatbeatsrock.com/api/vs";

#[derive(serde::Serialize, Debug, Clone)]
struct WbrRequest {
    gid: String,
    guess: String,
    prev: String,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct WbrResponseInner {
    guess_wins: bool,
    guess_emoji: String,
    reason: String,
    cached: bool,
    cache_count: Option<u64>,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct WbrResponse {
    data: WbrResponseInner,
}

#[derive(serde::Deserialize, Debug, Clone)]
struct WbrErrorResponse {
    error: String,
}

fn do_guess(client: &reqwest::blocking::Client, guess: WbrRequest) -> Result<WbrResponseInner, WbrErrorResponse> {
    let json = serde_json::to_string(&guess).unwrap();
    debug!("request {json}");
    let response = client.post(API)
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .unwrap()
        .text()
        .unwrap();
    debug!("response {response}");
    serde_json::from_str::<WbrResponse>(&response)
        .map_err(|_| serde_json::from_str::<WbrErrorResponse>(&response).unwrap())
        .map(|response| response.data)
}

fn main() {
    #[cfg(debug_assertions)]
    colog::default_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    #[cfg(not(debug_assertions))]
    colog::init();

    let gid = uuid::Uuid::new_v4().to_string();
    debug!("gid {gid}");
    let client = reqwest::blocking::Client::builder()
        .user_agent("wbr-cli/0.1.0 (+https://github.com/arthomnix/wbr-cli)")
        .build()
        .unwrap();

    let mut prev_guess = "rock".to_string();
    let mut prev_emoji = "🪨".to_string();
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let mut count = 0;

    loop {
        let (guess, response) = loop {
            print!("{} {} {}{} ", "What beats".blue(), prev_guess.bold().blue(), prev_emoji.bold().blue(), "?".blue());
            stdout.flush().unwrap();
            let mut guess = String::new();
            stdin.read_line(&mut guess).unwrap();
            guess = guess.trim().to_string();

            let request = WbrRequest {
                gid: gid.clone(),
                guess: guess.clone(),
                prev: prev_guess.clone(),
            };
            match do_guess(&client, request) {
                Ok(response) => break (guess, response),
                Err(e) => eprintln!("{} {}", "API error:".red(), e.error.red()),
            };
        };

        if response.guess_wins {
            println!("{} {} {} {} {}{}", guess.bold().green(), response.guess_emoji.bold().green(), "beats".green(), prev_guess.bold().green(), prev_emoji.bold().green(), "!".green());
            println!("{}", response.reason.green());
            if let Some(n) = response.cache_count {
                println!("{} {}", n.to_string().bold().green(), "others guessed this too!".green());
            } else {
                println!("{}", "You're the first person to guess this!".green());
            }
            count += 1;
        } else {
            println!("{} {} {} {} {}{}", guess.bold().red(), response.guess_emoji.bold().red(), "does not beat".red(), prev_guess.bold().red(), prev_emoji.bold().red(), "!".red());
            println!("{}", response.reason.red());
            println!("{} {} {}", "You made".blue(), count.to_string().bold().blue(), "correct guesses".blue());
            break;
        }

        prev_guess = guess;
        prev_emoji = response.guess_emoji;
    }
}
