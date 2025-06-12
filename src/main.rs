use anyhow::Result;
use clap::Parser;
use libp2p::{Multiaddr, PeerId};

/// `patchlan` - A p2p mesh overlay network.
///
/// PatchLAN is a direct peer-to-peer mesh overlay network built on wireguard that allows you to
/// setup private virtual local networks over the internet with only a network key. Peer discovery,
/// key exchange, NAT traversal, and direct per-peer coneections are handled automatically.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    #[command(flatten)]
    global_opts: GlobalOpts,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(clap::Args, Debug, Clone)]
struct GlobalOpts {
    /// Make logging more verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Make logging less verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    quiet: u8,

    /// Network Key
    #[arg(long)]
    key: Option<String>,

    /// Relay Server
    relay: Multiaddr,

    seed: u8,
}

#[derive(clap::Subcommand, Debug, Clone)]
#[command()]
enum Command {
    Listen(Listen),
    Ping(Ping),
    AddRelay(AddRelay),
    Init(Init),
    Status(Status),
}

#[derive(clap::Args, Debug, Clone)]
struct Init {}

#[derive(clap::Args, Debug, Clone)]
struct Status {}

#[derive(clap::Args, Debug, Clone)]
struct Listen {}

#[derive(clap::Args, Debug, Clone)]
struct Ping {
    peer_id: PeerId,
}

#[derive(clap::Args, Debug, Clone)]
struct AddRelay {
    relay: Multiaddr,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt::init();
    tracing::info!("Starting...");

    let mut pl = patchlan::PatchLan::connect(args.global_opts.relay, args.global_opts.seed).await?;

    match args.command {
        None => pl.listen().await,
        Some(Command::Listen(_)) => pl.listen().await,
        Some(Command::Ping(ping_args)) => pl.ping(ping_args.peer_id).await,
        Some(_) => todo!("command not yet implemented"),
    }
}
