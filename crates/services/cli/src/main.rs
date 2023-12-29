use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use hex;
use lib_network::packet::{self, PacketBuilder};
use lib_web::discovery;
use log::{debug, error, info, warn};
use tokio;

#[derive(Parser)]
#[command(name = "UDP2P-cli")]
#[command(author = "NIST team M2 MIC")]
#[command(version = "1.0")]
#[command(about = "
 _    _ _____  _____ ___  _____
| |  | |  __ \\|  __ \\__ \\|  __ \\
| |  | | |  | | |__) | ) | |__) |
| |  | | |  | |  ___/ / /|  ___/
| |__| | |__| | |    / /_| |
 \\____/|_____/|_|   |____|_|\n
P2P data exchange using UDP and NAT traversal.")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch the available peers.
    FetchPeers {
        /// Url of peers page
        #[arg(short = 'u', long)]
        host: String,
    },
    /// Fetch the infos of a peer
    FetchPeerInfo {
        /// Url of the peers server
        #[arg(short = 'u', long)]
        host: String,

        /// Peer name
        #[arg(short, long)]
        peer: String,
    },
    /// Download the datum associated with a hash
    GetDatum {
        /// Address of the peer
        #[arg(short, long)]
        peer: String,
        /// Datum hash
        #[arg(short, long)]
        datum: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::FetchPeers { host } => {
            log::info!("Fetching peers from central server {}.", host);
            let client = discovery::get_client(5)?;
            let url = discovery::parse_url(host)?;
            let peers = discovery::get_peers_names(client, &url).await?;
            println!("Peers :");
            for (i, peer) in peers.iter().enumerate() {
                println!("\t[{i}] = {peer}");
            }
        }
        Commands::FetchPeerInfo { host, peer } => {
            log::info!(
                "Fetching peer infos for peer {} from central server {}.",
                peer,
                host
            );
            let client = discovery::get_client(5)?;
            let url = discovery::parse_url(host)?;
            let peers = discovery::get_peer_addresses(client, &url, &peer).await?;
            println!("{peers:?}");
        }
        Commands::GetDatum { peer, datum } => {
            log::info!("Fetching datum from peer {} with hash {}.", peer, datum);
            let datum_bytes = match hex::decode(datum) {
                Ok(h) => h,
                Err(e) => bail!("Failed to decode root hash. Please check your input."),
            };
            let packet = PacketBuilder::new()
                .gen_id()
                .packet_type(packet::PacketType::GetDatum)
                .body(datum_bytes)
                .build()
                .unwrap_or_default();
            println!("{packet:#?}");
            println!("{:#?}", packet.as_bytes());
        }
        _ => {
            println!("No subcommand");
        }
    }
    return Ok(());
}
