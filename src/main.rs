
use clap::{Parser, Subcommand};
// use crate::protocol::{Request, Response};

use dialoguer::Password;

use secretd::server;
use secretd::client;
use secretd::protocol::{Request, Response};



#[derive(Parser)]
#[command(name = "secretd")]
#[command(about = "Secure in-memory secret cache daemon", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Server {
        #[arg(long, default_value = "/tmp/secretd.sock")]
        socket: String,
        #[arg(long, default_value = "300")]
        ttl: u64,
    },
    Set {
        key: String,
    },
    Get {
        key: String,
    },
    Unlock,
    Lock,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let socket = "/tmp/secretd.sock";

    match cli.command {
        Commands::Server { socket, ttl } => {
            server::run_server(&socket, ttl, &xpassword("x")).await;
        }

        Commands::Set { key } => {
            match client::run_request(Request::Set { key, value: xpassword("x")}, socket).await {
                Ok(resp) => println!("{:?}", resp),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Commands::Get { key } => {
            match client::run_request(Request::Get { key }, socket).await {
                Ok(resp) => println!("{:?}", resp),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Commands::Unlock => {
            match client::run_request(Request::Unlock { password: xpassword("x") }, socket).await {
                Ok(resp) => println!("{:?}", resp),
                Err(e) => eprintln!("Error: {}", e),
            }
        }

        Commands::Lock => {
            match client::run_request(Request::Lock, socket).await {
                Ok(resp) => println!("{:?}", resp),
                Err(e) => eprintln!("Error: {}", e),
            }
        }
    }
}


fn xpassword(title: &str) -> String {
    Password::new()
        .with_prompt(title)
        .interact()
        .unwrap()
}
