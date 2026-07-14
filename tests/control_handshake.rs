//! Integration test: the versioned control handshake over a loopback QUIC pair.
//!
//! Verifies that client and server negotiate a common [`SessionParams`] set
//! (the smaller MTU / keepalive) before any data flows. Root-free (backlog
//! B-013 / M1).

use std::net::{Ipv4Addr, SocketAddr};

use vpn_rust::transport::control::{client_handshake, server_handshake};
use vpn_rust::transport::quic::{client_endpoint, dev_server_endpoint};
use vpn_rust::transport::SessionParams;

#[tokio::test]
async fn control_handshake_negotiates_params() -> anyhow::Result<()> {
    let bind: SocketAddr = (Ipv4Addr::LOCALHOST, 0).into();
    let (server_endpoint, server_cert) = dev_server_endpoint(bind)?;
    let server_addr = server_endpoint.local_addr()?;

    // Server offers a smaller MTU (1400) and larger keepalive (15s).
    let server_offer = SessionParams {
        mtu: 1400,
        keepalive_secs: 15,
    };
    let server = tokio::spawn(async move {
        let connection = server_endpoint
            .accept()
            .await
            .expect("incoming connection")
            .await?;
        let params = server_handshake(&connection, server_offer).await?;
        connection.closed().await;
        let _ = &server_endpoint;
        anyhow::Ok(params)
    });

    // Client requests a larger MTU (1500) and smaller keepalive (10s).
    let client_request = SessionParams {
        mtu: 1500,
        keepalive_secs: 10,
    };
    let endpoint = client_endpoint(server_cert)?;
    let connection = endpoint.connect(server_addr, "localhost")?.await?;
    let negotiated = client_handshake(&connection, client_request).await?;

    // Both peers should converge on the smaller values.
    let expected = SessionParams {
        mtu: 1400,
        keepalive_secs: 10,
    };
    assert_eq!(negotiated, expected, "client should see negotiated params");

    connection.close(0u32.into(), b"done");
    let server_params = server.await??;
    assert_eq!(
        server_params, expected,
        "server should agree on the same params"
    );
    Ok(())
}
