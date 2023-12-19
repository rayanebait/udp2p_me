pub mod discovery {
    //! This module contains functions to interact with the central REST server.
    //! Its goal is to provide all utilities discover peers, register, export root hashes etc...
    use anyhow::{bail, Context, Result};
    use log::{debug, error, info, warn};

    use reqwest::{Client, StatusCode, Url};

    #[derive(Debug)]
    pub struct Peer {
        pub name: String,
        pub addresses: Vec<String>,
    }

    /// Parse newline separated data into an array of strings.
    pub fn parse_newline_separated(data: impl Into<String>) -> Vec<String> {
        let data = data.into();
        let tokens = data
            .split('\n')
            .filter_map(|x| match x {
                "" => None,
                _ => Some(x.to_owned()),
            })
            .collect();
        return tokens;
    }

    /// Attempt to get data from a host and return it as a String.
    pub async fn get_raw_data(client: Client, url: &Url) -> Result<String> {
        let request = client.get(url.clone());
        let response = request
            .send()
            .await
            .context(format!("Failed to connect to host {}.", url.as_str()))?;

        match response.status() {
            StatusCode::OK => match response.text().await {
                Ok(s) => return Ok(s),
                Err(e) => bail!(format!(
                    "Failed to get retrieve text from host {}",
                    url.as_str()
                )),
            },
            StatusCode::NO_CONTENT => {
                return Ok("".to_string());
            }
            _ => {
                bail!(
                    "Failed to get data from host {} with status code {}.",
                    response.url().as_str(),
                    response.status()
                );
            }
        }
    }

    pub async fn get_peer_addresses(client: Client, base_url: &Url, peer: &str) -> Result<Peer> {
        // Peer adresses must be located at /peers/<p>/addresses from the base_url
        let url = base_url.join(format!("peers/{peer}/addresses").as_str())?;
        let data = get_raw_data(client, &url).await?;
        return Ok(Peer {
            name: peer.to_string(),
            addresses: parse_newline_separated(data),
        });
    }

    pub async fn get_peers_names(client: Client, base_url: &Url) -> Result<Vec<String>> {
        // Peers names must be located at /peers from the base_url
        let url = base_url.join("peers")?;
        let data = get_raw_data(client, &url).await?;
        let peers = parse_newline_separated(data);

        return Ok(peers);
    }

    pub async fn get_peer_key(client: Client, base_url: &Url, peer: &str) -> Result<String> {
        let url = base_url.join(format!("peers/{peer}/key").as_str())?;
        let data = get_raw_data(client, &url).await?;
        return Ok(data);
    }

    pub async fn get_peer_root(client: Client, base_url: &Url, peer: &str) -> Result<String> {
        let url = base_url.join(format!("peers/{peer}/root").as_str())?;
        let data = get_raw_data(client, &url).await?;
        return Ok(data);
    }

    #[cfg(test)]
    mod tests {

        use super::*;
        use std::time::Duration;
        use tokio;

        #[tokio::test]
        async fn lib_web_get_peers() {
            let host = Url::parse("https://jch.irif.fr:8443/").unwrap();
            let five_seconds = Duration::new(5, 0);
            let client = Client::builder()
                .timeout(five_seconds)
                .user_agent("Projet M2 protocoles Internet")
                .build()
                .unwrap();
            let result = get_peers_names(client, &host).await.unwrap();
            println!("Peers = {:?}", result);
        }

        #[tokio::test]
        async fn lib_web_get_peer_addresses() {
            let host = Url::parse("https://jch.irif.fr:8443/").unwrap();
            let five_seconds = Duration::new(5, 0);
            let client = Client::builder()
                .timeout(five_seconds)
                .user_agent("Projet M2 protocoles Internet")
                .build()
                .unwrap();
            let result = get_peer_addresses(client, &host, "jch.irif.fr")
                .await
                .unwrap();
            println!("Peer address = {:?}", result);
        }

        #[tokio::test]
        async fn lib_web_get_peer_key() {
            let host = Url::parse("https://jch.irif.fr:8443/").unwrap();
            let five_seconds = Duration::new(5, 0);
            let client = Client::builder()
                .timeout(five_seconds)
                .user_agent("Projet M2 protocoles Internet")
                .build()
                .unwrap();
            let result = get_peer_key(client, &host, "jch.irif.fr").await;
            match result {
                Ok(key) => {
                    if key.len() == 0 {
                        println!("Peer key doesn't exist.")
                    } else {
                        println!("Peer key = {:?}", key);
                    }
                }
                Err(e) => println!("Failed to retrieve peer key"),
            }
        }

        #[tokio::test]
        async fn lib_web_get_peer_root() {
            let host = Url::parse("https://jch.irif.fr:8443/").unwrap();
            let five_seconds = Duration::new(5, 0);
            let client = Client::builder()
                .timeout(five_seconds)
                .user_agent("Projet M2 protocoles Internet")
                .build()
                .unwrap();
            let result = get_peer_root(client, &host, "jch.irif.fr").await;
            match result {
                Ok(key) => {
                    if key.len() == 0 {
                        println!("Peer root doesn't exist.")
                    } else {
                        println!("Peer root = {:?}", key.as_bytes().to_vec());
                    }
                }
                Err(e) => println!("Failed to retrieve peer root"),
            }
        }
    }
}
