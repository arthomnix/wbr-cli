mod auth;
mod api;
mod save;

use std::io::Write;
use std::sync::Arc;
use clap::Parser;
use colored::Colorize;
use color_eyre::eyre::Result;
use color_eyre::owo_colors::OwoColorize;
use log::{debug, LevelFilter};
use crate::api::{do_guess, submit_score, submit_score_authenticated, AuthenticatedLeaderboardRequest, LeaderboardRequest, GameRequest, GameResponseInner, get_custom_game, CustomGameRequest, do_custom_guess, like_custom_game};
use crate::auth::{add_auth_cookie, auth_prompt, get_session_cookies, get_user_id};
use crate::save::SaveData;

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

fn do_game(is_custom: bool, start_guess: &str, start_emoji: &str, judging_criteria_win: &str, judging_criteria_loss: &str, mut save_template: SaveData, callback: impl Fn(&str, &str) -> Result<GameResponseInner>) -> Result<GameResult> {
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

            if guess == "EXIT" {
                print!("{}", "Save game? [y/N] ".blue());
                if read_yes_no_prompt(true)? {
                    save_template.update(prev_guess, prev_emoji, count);
                    save_template.save()?;
                }
                std::process::exit(0);
            }

            match callback(&guess, &prev_guess) {
                Ok(response) => break (guess, response),
                Err(e) => eprintln!("{} {}", "API error:".red(), e.to_string().red()),
            };
        };

        if response.guess_wins {
            println!("{} {} {} {} {}{}", guess.bold().green(), response.guess_emoji.bold().green(), judging_criteria_win.green(), prev_guess.bold().green(), prev_emoji.bold().green(), "!".green());
            println!("{}", response.reason.green());
            if !is_custom {
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

fn run_normal(client: &reqwest::blocking::Client, authenticated: bool, mut save_data: Option<SaveData>) -> Result<()> {
    let mut gid = save_data.as_ref().map(|d| d.gid_oid.clone()).unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    debug!("gid {gid}");

    loop {
        let save_template = SaveData::template(false, gid.clone());

        let result = do_game(
            false,
            save_data.as_ref().map(|d| d.prev_guess.as_ref()).unwrap_or("rock"),
            save_data.as_ref().map(|d| d.prev_emoji.as_ref()).unwrap_or("🪨"),
            "beats",
            "does not beat",
            save_template,
            |guess, prev_guess| {
                let request = GameRequest {
                    gid: gid.clone(),
                    guess: guess.to_string(),
                    prev: prev_guess.to_string(),
                };
                do_guess(&client, request)
            }
        )?;

        print!("{}", "Would you like to submit to the leaderboard? [y/N] ".blue());
        if read_yes_no_prompt(true)? {
            if authenticated {
                let leaderboard_request = AuthenticatedLeaderboardRequest {
                    gid: gid.clone(),
                    score: result.score,
                    text: format!("{} {} did not beat {} {}", result.guess, result.emoji, result.prev_guess, result.prev_emoji),
                };
                if !submit_score_authenticated(&client, leaderboard_request)? {
                    println!("{}", "submit score unsuccessful".red());
                }
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

                let leaderboard_request = LeaderboardRequest {
                    gid: gid.clone(),
                    initials,
                    score: result.score,
                    text: format!("{} {} did not beat {} {}", result.guess, result.emoji, result.prev_guess, result.prev_emoji),
                };
                if !submit_score(&client, leaderboard_request)? {
                    println!("{}", "submit score unsuccessful".red());
                }
            }
        }

        print!("{}", "Play again? [y/N] ".blue());
        if !read_yes_no_prompt(true)? {
            break;
        }

        // only load the save the first time round
        save_data = None;
        // New gid for new game
        gid = uuid::Uuid::new_v4().to_string();
    }

    Ok(())
}

fn run_custom(client: &reqwest::blocking::Client, authenticated: bool, oid: String, mut save_data: Option<SaveData>) -> Result<()> {
    debug!("custom game oid {oid}");
    let game_info = get_custom_game(&client, &oid)?;
    println!(
        "{} {}{}{} {} {} {}",
        "Loaded custom game".blue(),
        game_info.attribute_data.title.bold().blue(),
        "! (".blue(),
        game_info.denormalized_vote_count.to_string().bold().blue(),
        "likes,".blue(),
        game_info.execution_count.to_string().bold().blue(),
        "plays)".blue()
    );

    loop {
        let save_template = SaveData::template(true, oid.clone());

        do_game(
            true,
            save_data.as_ref().map(|d| d.prev_guess.as_ref()).unwrap_or(&game_info.attribute_data.start_word),
            save_data.as_ref().map(|d| d.prev_emoji.as_ref()).unwrap_or(&game_info.attribute_data.start_emoji),
            &game_info.attribute_data.judging_criteria,
            &game_info.attribute_data.judging_criteria_loss,
            save_template,
            |guess, prev_guess| {
                let request = CustomGameRequest {
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

        // only load the save first time round
        save_data = None;
    }

    if authenticated && !game_info.has_liked() {
        print!("{}", "Like this custom game? [y/N] ".blue());
        if read_yes_no_prompt(true)? {
            if !like_custom_game(&client, &game_info.id)? {
                println!("{}", "like unsuccessful".red());
            }
        }
    }

    Ok(())
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

    if let Some(save) = SaveData::load()? {
        println!("{}", "Loaded saved game".italic().blue());
        if save.is_custom {
            run_custom(&client, uid.is_some(), save.gid_oid.clone(), Some(save))?;
        } else {
            run_normal(&client, uid.is_some(), Some(save))?;
        }
    } else {
        if let Some(custom_username) = args.custom_username {
            let username = custom_username.strip_prefix('@').unwrap_or(&custom_username);
            let oid = get_user_id(&client, username)?;

            run_custom(&client, uid.is_some(), oid, None)?;
        } else {
            run_normal(&client, uid.is_some(), None)?;
        }
    }

    Ok(())
}