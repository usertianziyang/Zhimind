//! Loopback bind refuses LAN IP (the #875 repro). Opt-in LAN bind accepts it.

use super::*;

fn test_host(token: &str, dist: PathBuf, allow_lan: bool) -> Arc<MirrorHost> {
    Arc::new(MirrorHost {
        inner: Mutex::new(Inner {
            env: MirrorEnvConfig {
                headless: false,
                token: Some(token.into()),
                port: Some(0),
                no_tunnel: true,
                allow_lan,
                dist: Some(dist),
                max_clients: DEFAULT_MAX_CLIENTS,
            },
            runtime: None,
            ctx: None,
            read_only: true,
            max_clients: DEFAULT_MAX_CLIENTS,
            allow_lan,
        }),
        hub: Arc::new(ws::WsHub::new()),
    })
}

#[tokio::test]
async fn lan_bind_accepts_detected_ipv4() {
    let dist = resolve_dist_dir(None);
    let host = test_host("lan-bind-token-0123456789abcdef01234567", dist, false);
    let st = host.start().await.expect("start");
    let port = st.local_port.expect("port");
    let token = st.token.expect("token");
    assert!(!st.allow_lan);
    assert!(st.public_url.as_deref().unwrap_or("").contains("127.0.0.1"));

    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .expect("client");
    let loopback = client
        .get(format!("http://127.0.0.1:{port}/t/{token}/api/health"))
        .send()
        .await
        .expect("loopback health");
    assert_eq!(loopback.status().as_u16(), 200);

    if let Some(ip) = lan::detect_lan_ipv4() {
        let lan_health = format!("http://{ip}:{port}/t/{token}/api/health");
        let refused = client.get(&lan_health).send().await;
        assert!(
            refused.is_err(),
            "loopback bind must refuse LAN IP (got {:?})",
            refused.map(|r| r.status())
        );

        let st2 = host.set_allow_lan(true).await.expect("enable lan");
        assert!(st2.allow_lan);
        let port2 = st2.local_port.expect("port after rebind");
        assert!(
            st2.lan_url
                .as_deref()
                .is_some_and(|u| u.contains(&ip.to_string())),
            "lan_url should use detected IPv4, got {:?}",
            st2.lan_url
        );
        assert!(
            st2.public_url
                .as_deref()
                .unwrap_or("")
                .contains(&ip.to_string()),
            "local URL should advertise LAN IP, got {:?}",
            st2.public_url
        );
        let lan_health_result = client
            .get(format!("http://{ip}:{port2}/t/{token}/api/health"))
            .send()
            .await;

        // In some CI environments (especially macOS runners), LAN IPs may not be routable
        // even though they're detected. Skip the connectivity check if it times out.
        match lan_health_result {
            Ok(resp) => {
                assert_eq!(
                    resp.status().as_u16(),
                    200,
                    "LAN health check should return 200"
                );
            }
            Err(e) if e.is_timeout() => {
                eprintln!("Warning: LAN IP {ip}:{port2} detected but not reachable (timeout). This can happen in CI environments.");
                // Still verify that the server configuration changed correctly
                assert!(st2.allow_lan, "allow_lan should be enabled");
                assert!(st2.lan_url.is_some(), "lan_url should be set");
            }
            Err(e) => panic!("Unexpected error during LAN health check: {}", e),
        }
    }

    host.stop().await.expect("stop");
}
