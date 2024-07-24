mod auth;
mod api;

use std::io::Write;
use std::sync::Arc;
use clap::Parser;
use colored::Colorize;
use color_eyre::eyre::Result;
use log::{debug, LevelFilter};
use crate::api::{do_guess, submit_score, submit_score_authenticated, WbrAuthenticatedLeaderboardRequest, WbrLeaderboardRequest, WbrGameRequest, WbrGameResponseInner, get_custom_game, WbrCustomGameRequest, do_custom_guess};
use crate::auth::{add_auth_cookie, auth_prompt, get_session_cookies, get_user_id};

#[derive(Parser, Debug)]
#[command(version)]
struct Args {
    #[arg(short, long)]
    custom_username: Option<String>,
}

fn read_yes_no_prompt(default_no: bool) -> Result<bool> {
    std::io::stdout().flush()?;
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf)?;
    Ok(if default_no {
        buf.to_lowercase().starts_with('y')
    } else {
        !buf.to_lowercase().starts_with('n')
    })
}

struct GameResult {
    score: u64,
    guess: String,
    emoji: String,
    prev_guess: String,
    prev_emoji: String,
}

fn do_game(display_cache: bool, start_guess: &str, start_emoji: &str, judging_criteria_win: &str, judging_criteria_loss: &str, callback: impl Fn(&str, &str) -> Result<WbrGameResponseInner>) -> Result<GameResult> {
    let mut count: u64 = 0;
    let mut prev_guess = start_guess.to_string();
    let mut prev_emoji = start_emoji.to_string();

    loop {
        let (guess, response) = loop {
            print!("{} {} {} {}{} ", "What".blue(), judging_criteria_win.blue(), prev_guess.bold().blue(), prev_emoji.bold().blue(), "?".blue());
            std::io::stdout().flush()?;
            let mut guess = String::new();
            std::io::stdin().read_line(&mut guess)?;
            guess = guess.trim().to_string();

            match callback(&guess, &prev_guess) {
                Ok(response) => break (guess, response),
                Err(e) => eprintln!("{} {}", "API error:".red(), e.to_string().red()),
            };
        };

        if response.guess_wins {
            println!("{} {} {} {} {}{}", guess.bold().green(), response.guess_emoji.bold().green(), judging_criteria_win.green(), prev_guess.bold().green(), prev_emoji.bold().green(), "!".green());
            println!("{}", response.reason.green());
            if display_cache {
                if let Some(n) = response.cache_count {
                    println!("{} {}", n.to_string().bold().green(), "others guessed this too!".green());
                } else {
                    println!("{}", "You're the first person to guess this!".green());
                }
            }
            count += 1;
        } else {
            println!("{} {} {} {} {}{}", guess.bold().red(), response.guess_emoji.bold().red(), judging_criteria_loss.red(), prev_guess.bold().red(), prev_emoji.bold().red(), "!".red());
            println!("{}", response.reason.red());
            println!("{} {} {}", "You made".blue(), count.to_string().bold().blue(), "correct guesses".blue());
            break Ok(GameResult {
                score: count,
                guess,
                emoji: response.guess_emoji,
                prev_guess,
                prev_emoji
            });
        }

        prev_guess = guess;
        prev_emoji = response.guess_emoji;
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    #[cfg(debug_assertions)]
    colog::default_builder()
        .filter_level(LevelFilter::Debug)
        .init();

    #[cfg(not(debug_assertions))]
    colog::default_builder()
        .filter_level(LevelFilter::Warn)
        .init();

    let args = Args::parse();

    let cookie_jar = Arc::new(reqwest::cookie::Jar::default());
    let client = reqwest::blocking::Client::builder()
        .user_agent(format!("wbr-cli/{} (+https://github.com/arthomnix/wbr-cli)", env!("CARGO_PKG_VERSION")))
        .cookie_provider(Arc::clone(&cookie_jar))
        .build()?;

    let accounts = get_session_cookies(&client, &cookie_jar)?;
    let uid = if let Some(account) = auth_prompt(accounts)? {
        add_auth_cookie(&cookie_jar, &account.auth_cookie);
        Some(account.user_id)
    } else {
        None
    };

    if let Some(custom_username) = args.custom_username {
        let username = custom_username.strip_prefix('@').unwrap_or(&custom_username);
        let oid = get_user_id(&client, username)?;
        debug!("custom game oid {oid}");

        let game_info = get_custom_game(&client, &oid)?;
        println!("{} {}{}", "Loaded custom game".blue(), game_info.title.bold().blue(), "!".blue());

        loop {
            do_game(
                false,
                &game_info.start_word,
                &game_info.start_emoji,
                &game_info.judging_criteria,
                &game_info.judging_criteria_loss,
                |guess, prev_guess| {
                    let request = WbrCustomGameRequest {
                        oid: oid.clone(),
                        guess: guess.to_string(),
                        prev: prev_guess.to_string(),
                    };
                    do_custom_guess(&client, request)
                }
            )?;

            print!("{}", "Play again? [y/N] ".blue());
            if !read_yes_no_prompt(true)? {
                break;
            }
        }
    } else {
        let mut gid = uuid::Uuid::new_v4().to_string();
        debug!("gid {gid}");

        loop {
            let result = do_game(
                true,
                "rock",
                "🪨",
                "beats",
                "does not beat",
                |guess, prev_guess| {
                    let request = WbrGameRequest {
                        gid: gid.clone(),
                        guess: guess.to_string(),
                        prev: prev_guess.to_string(),
                    };
                    do_guess(&client, request)
                }
            )?;

            print!("{}", "Would you like to submit to the leaderboard? [y/N] ".blue());
            if read_yes_no_prompt(true)? {
                if uid.is_some() {
                    let leaderboard_request = WbrAuthenticatedLeaderboardRequest {
                        gid: gid.clone(),
                        score: result.score,
                        text: format!("{} {} did not beat {} {}", result.guess, result.emoji, result.prev_guess, result.prev_emoji),
                    };
                    submit_score_authenticated(&client, leaderboard_request)?;
                } else {
                    let mut buf = String::new();
                    let initials = loop {
                        print!("{}", "Enter leaderboard initials (3 characters): ".blue());
                        std::io::stdout().flush()?;
                        buf.clear();
                        std::io::stdin().read_line(&mut buf)?;
                        let buf = buf.trim().to_string();
                        if buf.chars().count() == 3 {
                            break buf;
                        }
                        print!("{}", "Must be 3 characters!".red());
                    };

                    let leaderboard_request = WbrLeaderboardRequest {
                        gid: gid.clone(),
                        initials,
                        score: result.score,
                        text: format!("{} {} did not beat {} {}", result.guess, result.emoji, result.prev_guess, result.prev_emoji),
                    };
                    submit_score(&client, leaderboard_request)?;
                }
            }

            print!("{}", "Play again? [y/N] ".blue());
            if !read_yes_no_prompt(true)? {
                break;
            }

            // New gid for new game
            gid = uuid::Uuid::new_v4().to_string();
        }
    }

    Ok(())
}