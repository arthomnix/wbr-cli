use std::io::Write;
use std::str::FromStr;
use log::debug;
use url::Url;
use color_eyre::eyre::Result;
use crate::read_yes_no_prompt;
use crate::api::{api_get, endpoint_url};

const USER: &str = "https://xrrlbpmfxuxumxqbccxz.supabase.co/auth/v1/user";
const SUPABASE_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InhycmxicG1meHV4dW14cWJjY3h6Iiwicm9sZSI6ImFub24iLCJpYXQiOjE2OTIyMzc2NTAsImV4cCI6MjAwNzgxMzY1MH0.8Xae0-VrRVKTGmMSJt2o0WGL6Q5NXgWdAyASsXEjv4E";
const AUTH_COOKIE_NAME: &str = "sb-xrrlbpmfxuxumxqbccxz-auth-token";

#[derive(serde::Deserialize, Clone, Debug)]
struct SupabaseUserResponse {
    id: String,
    role: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct WbrProfileResponseInner {
    id: String,
    handle: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct WbrProfileResponse {
    data: WbrProfileResponseInner,
}

#[derive(Clone, Debug)]
pub(crate) struct AuthInfo {
    pub(crate) username: String,
    pub(crate) user_id: String,
    pub(crate) auth_cookie: String,
}

pub(crate) fn get_user_id(client: &reqwest::blocking::Client, handle: &str) -> Result<String> {
    let response = api_get(client, &format!("users?handle={handle}"))?;
    let profile = serde_json::from_str::<WbrProfileResponse>(&response)?;
    Ok(profile.data.id)
}

pub(crate) fn add_auth_cookie(jar: &reqwest::cookie::Jar, cookie: &str) {
    jar.add_cookie_str(
        &format!("{}={}; Domain=www.whatbeatsrock.com; SameSite=Lax;", AUTH_COOKIE_NAME, cookie),
        &"https://www.whatbeatsrock.com".parse::<Url>().unwrap()
    );
}

pub(crate) fn get_session_cookies(client: &reqwest::blocking::Client, jar: &reqwest::cookie::Jar) -> Result<Vec<AuthInfo>> {
    Ok(rookie::load(Some(vec!["www.whatbeatsrock.com".to_string()]))?
        .into_iter()
        .filter_map(|cookie| {
            debug!("found cookie {cookie:?}");
            if cookie.name != AUTH_COOKIE_NAME {
                return None;
            }

            let decoded = urlencoding::decode(&cookie.value).ok()?;
            debug!("{decoded}");
            let token_parts = serde_json::from_str::<Vec<Option<String>>>(&decoded).ok()?;
            let user_info = match client.get(USER)
                .header("apikey", SUPABASE_KEY)
                .bearer_auth(token_parts[0].as_ref()?)
                .send()
                .map(|r| {
                    let text = r.text();
                    debug!("{text:?}");
                    text.map(|t| serde_json::from_str::<SupabaseUserResponse>(&t))
                })
            {
                Ok(Ok(Ok(user_info))) => user_info,
                _ => {
                    debug!("user invalid");
                    return None;
                },
            };

            if !(&user_info.role == "authenticated") {
                debug!("user not authenticated");
                return None;
            }

            add_auth_cookie(&jar, &decoded);

            let profile = match client.get(endpoint_url(&format!("users/{}/profile", &user_info.id)))
                .send()
                .map(|r| {
                    let text = r.text();
                    debug!("{text:?}");
                    text.map(|t| serde_json::from_str::<WbrProfileResponse>(&t))
                })
            {
                Ok(Ok(Ok(profile))) => profile,
                _ => {
                    debug!("get profile failed");
                    return None;
                },
            };

            debug!("found user id {} username {}", &user_info.id, &profile.data.handle);

            Some(AuthInfo {
                username: profile.data.handle,
                user_id: user_info.id,
                auth_cookie: decoded.to_string(),
            })
        })
        .collect::<Vec<AuthInfo>>())
}

pub(crate) fn auth_prompt(accounts: Vec<AuthInfo>) -> Result<Option<AuthInfo>> {
    if accounts.len() == 0 {
        println!("If you want to use an account, log in at https://www.whatbeatsrock.com/login in your web browser!");
        return Ok(None);
    }

    if accounts.len() == 1 {
        println!("Found logged in account: @{}", accounts[0].username);
        print!("Use this account? [Y/n] ");
        if read_yes_no_prompt(false)? {
            Ok(Some(accounts[0].clone()))
        } else {
            Ok(None)
        }
    } else {
        println!("Found multiple logged in accounts:");
        for (n, account) in accounts.iter().enumerate() {
            println!("[{}]: @{}", n + 1, account.username);
        }

        let account_number = loop {
            print!("Enter account number (0 for no account): ");
            std::io::stdout().flush()?;
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            buf = buf.trim().to_string();

            if let Ok(n) = usize::from_str(&buf) {
                if n <= accounts.len() {
                    break n;
                } else {
                    println!("Account number must be between 0 and {}", accounts.len());
                }
            } else {
                println!("Please enter a valid number!");
            }
        };

        if account_number == 0 {
            Ok(None)
        } else {
            Ok(Some(accounts[account_number - 1].clone()))
        }
    }
}