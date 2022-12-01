use clap::Parser;
use patchlan::add;

/// `patchlan` - A p2p mesh overlay network.
///
/// PatchLAN is a direct peer-to-peer mesh overlay network built on wireguard that allows you to
/// setup private virtual local networks over the internet with only a network key. Peer discovery,
/// key exchange, NAT traversal, and direct per-peer coneections are handled automatically.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about)]
struct Args {
    /// Make logging more verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Make logging less verbose
    #[arg(long, short, action = clap::ArgAction::Count)]
    quiet: u8,

    /// Network Key
    #[arg(long)]
    key: Option<String>,
}

fn main() {
    let _args = Args::parse();
    println!("1 + 1 = {}", add(1, 1));
}
