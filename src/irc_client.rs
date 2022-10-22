use crate::rotdb::RotDb;
use crate::line_parse::{ParsedLine, parse_line};

use std::time::Duration;
use tokio::sync::mpsc;
use tokio::net::TcpStream;

pub struct IrcClient {
    db: RotDb,
    remote_addr: String,
    nick: String,
    channels: Vec<String>,
    shutdown_recv: mpsc::Receiver<bool>,
}

impl IrcClient {
    pub fn new(filename: &str, remote_addr: &str, nick: &str) -> IrcClient {
        let (shutdown_send, shutdown_recv) = mpsc::channel(1);

        tokio::spawn(async move {
            match tokio::signal::ctrl_c().await {
                Ok(()) => {},
                Err(err) => {
                    eprintln!("Failed to wait for Ctrl+C signal: {}", err);
                }
            }
            let _ = shutdown_send.send(true).await;
        });

        IrcClient {
            db: RotDb::new(filename),
            remote_addr: remote_addr.to_string(),
            nick: nick.to_string(),
            channels: Vec::new(),
            shutdown_recv,
        }
    }

    pub fn join(&mut self, channel: &str) {
        self.channels.push(channel.to_string());
    }

    pub async fn run(&mut self) {
        self.connect().await;

        let mut save_timer = tokio::time::interval(Duration::from_secs(15 * 60));
        save_timer.tick().await;    // The first tick comes immediately

        loop {
            tokio::select! {
                _ = save_timer.tick() => self.db.sync(),
                _ = self.shutdown_recv.recv() => break,
            }
        }

        println!("Would disconnect cleanly");
    }

    async fn connect(&self) {
        // TODO
        println!("Would connect to {}", self.remote_addr);
        println!("Would set IDENT to {0} . . {0}", self.nick);
        for chan in &self.channels {
            println!("Would join #{}", chan);
        }
    }
}
