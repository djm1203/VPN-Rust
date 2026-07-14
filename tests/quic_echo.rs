//! Integration test: a QUIC datagram round-trips between two loopback peers.
//!
//! This exercises the [`Transport`] seam and the `quinn` implementation without
//! a TUN device or root privileges (milestone M1 spike / backlog B-005).

use std::net::{Ipv4Addr, SocketAddr};

use bytes::Bytes;
use vpn_rust::transport::quic::{dev_client_endpoint, dev_server_endpoint};
use vpn_rust::transport::{QuicTransport, Transport};

#[tokio::test]
async fn quic_datagram_round_trip() -> anyhow::Result<()> {
    // Bind a server on an OS-assigned loopback port.
    let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
    let (server_endpoint, server_cert) = dev_server_endpoint(bind)?;
    let server_addr = server_endpoint.local_addr()?;

    // Server task: accept one connection and echo a single datagram back.
    let server = tokio::spawn(async move {
        let incoming = server_endpoint
            .accept()
            .await
            .expect("server should receive an incoming connection");
        let connection = incoming.await?;
        let transport = QuicTransport::new(connection);

        let datagram = transport.recv_datagram().await?;
        transport.send_datagram(datagram).await?;

        // Keep the endpoint alive until the client closes the connection (which
        // it does once it has received the echo), then shut down promptly.
        transport.connection().closed().await;
        let _ = &server_endpoint;
        anyhow::Ok(())
    });

    // Client: connect, send a datagram, expect the same bytes echoed back.
    let client_endpoint = dev_client_endpoint(server_cert)?;
    let connection = client_endpoint.connect(server_addr, "localhost")?.await?;
    let client = QuicTransport::new(connection);

    let payload = Bytes::from_static(b"hello quic vpn");
    client.send_datagram(payload.clone()).await?;
    let echoed = client.recv_datagram().await?;

    assert_eq!(
        echoed, payload,
        "echoed datagram should match what was sent"
    );
    assert!(
        client.max_datagram_size().is_some(),
        "peer should advertise a datagram size after the handshake"
    );

    // Close the connection so the server task can shut down without waiting for
    // the idle timeout.
    client.connection().close(0u32.into(), b"done");

    server.await??;
    Ok(())
}
