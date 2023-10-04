#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::uninlined_format_args)]    // Added in Rust 1.66

mod rotdb;
mod line_parse;
mod irc_client;

use std::env;
use irc_client::IrcClient;

#[tokio::main(flavor="current_thread")]
async fn main() {
    let mut argp = env::args();
    let self_exe = argp.next();
    if argp.len() < 2 {
        eprintln!("Usage: {} hostname:port nick [channel [...]]",
                  self_exe.unwrap_or_else(|| "<Unknown>".to_string()));
        std::process::exit(1);
    }

    let remote_addr = argp.next().unwrap();
    let nick = argp.next().unwrap();

    let mut client = IrcClient::new("zot.db", &remote_addr, &nick);
    for channel in argp {
        client.join(&channel);
    }

    client.run().await;
}
