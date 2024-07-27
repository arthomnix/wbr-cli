# wbr-cli

A command line client for [What Beats Rock](https://www.whatbeatsrock.com)

## Installation
wbr-cli requires that [Rust](https://www.rust-lang.org/learn/get-started) and Git are installed.

To install wbr-cli, run the following command: `cargo install --git https://github.com/arthomnix/wbr-cli.git`

## Usage
To play a normal game, run `wbr`. To play a custom game, run `wbr -c <username>`, specifying the username of the user
whose custom game you want to play.

wbr-cli supports playing with an account by reading the authentication cookie from your browser. To play with an account,
log in to What Beats Rock in a browser. If this doesn't work, try closing all browser windows to force the browser to
save its cookies to disk.

If you want to exit a game, type `EXIT` (must be in all caps). This will offer you the option to save the game - if you
say yes, it will pick up from where you left off the next time you start `wbr`. If you want to guess the word `EXIT`, do
it in lowercase (WBR guesses are not case-sensitive).