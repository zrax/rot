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
                  self_exe.unwrap_or("<Unknown>".to_string()));
        std::process::exit(1);
    }

    let remote_addr = argp.next().unwrap();
    let nick = argp.next().unwrap();

    let mut client = IrcClient::new("zot.db", &remote_addr, &nick);
    while let Some(channel) = argp.next() {
        client.join(&channel);
    }

    client.run().await;
}
