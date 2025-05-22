use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use vpn_rust::net::tls::start_tls_server;
use vpn_rust::net::tun::TunInterface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tun = Arc::new(Mutex::new(TunInterface::create()?));
    println!("Tun device created: {}", tun.lock().await.name);

    let listener = start_tls_server("122.0.0.1:4433").await?;

    loop {
        let (mut stream, _) = listener.accept().await?;
        let tun = Arc::clone(&tun);

        println!("[server] Client connected");

        tokio::spawn(async move {
            loop {
                let mut len_buf = [0u8; 2];
                if stream.read_exact(&mut len_buf).await.is_err() {
                    break;
                }

                let len = u16::from_be_bytes(len_buf) as usize;
                let mut buf = vec![0u8; len];
                if stream.read_exact(&mut buf).await.is_err() {
                    break;
                }

                println!("[server] Got {} bytes from client", len);
                let mut tun_guard = tun.lock().await;
                tun.lock().await.write_packet(&buf).await.unwrap();

                //then echo it back
                stream.write_all(&(len as u16).to_be_bytes()).await.unwrap();
                stream.write_all(&buf).await.unwrap();
            }
        });
    }
}
