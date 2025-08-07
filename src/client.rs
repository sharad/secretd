
use tokio::net::UnixStream;
use serde::{Serialize, Deserialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::protocol::{Request, Response};

pub async fn run_request(request: Request, socket: &str) -> Result<Response, Box<dyn std::error::Error>> {
    let mut stream = UnixStream::connect(socket).await?;
    // Serialize and send the request
    // let res_data = serde_json::to_vec(&res).expect("Serialize failed");
    let request_bytes = bincode::serialize(&request)?;
    stream.write_all(&request_bytes).await?;
    stream.shutdown().await?; // close write side so server reads EOF
    // Read the response
    let mut response_bytes = Vec::new();
    stream.read_to_end(&mut response_bytes).await?;
    // Deserialize response
    // let req: Request = match serde_json::from_slice(&buf[..n])
    let response: Response = bincode::deserialize(&response_bytes)?;
    Ok(response)
}
