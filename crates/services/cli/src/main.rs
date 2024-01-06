use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use hex;
use lib_network::{
    action::*,
    congestion_handler::*,
    import_export::{download_from, handshake, peek_until_root_reply_from},
    peer::*,
    store::*,
    task_launcher_canceller::*,
};
use lib_web::discovery;
use log::{error, info};
use owo_colors::OwoColorize;
use std::{fs::File, io::Write, net::SocketAddr, sync::Arc};
use tokio::{self, net::UdpSocket};

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
    Peers {
        /// Url of peers page
        #[arg(short = 'u', long)]
        host: String,
    },
    /// Download a file from a hash
    Download {
        /// Address of the peer
        #[arg(short, long)]
        peer: String,
        /// Datum hash
        #[arg(short, long)]
        datum: Option<String>,
        /// Output path
        /// Default value is ./dump
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 100)]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::Peers { host } => {
            info!("Fetching peers from central server {}.", host);
            let client = discovery::get_client(5)?;
            let url = discovery::parse_url(host)?;
            let peers = discovery::get_peers_names(&client, &url).await?;
            println!("Available peers :");
            for (_i, peer) in peers.iter().enumerate() {
                let addr: discovery::Peer;
                match discovery::get_peer_addresses(&client, &url, &peer).await {
                    Ok(a) => addr = a,
                    Err(_) => continue,
                }

                let root = discovery::get_peer_root(&client, &url, peer).await?;
                let key = discovery::get_peer_key(&client, &url, peer).await?;
                println!("\n\u{1f4c7} {}", peer.green());
                println!("  \u{1f4e9} Addresses :");
                for a in addr.addresses.into_iter() {
                    println!("    - {a}");
                }
                match &root.len() {
                    0 => (),
                    _ => println!("  \u{1f517} Root : {}", hex::encode(root)),
                }
                match &key.len() {
                    0 => (),
                    _ => println!("  \u{1f511} Key  : {}", hex::encode(key)),
                }
            }
        }
        Commands::Download {
            peer,
            datum,
            output,
        } => {
            let peer_hash: Option<[u8; 32]> = match datum {
                Some(d) => {
                    info!("Fetching content from peer {} for hash {}.", peer, d);
                    println!("Fetching content from peer {} for hash {}.", peer, d);
                    match hex::decode(d) {
                        Ok(h) => match <[u8; 32]>::try_from(h) {
                            Ok(i) => Some(i),
                            Err(_e) => {
                                error!("Invalid root hash. Please check your input.");
                                bail!("Invalid root hash. Please check your input.");
                            }
                        },
                        Err(_e) => {
                            error!("Failed to decode root hash. Please check your input.");
                            bail!("Failed to decode root hash. Please check your input.")
                        }
                    }
                }
                None => {
                    info!("Fetching content from peer {} from root hash.", peer);
                    println!("Fetching content from peer {} from root hash.", peer);
                    None
                }
            };

            let addr4 = UdpSocket::bind("0.0.0.0:40000").await;
            let sock4: Arc<UdpSocket>;
            match addr4 {
                Ok(a) => sock4 = Arc::new(a),
                Err(e) => {
                    error!("Failed to bind IPv4 address : {e}");
                    bail!("Failed to bind IPv4 address : {e}")
                }
            }
            let addr6 = UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000)).await;
            let sock6: Arc<UdpSocket>;
            match addr6 {
                Ok(a) => sock6 = Arc::new(a),
                Err(e) => {
                    error!("Failed to bind IPv6 address : {e}");
                    bail!("Failed to bind IPv6 address : {e}")
                }
            }

            let server_sock_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
            let maps = build_tree_mutex();
            let queues = build_queues();
            let active_peers = ActivePeers::build_mutex();

            let (
                _receive_queue,
                _send_queue,
                action_queue,
                process_queue,
                _pending_ids,
                _receive_queue_state,
                action_queue_state,
                _send_queue_state,
                process_queue_state,
                process_queue_readers_state,
            ) = (
                Arc::clone(&queues.0),
                Arc::clone(&queues.1),
                Arc::clone(&queues.2),
                Arc::clone(&queues.3),
                Arc::clone(&queues.4),
                Arc::clone(&queues.5),
                Arc::clone(&queues.6),
                Arc::clone(&queues.7),
                Arc::clone(&queues.8),
                Arc::clone(&queues.9),
            );

            let mut my_data = Peer::new();
            my_data.set_name("nist".to_string());
            let my_data = Arc::new(my_data);

            task_launcher(
                queues,
                active_peers.clone(),
                my_data.clone(),
                sock4.clone(),
                sock6.clone(),
            );

            let sock_addr: SocketAddr;
            match peer.parse() {
                Ok(s) => sock_addr = s,
                Err(e) => {
                    error!("Invalid peer address {e}.");
                    bail!("Invalid peer address {e}.")
                }
            }

            handshake(
                process_queue.clone(),
                process_queue_readers_state.clone(),
                action_queue.clone(),
                action_queue_state.clone(),
                server_sock_addr4,
                my_data.clone(),
            );

            info!("Contacting address {}", sock_addr.to_string());

            handshake(
                process_queue.clone(),
                process_queue_readers_state.clone(),
                action_queue.clone(),
                action_queue_state.clone(),
                sock_addr,
                my_data.clone(),
            );

            let peer_hash = match peer_hash {
                Some(hash) => Some(hash),
                None => match peek_until_root_reply_from(
                    process_queue.clone(),
                    process_queue_state.clone(),
                    process_queue_readers_state.clone(),
                    action_queue.clone(),
                    action_queue_state.clone(),
                    sock_addr,
                    100000,
                )
                .await
                {
                    Ok(Action::ProcessRootReply(hash, _)) => hash,
                    Err(PeerError::ResponseTimeout) => bail!("Couldn't fetch peer root"),
                    _ => bail!("Unexpected error"),
                },
            };

            let peer_hash = match peer_hash {
                Some(h) => h,
                None => {
                    println!("{}", "Peer is not exporting any file.".red());
                    error!("{}", "Peer is not exporting any file.".red());
                    std::process::exit(0);
                }
            };

            info!("Selected peer hash is {}", hex::encode(&peer_hash));

            let content = download_from(
                Arc::clone(&process_queue),
                Arc::clone(&process_queue_readers_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                Arc::clone(&maps),
                peer_hash,
                sock_addr,
                100000,
            )
            .await;

            match content {
                Ok(node) => {
                    let path = match output {
                        Some(s) => s.to_string(),
                        None => {
                            info!("No output file provided, defaulting to ./dump");
                            "./dump".to_string()
                        }
                    };

                    log::info!(
                        "Saving file from peer {} for hash {}.",
                        peer,
                        hex::encode(&peer_hash)
                    );
                    println!(
                        "Saving file from peer {} for hash {}.",
                        peer,
                        hex::encode(&peer_hash)
                    );

                    let mut file = match File::create(&path) {
                        Ok(f) => f,
                        Err(e) => {
                            error!("Could not create or open file {} : {}", &path, e);
                            std::process::exit(0);
                        }
                    };

                    let content = node.flatten();
                    println!("Size of file {}", &content.len());
                    match file.write(&content) {
                        Ok(size) => println!("Download completed. Wrote {} bytes", size),
                        Err(e) => {
                            println!("Failed to save file {e}");
                            error!("Failed to save file {e}");
                            std::process::exit(0);
                        }
                    }
                }
                Err(PeerError::FileIsDirectory) => {
                    match maps.lock() {
                        Ok(m) => {
                            println!("\nFile tree :");
                            let n_to_h_hashmap = get_name_to_hash_hashmap(&m.0, &m.2);
                            let mut names: Vec<&String> = n_to_h_hashmap.keys().collect();
                            names.sort();
                            for n in names.into_iter() {
                                let mut step = n.chars().filter(|ch| *ch == '/').count();
                                if step > 0 {
                                    step -= 1;
                                }
                                let carry = str::repeat("   ", step);
                                println!("{carry}└──\u{1f4c4} {n}");
                                println!(
                                    "   {carry} {}",
                                    hex::encode(
                                        n_to_h_hashmap
                                            .get(n)
                                            .expect("Failed because of unknown error [code 31]")
                                    )
                                );
                            }
                        }
                        Err(e) => {
                            error!("[Code 32] Download failed with error {e}");
                            std::process::exit(0);
                            // bail!("Download failed with error {e}")
                        }
                    };
                }
                Err(e) => {
                    error!("[Code 33] Download failed with error {e}");
                    std::process::exit(0);
                }
            }
        }
    }
    std::process::exit(0);
    // return Ok(());
}
