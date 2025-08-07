
use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use clap::{Parser, Subcommand};
// use rand::Rng;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
#[command(name = "secretd")]
#[command(about = "Secure in-memory secret store",
          long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the secret server
    Server {
        /// Path to Unix socket
        #[arg(short, long, default_value = "/tmp/secretd.sock")]
        socket: String,
    },
    /// Set a secret
    Set {
        key: String,
        value: String,
        #[arg(short, long)]
        password: String,
    },
    /// Get a secret
    Get {
        key: String,
        #[arg(short, long)]
        password: String,
    },
    /// Unlock the store
    Unlock {
        #[arg(short, long)]
        password: String,
    },
    /// Lock the store
    Lock,
}

#[derive(Serialize, Deserialize, Debug)]
enum Request {
    Set { key: String, value: String, password: String },
    Get { key: String, password: String },
    Unlock { password: String },
    Lock,
}

#[derive(Serialize, Deserialize, Debug)]
enum Response {
    Ok(Option<String>),
    Err(String),
}

struct StoreEntry {
    value: String,
    expiry: Instant,
}

struct SecretStore {
    secrets: HashMap<String, StoreEntry>,
    unlocked: bool,
    master_password: Option<String>,
    ttl: Duration,
}

impl SecretStore {
    fn new(ttl_secs: u64) -> Self {
        Self {
            secrets: HashMap::new(),
            unlocked: false,
            master_password: None,
            ttl: Duration::new(ttl_secs, 0),
        }
    }

    fn unlock(&mut self, password: &str) -> bool {
        if let Some(ref pw) = self.master_password {
            self.unlocked = pw == password;
        } else {
            self.master_password = Some(password.to_string());
            self.unlocked = true;
        }
        self.unlocked
    }

    fn lock(&mut self) {
        self.unlocked = false;
    }

    fn set(&mut self, key: String, value: String, password: String) -> Result<(), String> {
        if !self.unlocked || self.master_password.as_ref() != Some(&password) {
            return Err("Unauthorized".into());
        }
        let entry = StoreEntry {
            value,
            expiry: Instant::now() + self.ttl,
        };
        self.secrets.insert(key, entry);
        Ok(())
    }

    fn get(&mut self, key: String, password: String) -> Result<Option<String>, String> {
        if !self.unlocked || self.master_password.as_ref() != Some(&password) {
            return Err("Unauthorized".into());
        }
        if let Some(entry) = self.secrets.get(&key) {
            if entry.expiry > Instant::now() {
                return Ok(Some(entry.value.clone()));
            }
        }
        Ok(None)
    }
}

fn handle_client(mut stream: UnixStream, store: Arc<Mutex<SecretStore>>) {
    let mut buf = Vec::new();
    if stream.read_to_end(&mut buf).is_ok() {
        if let Ok(req) = bincode::deserialize::<Request>(&buf) {
            let mut store = store.lock().unwrap();
            let resp = match req {
                Request::Set { key, value, password } => match store.set(key, value, password) {
                    Ok(_) => Response::Ok(None),
                    Err(e) => Response::Err(e),
                },
                Request::Get { key, password } => match store.get(key, password) {
                    Ok(val) => Response::Ok(val),
                    Err(e) => Response::Err(e),
                },
                Request::Unlock { password } => {
                    if store.unlock(&password) {
                        Response::Ok(None)
                    } else {
                        Response::Err("Invalid password".into())
                    }
                }
                Request::Lock => {
                    store.lock();
                    Response::Ok(None)
                }
            };
            let encoded = bincode::serialize(&resp).unwrap();
            let _ = stream.write_all(&encoded);
        }
    }
}

fn start_server(socket_path: &str) {
    let _ = fs::remove_file(socket_path);
    let listener = UnixListener::bind(socket_path).expect("Failed to bind socket");
    println!("Server listening on {}", socket_path);

    let store = Arc::new(Mutex::new(SecretStore::new(300)));

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let store = Arc::clone(&store);
                thread::spawn(move || {
                    handle_client(stream, store);
                });
            }
            Err(err) => eprintln!("Connection failed: {}", err),
        }
    }
}

fn send_request(req: Request, socket_path: &str) {
    if let Ok(mut stream) = UnixStream::connect(socket_path) {
        let data = bincode::serialize(&req).unwrap();
        if stream.write_all(&data).is_ok() {
            let mut resp_buf = Vec::new();
            if stream.read_to_end(&mut resp_buf).is_ok() {
                if let Ok(resp) = bincode::deserialize::<Response>(&resp_buf) {
                    match resp {
                        Response::Ok(Some(val)) => println!("{}", val),
                        Response::Ok(None) => println!("OK"),
                        Response::Err(e) => eprintln!("Error: {}", e),
                    }
                }
            }
        }
    } else {
        eprintln!("Failed to connect to server");
    }
}

fn main() {
    let cli = Cli::parse();
    let socket = "/tmp/secretd.sock";

    match cli.command {
        Commands::Server { socket } => start_server(&socket),
        Commands::Set { key, value, password } => {
            send_request(Request::Set { key, value, password }, socket);
        }
        Commands::Get { key, password } => {
            send_request(Request::Get { key, password }, socket);
        }
        Commands::Unlock { password } => {
            send_request(Request::Unlock { password }, socket);
        }
        Commands::Lock => {
            send_request(Request::Lock, socket);
        }
    }
}
