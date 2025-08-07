


use tokio::net::UnixListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::sync::Arc;
use crate::store::SecretStore;
use crate::protocol::{Request, Response};


async fn handle_request(req: Request, store: &SecretStore) -> Response {
    match req {
        Request::Unlock { password } => {
            // if store.check_password(&password) {
            //     Response::Ok(None)
            // } else {
            //     Response::Error("Invalid password".to_string())
            // }
            Response::Ok(None)
        }
        Request::Set { key, value } => {
            store.set(key, value).await;
            Response::Ok(None)
        }
        Request::Get { key } => {
            match store.get(&key).await {
                Some(value) => Response::Ok(Some(value)),
                None => Response::Error("Key not found or expired".into()),
            }
        }
        Request::Lock => {
            // store.clear().await;
            Response::Ok(None)
        }
    }
}


pub async fn run_server(socket_path: &str, ttl: u64, password: &str) -> std::io::Result<()> {

    let store = SecretStore::new(password, ttl);

    if std::fs::remove_file(socket_path).is_ok() {
        println!("Removed stale socket at {}", socket_path);
    }

    let listener = UnixListener::bind(socket_path)?;

    println!("Server listening on {}", socket_path);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let store = Arc::clone(&store);

        tokio::spawn(async move {
            let mut buf = vec![0; 4096];
            let n = match socket.read(&mut buf).await {
                Ok(n) if n == 0 => return,
                Ok(n) => n,
                Err(e) => {
                    eprintln!("Failed to read socket: {:?}", e);
                    return;
                }
            };

            // let req: Request = match serde_json::from_slice(&buf[..n]) {
            let req: Request = match bincode::deserialize(&buf[..n]) {
                Ok(req) => req,
                Err(e) => {
                    eprintln!("Failed to parse request: {:?}", e);
                    return;
                }
            };

            let res = handle_request(req, &store).await;
            // let res_data = serde_json::to_vec(&res).expect("Serialize failed");
            let res_data = bincode::serialize(&res).expect("Serialize failed");
            
            if let Err(e) = socket.write_all(&res_data).await {
                eprintln!("Failed to write response: {:?}", e);
            }
        });
    }
}
