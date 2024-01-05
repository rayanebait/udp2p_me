use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use hex;
use lib_network::{
    action::*, congestion_handler::*, import_export::*, packet::*, peer::*, store::*,
    task_launcher_canceller::*,
};
use lib_web::discovery;
use log::{debug, error, info, warn};
use std::{
    fs::File,
    io::Write,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{self, net::UdpSocket, time::sleep};

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
    /// Download the file system structure below a hash
    /// If no hash is provided, the program will first try to get the root hash of the peer.
    FetchFileTree {
        /// Address of the peer
        #[arg(short, long)]
        peer: String,
        /// Datum hash
        #[arg(short, long)]
        datum: Option<String>,
    },
    /// Download a file from a hash
    FetchFile {
        /// Address of the peer
        #[arg(short, long)]
        peer: String,
        /// Datum hash
        #[arg(short, long)]
        datum: String,
        /// Output path
        /// Default value is ./dump
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 10)]
async fn main() -> Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Commands::FetchPeers { host } => {
            log::info!("Fetching peers from central server {}.", host);
            let client = discovery::get_client(5)?;
            let url = discovery::parse_url(host)?;
            let peers = discovery::get_peers_names(&client, &url).await?;
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
            let addr = discovery::get_peer_addresses(&client, &url, &peer).await?;
            let root = discovery::get_peer_root(&client, &url, peer).await?;
            let key = discovery::get_peer_key(&client, &url, peer).await?;
            println!("Informations for peer {peer}");
            println!("Addresses :");
            for a in addr.addresses.into_iter() {
                println!("\t- {a}");
            }
            println!("Root :\n\t- {}", hex::encode(root));
            println!("Key :\n\t- {}", hex::encode(key));
        }
        Commands::GetDatum { peer, datum } => {
            log::info!("Fetching datum from peer {} with hash {}.", peer, datum);
            let datum = match hex::decode(datum) {
                Ok(h) => match <[u8; 32]>::try_from(h) {
                    Ok(i) => i,
                    Err(e) => {
                        bail!("Invalid root hash. Please check your input.");
                    }
                },
                Err(e) => bail!("Failed to decode root hash. Please check your input."),
            };
            let packet = PacketBuilder::new()
                .gen_id()
                .packet_type(PacketType::GetDatum)
                .body(datum.to_vec())
                .build()
                .unwrap_or_default();
            println!("{packet:#?}");
            println!("{:#?}", packet.as_bytes());
        }
        Commands::FetchFileTree { peer, datum } => {
            let mut peer_hash: Option<[u8; 32]> = match datum {
                Some(d) => {
                    log::info!("Fetching file tree from peer {} for hash {}.", peer, d);
                    println!("Fetching file tree from peer {} for hash {}.", peer, d);
                    match hex::decode(d) {
                        Ok(h) => match <[u8; 32]>::try_from(h) {
                            Ok(i) => Some(i),
                            Err(e) => {
                                bail!("Invalid root hash. Please check your input.");
                                None
                            }
                        },
                        Err(e) => bail!("Failed to decode root hash. Please check your input."),
                    }
                }
                None => {
                    log::info!("Fetching file tree from peer {} from root hash.", peer);
                    println!("Fetching file tree from peer {} from root hash.", peer);
                    None
                }
            };

            let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
            let sock6 = Arc::new(
                UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
                    .await
                    .unwrap(),
            );
            let maps = build_tree_mutex();
            let queues = build_queues();
            let active_peers = ActivePeers::build_mutex();

            let receive_queue_state = Arc::clone(&queues.5);
            let action_queue = Arc::clone(&queues.2);
            let action_queue_state = Arc::clone(&queues.6);
            let process_queue = Arc::clone(&queues.3);
            let process_queue_state = Arc::clone(&queues.8);
            let process_queue_readers_state = Arc::clone(&queues.9);

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

            /*jch */
            let sock_addr: SocketAddr = peer.parse().unwrap();
            {
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    Action::SendHello(None, "nist".to_string().into_bytes(), *&sock_addr),
                );
                QueueState::set_non_empty_queue(action_queue_state.clone());
            }

            let peer_hash = match peer_hash {
                Some(h) => h,
                None => {
                    match {
                        let mut attempt = 0;
                        loop {
                            if attempt < 4 {
                                attempt += 1;
                            } else {
                                break None;
                            }
                            Queue::lock_and_push(
                                Arc::clone(&action_queue),
                                Action::SendRoot(None, sock_addr),
                            );
                            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                            process_queue_state.wait();
                            sleep(Duration::from_millis(100)).await;
                            let guard = active_peers.lock().unwrap();
                            /*If panics here, means the packet received had invalid hash (body length<32) */
                            match guard.get(sock_addr) {
                                Some(peer) => break peer.get_root_hash(),
                                None => continue,
                            }
                        }
                    } {
                        Some(h) => h,
                        None => {
                            println!("Peer is not exporting any file.");
                            bail!("Peer is not exporting any file.");
                        }
                    }
                }
            };

            fetch_subtree_from(
                Arc::clone(&process_queue),
                Arc::clone(&process_queue_readers_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                Arc::clone(&maps),
                // yoan_hash,
                peer_hash,
                sock_addr,
            )
            .await;

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
                        let mut carry = str::repeat("   ", step);
                        carry.push_str("└──");
                        println!("{carry} {n}");
                        println!("   {carry} {}", hex::encode(n_to_h_hashmap.get(n).unwrap()));
                    }
                }
                Err(e) => error!("Got error {e}"),
            };
        }
        Commands::FetchFile {
            peer,
            datum,
            output,
        } => {
            let hash = match hex::decode(datum) {
                Ok(h) => match <[u8; 32]>::try_from(h) {
                    Ok(i) => i,
                    Err(e) => {
                        bail!("Invalid root hash. Please check your input.");
                    }
                },
                Err(e) => bail!("Failed to decode root hash. Please check your input."),
            };

            let path = match output {
                Some(s) => s.to_string(),
                None => "./dump".to_string(),
            };

            log::info!("Fetching file from peer {} for hash {}.", peer, &datum);
            println!("Fetching file from peer {} for hash {}.", peer, &datum);

            let mut file = match File::create(&path) {
                Ok(f) => f,
                Err(e) => bail!("Could not create or open file {} : {}", &path, e),
            };

            let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
            let sock6 = Arc::new(
                UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
                    .await
                    .unwrap(),
            );
            let maps = build_tree_mutex();
            let queues = build_queues();
            let active_peers = ActivePeers::build_mutex();

            let receive_queue_state = Arc::clone(&queues.5);
            let action_queue = Arc::clone(&queues.2);
            let action_queue_state = Arc::clone(&queues.6);
            let process_queue = Arc::clone(&queues.3);
            let process_queue_state = Arc::clone(&queues.8);
            let process_queue_readers_state = Arc::clone(&queues.9);

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

            /*jch */
            let sock_addr: SocketAddr = peer.parse().unwrap();
            {
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    Action::SendHello(None, "nist".to_string().into_bytes(), *&sock_addr),
                );
                QueueState::set_non_empty_queue(action_queue_state.clone());
            }

            let downloaded_file = download_file(
                Arc::clone(&process_queue),
                Arc::clone(&process_queue_readers_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                Arc::clone(&maps),
                hash,
                sock_addr,
            )
            .await;

            match downloaded_file {
                Ok(f) => {
                    let content = f.flatten();
                    println!("Size of file {}", &content.len());
                    match file.write(&content) {
                        Ok(size) => println!("Download completed. Wrote {} bytes", size),
                        Err(e) => {
                            println!("Failed to save file {e}");
                            bail!("Failed to save file {e}")
                        }
                    }
                }
                Err(e) => {
                    println!("Download failed {e}");
                    bail!("Download failed {e}")
                }
            };
        }
        _ => {
            error!("No subcommand");
        }
    }
    return Ok(());
}
