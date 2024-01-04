use std::future::IntoFuture;
use std::io::Error;
use std::str::FromStr;
use std::thread::sleep;
use std::time::{Instant, Duration};

use tokio::{join, try_join, select};
use tokio_macros;
use reqwest::{Url,Client};
use lib_web::discovery::{*};
use sha2::{Sha256, Digest};
use hex;

#[tokio::main]
async fn main() {
    let mut hasher = Sha256::new();
    hasher.update("");
    println!("{:?}", hex::encode(hasher.finalize().as_slice()));
    let now = Instant::now();
    sleep(Duration::from_secs(1));
    let then = Instant::now();
    let elapsed = now.elapsed();
    println!("{:?}, {:?}", elapsed, then-now);
}
