use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use futures::{FutureExt as _, StreamExt};
use libp2p::{
    dcutr, identify, identity, noise, ping, relay,
    swarm::{NetworkBehaviour, SwarmEvent},
    tcp, yamux, Multiaddr, PeerId, Swarm,
};
use tracing::{info, warn};

#[derive(NetworkBehaviour)]
struct PatchLanBehavior {
    relay_client: relay::client::Behaviour,
    ping: ping::Behaviour,
    identify: identify::Behaviour,
    dcutr: dcutr::Behaviour,
}

pub struct PatchLan {
    swarm: Swarm<PatchLanBehavior>,
    relay_address: Multiaddr,
}

impl PatchLan {
    pub async fn connect(relay_address: Multiaddr, seed: u8) -> Result<PatchLan> {
        let keypair = generate_ed25519(seed);

        // Setup "swarm"
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(keypair)
            .with_tokio()
            .with_tcp(
                tcp::Config::default().nodelay(true),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_quic()
            .with_dns()?
            .with_relay_client(noise::Config::new, yamux::Config::default)?
            .with_behaviour(|keypair, relay_behaviour| PatchLanBehavior {
                relay_client: relay_behaviour,
                ping: ping::Behaviour::new(ping::Config::new()),
                identify: identify::Behaviour::new(identify::Config::new(
                    "/patchlan/0.0.1".to_string(),
                    keypair.public(),
                )),
                dcutr: dcutr::Behaviour::new(keypair.public().to_peer_id()),
            })?
            .build();

        swarm.listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse().unwrap())?;
        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())?;

        let mut delay = futures_timer::Delay::new(Duration::from_secs(1)).fuse();
        let mut listen_events_remaining = 2;

        // Wait for "new listen address" events for each interface
        loop {
            futures::select! {
                event = swarm.next() => {
                    match event.expect("None event found") {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            listen_events_remaining -= 1;
                            info!("Listening on: {address}");

                            if listen_events_remaining == 0 {
                                break
                            }
                        }
                        event => panic!("{event:?}"),
                    }
                },
                _ = delay => {
                    warn!("not all interfaces listened");
                    break
                }
            }
        }

        // Connect to relay
        swarm.dial(relay_address.clone())?;
        let mut learned_observed_addr = false;
        let mut tol_relay_observed_addr = false;

        loop {
            match swarm.next().await.ok_or(anyhow!("EOF"))? {
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Ping(_)) => {}
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Identify(identify::Event::Sent {
                    ..
                })) => {
                    info!("identity sent");
                    tol_relay_observed_addr = true;
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Identify(
                    identify::Event::Received {
                        info: identify::Info { observed_addr, .. },
                        ..
                    },
                )) => {
                    info!(address = %observed_addr, "got observed address from relay");
                    learned_observed_addr = true
                }
                SwarmEvent::ConnectionEstablished { .. } => {}
                SwarmEvent::NewListenAddr { .. } => {}
                SwarmEvent::Dialing { .. } => {}
                event => panic!("unexpected event: {event:?}"),
            }

            if learned_observed_addr && tol_relay_observed_addr {
                break;
            }
        }
        Ok(PatchLan {
            swarm,
            relay_address,
        })
    }

    pub async fn listen(&mut self) -> Result<()> {
        let swarm = &mut self.swarm;

        // Listen on relay interface
        swarm.listen_on(
            self.relay_address
                .clone()
                .with(libp2p::multiaddr::Protocol::P2pCircuit),
        )?;

        while let Some(event) = swarm.next().await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on: {address}");

                    // break;
                    dbg!(swarm.network_info());
                    dbg!(swarm.listeners().collect::<Vec<_>>());
                    dbg!(swarm.local_peer_id());
                    dbg!(swarm.external_addresses().collect::<Vec<_>>());
                    dbg!(swarm.connected_peers().collect::<Vec<_>>());
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. },
                )) => {
                    info!("Relay accepted our request");
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::RelayClient(event)) => {
                    info!(?event);
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Dcutr(event)) => {
                    info!(?event);
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Ping(_)) => {}
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    info!(?peer_id, ?endpoint, "Connection established");
                    dbg!(swarm.network_info());
                    dbg!(swarm.listeners().collect::<Vec<_>>());
                    dbg!(swarm.local_peer_id());
                    dbg!(swarm.external_addresses().collect::<Vec<_>>());
                    dbg!(swarm.connected_peers().collect::<Vec<_>>());
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    info!(?peer_id, "Connection Failed: {error}");
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub async fn ping(&mut self, peer: PeerId) -> Result<()> {
        let swarm = &mut self.swarm;

        swarm
            .dial(
                self.relay_address
                    .clone()
                    .with(libp2p::multiaddr::Protocol::P2pCircuit)
                    .with(libp2p::multiaddr::Protocol::P2p(peer)),
            )
            .context("dial relay")?;

        while let Some(event) = swarm.next().await {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on: {address}");

                    // break;
                    dbg!(swarm.network_info());
                    dbg!(swarm.listeners().collect::<Vec<_>>());
                    dbg!(swarm.local_peer_id());
                    dbg!(swarm.external_addresses().collect::<Vec<_>>());
                    dbg!(swarm.connected_peers().collect::<Vec<_>>());
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::RelayClient(
                    relay::client::Event::ReservationReqAccepted { .. },
                )) => {
                    info!("Relay accepted our request");
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::RelayClient(event)) => {
                    info!(?event);
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Dcutr(event)) => {
                    info!(?event);
                }
                SwarmEvent::Behaviour(PatchLanBehaviorEvent::Ping(_)) => {}
                SwarmEvent::ConnectionEstablished {
                    peer_id, endpoint, ..
                } => {
                    info!(?peer_id, ?endpoint, "Connection established");
                    dbg!(swarm.network_info());
                    dbg!(swarm.listeners().collect::<Vec<_>>());
                    dbg!(swarm.local_peer_id());
                    dbg!(swarm.external_addresses().collect::<Vec<_>>());
                    dbg!(swarm.connected_peers().collect::<Vec<_>>());
                }
                SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                    info!(?peer_id, "Connection Failed: {error}");
                }
                _ => {}
            }
        }

        // TODO: do some sort of application-level ping

        Ok(())
    }
}

fn generate_ed25519(secret_key_seed: u8) -> identity::Keypair {
    let mut bytes = [0u8; 32];
    bytes[0] = secret_key_seed;

    identity::Keypair::ed25519_from_bytes(bytes).expect("only errors on wrong length")
}
