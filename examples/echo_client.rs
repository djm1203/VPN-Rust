use anyhow::Result;
use tokio::net::UdpSocket;

#[tokio::main]
async fn main() -> Result<()> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect("127.0.0.1:8080").await?;

    let msg = b"ping from client";
    socket.send(msg).await?;
    println!("Send: {}", String::from_utf8_lossy(msg));

    let mut buf = [0u8; 1024];
    let len = socket.recv(&mut buf).await?;
    println!("Received: {}", String::from_utf8_lossy(&buf[..len]));

    Ok(())
}
