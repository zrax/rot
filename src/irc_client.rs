use crate::rotdb::RotDb;
use crate::line_parse::{ParsedLine, parse_line};

use tokio::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[derive(PartialEq)]
enum PingState {
    Reset,
    Waiting,
    PingPending,
}

pub struct IrcClient {
    db: RotDb,
    remote_addr: String,
    nick: String,
    channels: Vec<String>,
    shutdown_recv: mpsc::Receiver<bool>,
    ping_state: PingState,
}

const DB_SAVE_INTERVAL: Duration = Duration::from_secs(15 * 60);
const PING_INTERVAL: Duration = Duration::from_secs(5 * 60);
const TIMEOUT_DURATION: Duration = Duration::from_secs(60);

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
            ping_state: PingState::Reset,
        }
    }

    pub fn join(&mut self, channel: &str) {
        self.channels.push(channel.to_string());
    }

    pub async fn run(&mut self) {
        let mut save_timer = tokio::time::interval(DB_SAVE_INTERVAL);
        save_timer.tick().await;    // The first tick comes immediately

        let mut sock = match self.connect(false).await {
            Some(sock) => sock,
            None => return,
        };

        let ping_timer = tokio::time::sleep(PING_INTERVAL);
        tokio::pin!(ping_timer);

        let mut chunk = Vec::<u8>::new();
        let mut buf = [0; 1024];
        loop {
            if self.ping_state == PingState::Reset {
                ping_timer.as_mut().reset(Instant::now() + PING_INTERVAL);
                self.ping_state = PingState::Waiting;
            }

            tokio::select! {
                result = sock.read(&mut buf) => match result {
                    Ok(0) => {
                        eprintln!("Server closed the connection");
                        sock = match self.connect(true).await {
                            Some(sock) => sock,
                            None => return,
                        };
                    }
                    Ok(n) => {
                        chunk.extend(&buf[0..n]);
                        chunk = self.process_lines(&chunk, &mut sock);
                    }
                    Err(err) => {
                        eprintln!("Failed to read from server: {}", err);
                        sock = match self.connect(true).await {
                            Some(sock) => sock,
                            None => return,
                        };
                    }
                },
                _ = &mut ping_timer => match self.ping_state {
                    PingState::Reset => unreachable!(),
                    PingState::Waiting => {
                        let _ = sock.write_all(b"PING :rot\r\n").await;
                        self.ping_state = PingState::PingPending;
                        ping_timer.as_mut().reset(Instant::now() + TIMEOUT_DURATION);
                    }
                    PingState::PingPending => {
                        eprintln!("No PING response from server");
                        sock = match self.connect(true).await {
                            Some(sock) => sock,
                            None => return,
                        };
                    }
                },
                _ = save_timer.tick() => self.db.sync(),
                _ = self.shutdown_recv.recv() => break,
            }
        }

        // Still connected, so try to perform a graceful departure
        let _ = sock.write_all(b"QUIT :--rot!\r\n").await;
    }

    fn process_lines(&mut self, mut chunk: &[u8], sock: &mut TcpStream) -> Vec<u8> {
        while let Some(pos) = chunk.iter().position(|c| *c == b'\n') {
            let parts = irc_split(&chunk[0..pos]);
            chunk = &chunk[pos + 1..];

            if parts.len() >= 2 && parts[0] == "PING" {
                let _ = sock.write_all(format!("PONG {}\r\n", parts[1]).as_bytes());
            } else if parts.len() >= 2 && parts[1] == "PONG" {
                // The timer itself will be reset by the event loop.
                self.ping_state = PingState::Reset;
            } else if parts.len() >= 3 && parts[1] == "PRIVMSG" {
                // TODO
                println!("Would process PRIVMSG");
            }
        }
        // Return the remainder for the next call
        chunk.to_owned()
    }

    async fn reconnect_delay(&mut self) -> bool {
        eprintln!("Retrying in 60 sec...");
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(60)) => true,
            _ = self.shutdown_recv.recv() => false,
        }
    }

    async fn connect(&mut self, initial_delay: bool) -> Option<TcpStream> {
        if initial_delay && !self.reconnect_delay().await {
            return None;
        }

        let mut sock = loop {
            let connect_fut = TcpStream::connect(&self.remote_addr);
            match tokio::time::timeout(TIMEOUT_DURATION, connect_fut).await {
                Ok(Ok(sock)) => break sock,
                Ok(Err(err)) => {
                    eprintln!("Failed to connect to {}: {}", self.remote_addr, err);
                }
                Err(_) => eprintln!("Connection timed out"),
            };

            if !self.reconnect_delay().await {
                return None;
            }
        };

        let peer_name = match sock.peer_addr() {
            Ok(addr) => addr.to_string(),
            Err(_) => "<unknown>".to_string(),
        };
        println!("Connected to {}", peer_name);

        // Minimal identification necessary to satisfy the IRC server
        let _ = sock.write_all(
                    format!("NICK {0}\r\n\
                             USER {0} . . :{0}\r\n", self.nick).as_bytes()
                ).await;

        // Join the requested IRC channel(s)
        for chan in &self.channels {
            let _ = sock.write_all(format!("JOIN #{}\r\n", chan).as_bytes()).await;
        }

        // Signal reset of the ping timer
        self.ping_state = PingState::Reset;

        // If we lost the connection during the writes above, we'll catch it
        // when we try to read from the socket in the main loop.
        Some(sock)
    }
}

fn irc_split(mut line: &[u8]) -> Vec<String> {
    let mut parts = vec![];
    let mut scan = 0;

    while scan < line.len() {
        if line[scan].is_ascii_whitespace() {
            parts.push(String::from_utf8_lossy(&line[0..scan]).to_string());
            while scan < line.len() && line[scan].is_ascii_whitespace() {
                scan += 1;
            }
            line = &line[scan..];
            scan = 0;
            if line.starts_with(b":") {
                parts.push(String::from_utf8_lossy(&line).to_string());
                break;
            }
        } else {
            scan += 1;
        }
    }
    if scan != 0 {
        parts.push(String::from_utf8_lossy(&line).to_string());
    }

    parts
}
