use clap::{Parser, Subcommand};
use log::{debug, error, info, warn};
// use anyhow::bail;
// use lib_file::mk_fs::MktFsNode;
// use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "UDP2P-cli")]
#[command(author = "Educated fool")]
#[command(version = "1.0")]
#[command(about = "P2P data exchange using UDP and NAT traversal.", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Fetch the available peers.
    FetchPeers {
        /// Url of peers page
        #[arg(short = 'u', long)]
        host: Option<String>,
    },
}

fn main() {
    env_logger::init();
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::FetchPeers { host }) => {
            println!("{:?}", host); // if host is not provided in correct form, the app will tell the user
        }
        None => {
            println!("No subcommand");
        }
    }
}

/*
let match_result = command!()
        .about("Connect to a distributed file sharing network of peers even behind a NAT.")
        .subcommand(
            Command::new("fetch-peers")
                .arg(
                    Arg::new("host")
                        .short('H')
                        .long("host")
                        .default_value("https://jch.irif.fr:8443")
                        .help("Hostname for the central server hosting the list of peers."),
                )
                .arg(
                    Arg::new("peers")
                        .long("peers")
                        .help("Peers directory location")
                        .default_value("/peers"),
                ),
        )
        .get_matches();

    let default_host = String::from("https://jch.irif.fr:8443");
    let host = match_result
        .subcommand_matches("fetch-peers")
        .unwrap()
        .get_one::<String>("host")
        .unwrap();
    let peers_directory = match_result
        .subcommand_matches("fetch-peers")
        .unwrap()
        .get_one::<String>("peers")
        .unwrap();

    println!("Fetching peers from : {}", host);
    println!("Directory : {}", peers_directory);

    let peers = udp2p::basic(&host)
        .await
        .expect("Could not retrieve peers.");

    println!("Got peers {}", peers);
} */
