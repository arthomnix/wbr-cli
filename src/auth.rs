use std::io::Write;
use std::str::FromStr;
use log::debug;
use crate::read_yes_no_prompt;

const USER: &str = "https://xrrlbpmfxuxumxqbccxz.supabase.co/auth/v1/user";
const SUPABASE_KEY: &str = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpc3MiOiJzdXBhYmFzZSIsInJlZiI6InhycmxicG1meHV4dW14cWJjY3h6Iiwicm9sZSI6ImFub24iLCJpYXQiOjE2OTIyMzc2NTAsImV4cCI6MjAwNzgxMzY1MH0.8Xae0-VrRVKTGmMSJt2o0WGL6Q5NXgWdAyASsXEjv4E";
pub(crate) const AUTH_COOKIE_NAME: &str = "sb-xrrlbpmfxuxumxqbccxz-auth-token";


#[derive(serde::Deserialize, Clone, Debug)]
struct SupabaseCustomClaims {
    global_name: String,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct SupabaseUserMetadata {
    custom_claims: SupabaseCustomClaims,
}

#[derive(serde::Deserialize, Clone, Debug)]
struct SupabaseUserResponse {
    id: String,
    role: String,
    user_metadata: SupabaseUserMetadata,
}

#[derive(Clone, Debug)]
pub(crate) struct AuthInfo {
    pub(crate) username: String,
    pub(crate) user_id: String,
    pub(crate) auth_cookie: String,
}

pub(crate) fn get_session_cookies(client: &reqwest::blocking::Client) -> Vec<AuthInfo> {
    rookie::load(Some(vec!["www.whatbeatsrock.com".to_string()]))
        .unwrap()
        .into_iter()
        .filter_map(|cookie| {
            debug!("found cookie {cookie:?}");
            if cookie.name != AUTH_COOKIE_NAME {
                return None;
            }

            let decoded = urlencoding::decode(&cookie.value).unwrap();
            debug!("{decoded}");
            let token_parts = serde_json::from_str::<Vec<Option<String>>>(&decoded).unwrap();
            let user_info = match client.get(USER)
                .header("apikey", SUPABASE_KEY)
                .bearer_auth(token_parts[0].as_ref().unwrap())
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

            if &user_info.role != "authenticated" {
                debug!("user not authenticated");
                None
            } else {
                debug!("found user id {} username {}", user_info.id, user_info.user_metadata.custom_claims.global_name);
                Some(AuthInfo {
                    username: user_info.user_metadata.custom_claims.global_name,
                    user_id: user_info.id,
                    auth_cookie: cookie.value,
                })
            }
        })
        .collect::<Vec<AuthInfo>>()
}

pub(crate) fn auth_prompt(accounts: Vec<AuthInfo>) -> Option<AuthInfo> {
    if accounts.len() == 0 {
        return None;
    }

    if accounts.len() == 1 {
        println!("Found logged in account: @{}", accounts[0].username);
        print!("Use this account? [Y/n]");
        if read_yes_no_prompt() {
            Some(accounts[0].clone())
        } else {
            None
        }
    } else {
        println!("Found multiple logged in accounts:");
        for (n, account) in accounts.iter().enumerate() {
            println!("[{}]: @{}", n + 1, account.username);
        }

        let account_number = loop {
            print!("Enter account number (0 for no account): ");
            std::io::stdout().flush().unwrap();
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf).unwrap();
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
            None
        } else {
            Some(accounts[account_number - 1].clone())
        }
    }
}