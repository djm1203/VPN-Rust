use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vpn_rust::net::tun::TunInterface;
use vpn_rust::net::tls::connect_tls;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let tun = TunInterface::create()?;
    println!("Tun device created: {}", tun.name);
    
    let domain = "localhost";
    let addr = "127.0.0.1:4433";
    let mut tls_stream = connect_tls(domain, addr).await?;
    println!("Connected to server over TLS");

    // forward from tun to tls
    let forward_task = tokio::spawn(async move {
        loop {
            let packet = read_tun.read_packet().await().unwrap();
            println!("[client] Sending {} bytes to TLS", packet.len());
            
            if let Err(e) = tls_write.write_all(&(packet.len() as u16).to_be_bytes()).await {
                eprintln!("write length failed: {e}");
                break;
            }
            if let Err(e) = tls_write.write_all(&packet).await {
                eprintln!("write data failed: {e}");
                break;
            }
        }
    });

    // read tls to tun
    loop {
        let mut len_buf = [0u8; 2];
        tls_read.read_exact(&mut len_buf).await?;
        let len = u16::from_be_bytes(let_buf) as usize;

        let mut buf = vec![0u8; len];
        tls_read.read_exact(&mut len_buf).await?;
        println!("[client] Received {} bytes from TLS", len);
        read_tun.write_packet(&buf).await?;
    }

    forward_task.await?;
    Ok()
}
