use tokio::io::{AsyncReadExt, AsyncWriteExt};
use vpn_rust::net::tls::connect_tls;
use vpn_rust::net::tun::TunInterface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut tun = TunInterface::create_client()?;
    tun.configure_client_ip()?;
    println!("Tun device created: {}", tun.name);

    let domain = "localhost";
    let addr = "127.0.0.1:4433";
    let mut tls_stream = connect_tls(domain, addr).await?;
    println!("Connected to server over TLS");

    loop {
        let packet = tun.read_packet().await?;
        println!("[client] Sending {} bytes to TLS", packet.len());
        tls_stream
            .write_all(&(packet.len() as u16).to_be_bytes())
            .await?;
        tls_stream.write_all(&packet).await?;

        let mut len_buf = [0u8; 2];
        tls_stream.read_exact(&mut len_buf).await?;
        let len = u16::from_be_bytes(len_buf) as usize;
        let mut buf = vec![0u8; len];
        tls_stream.read_exact(&mut buf).await?;
        println!("[client] Received {} bytes from TLS", len);
        tun.write_packet(&buf).await?;
    }

    #[allow(unreachable_code)]
    Ok(())
}
