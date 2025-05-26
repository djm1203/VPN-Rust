mod config;
mod net {
    pub mod tls;
    pub mod tun;
}

use config::OVPNConfig;
use net::tls::connect_tls;
use net::tun::TunInterface;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    ///setup the ovpn for certs and showing we could establish a connection
    //let config = OVPNConfig::from_file("client.ovpn")?;
    //println!("Loaded config: {:#?}", config);

    //let addr = format!("{}:{}", config.remote_addr, config.remote_port);
    //let _stream = connect_tls(&config.remote_addr, &addr).await?;
    //println!("TLS connection established!");

    ///setup the tun to read packets and echo them back
    let mut tun = TunInterface::create_client()?;
    tun.configure_client_ip()?;
    println!("TUN device created: {}", tun.name);

    loop {
        let packet = tun.read_packet().await?;
        println!("Got packet from TUN: {} bytes", packet.len());
        tun.write_packet(&packet).await?;
    }

    //Ok(())
}
