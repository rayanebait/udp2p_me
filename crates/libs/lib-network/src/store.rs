use std::collections::HashMap;

use lib_web::discovery::Peer;

use crate::{action::Action, peer::peer::PeerError};
use std::sync::{Arc, Mutex, PoisonError};

pub fn build_tree_mutex() -> Arc<
    Mutex<(
        HashMap<[u8; 32], [u8; 32]>,
        HashMap<[u8; 32], Vec<[u8; 32]>>,
        HashMap<[u8; 32], String>,
        HashMap<String, Vec<[u8; 32]>>,
    )>,
> {
    Arc::new(Mutex::new((
        HashMap::<[u8; 32], [u8; 32]>::new(),
        HashMap::<[u8; 32], Vec<[u8; 32]>>::new(),
        HashMap::<[u8; 32], String>::new(),
        HashMap::<String, Vec<[u8; 32]>>::new(),
    )))
}
pub fn build_tree_maps(
    action: &Action,
    maps: Arc<
        Mutex<(
            HashMap<[u8; 32], [u8; 32]>,
            HashMap<[u8; 32], Vec<[u8; 32]>>,
            HashMap<[u8; 32], String>,
            HashMap<String, Vec<[u8; 32]>>,
        )>,
    >,
) -> Result<Option<Vec<[u8; 32]>>, PeerError> {
    let mut guard = match maps.lock() {
        Ok(maps) => maps,
        Err(poison_error) => panic!("Some thread panicked"),
    };
    // let (
    //     mut child_to_parent_map,
    //     mut parent_to_child_map,
    //     mut name_to_hash_map,
    //     mut hash_to_name_map,
    // ) = (guard.0, guard.1, guard.2, guard.3);

    get_child_to_parent_hashmap(action, &mut guard.0)?;
    let childs = get_parent_to_child_hashmap(action, &mut guard.1)?;

    let child_to_parent_map = (guard.0).clone();
    get_hash_to_name_hashmap(action, &mut guard.2, &child_to_parent_map)?;

    let hash_to_name_map = (guard.2).clone();
    get_name_to_hash_hashmap(
        action,
        &mut guard.3,
        &child_to_parent_map,
        &hash_to_name_map,
    )?;

    Ok(childs)
}

pub fn get_child_to_parent_hashmap(
    action: &Action,
    hashmap: &mut HashMap<[u8; 32], [u8; 32]>,
) -> Result<(), PeerError> {
    match action {
        Action::ProcessDatum(data, address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return (Err(PeerError::InvalidPacket)),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return (Err(PeerError::InvalidPacket)),
            };
            hash.copy_from_slice(temp);
            match data_type {
                0 => {}
                1 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(32).map(|s| s.to_owned());
                    for leaf in leaves {
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        hashmap.insert(leaf_slice, hash);
                    }
                }
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = leaf.get(0..32).unwrap();
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        hashmap.insert(leaf_slice, hash);
                    }
                }
                _ => {
                    println!("Not a mkfs node");
                    return (Err(PeerError::InvalidPacket));
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
            return (Err(PeerError::InvalidPacket));
        }
    }
    return Ok(());
}

pub fn get_parent_to_child_hashmap(
    action: &Action,
    hashmap: &mut HashMap<[u8; 32], Vec<[u8; 32]>>,
) -> Result<Option<Vec<[u8; 32]>>, PeerError> {
    match action {
        Action::ProcessDatum(data, address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return (Err(PeerError::InvalidPacket)),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return (Err(PeerError::InvalidPacket)),
            };
            hash.copy_from_slice(temp);
            match data_type {
                0 => Ok(None),
                1 => Ok(None),
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    let mut children: Vec<[u8; 32]> = Vec::new();
                    for leaf in leaves {
                        let name = match leaf.get(0..32) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let leaf = match leaf.get(32..) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        children.push(leaf_slice);
                    }
                    hashmap.insert(hash, children.clone());
                    Ok(Some(children))
                }
                _ => {
                    println!("Not a mkfs node");
                    return (Err(PeerError::InvalidPacket));
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
            return (Err(PeerError::InvalidPacket));
        }
    }
}

pub fn get_hash_to_name_hashmap(
    action: &Action,
    h_to_n_hashmap: &mut HashMap<[u8; 32], String>,
    c_to_p_hashmap: &HashMap<[u8; 32], [u8; 32]>,
) -> Result<(), PeerError> {
    match action {
        Action::ProcessDatum(data, address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return (Err(PeerError::InvalidPacket)),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return (Err(PeerError::InvalidPacket)),
            };
            hash.copy_from_slice(temp);
            // If no parent is found then it is the root
            let mut parent_name: String = match c_to_p_hashmap.get(&hash) {
                Some(p) => h_to_n_hashmap
                    .get(p)
                    .unwrap_or(&"/".to_string())
                    .to_string(),
                None => "/".to_string(),
            };
            match parent_name.chars().last() {
                Some('/') => (),
                _ => parent_name.push_str("/"),
            };
            match data_type {
                0 => {}
                1 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(32).map(|s| s.to_owned());
                    for leaf in leaves {
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        h_to_n_hashmap.insert(leaf_slice, parent_name.clone());
                    }
                }
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = match leaf.get(0..32) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let mut name = name.to_vec();
                        name.retain(|&x| x != 0u8);
                        let name = String::from_utf8(name).unwrap();
                        let mut temp_name = parent_name.clone();
                        temp_name.push_str(&name);
                        let name = temp_name;
                        let leaf = match leaf.get(32..) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        h_to_n_hashmap.insert(leaf_slice, name);
                    }
                }
                _ => {
                    println!("Not a mkfs node");
                    return (Err(PeerError::InvalidPacket));
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
            return (Err(PeerError::InvalidPacket));
        }
    }
    return Ok(());
}

pub fn get_name_to_hash_hashmap(
    action: &Action,
    n_to_h_hashmap: &mut HashMap<String, Vec<[u8; 32]>>,
    c_to_p_hashmap: &HashMap<[u8; 32], [u8; 32]>,
    h_to_n_hashmap: &HashMap<[u8; 32], String>,
) -> Result<(), PeerError> {
    let action = action.clone();
    match action {
        Action::ProcessDatum(data, address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return (Err(PeerError::InvalidPacket)),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return (Err(PeerError::InvalidPacket)),
            };
            hash.copy_from_slice(temp);
            let parent_hash = c_to_p_hashmap.get(&hash);
            let parent_name: String = match parent_hash {
                Some(h) => h_to_n_hashmap.get(h).unwrap(),
                None => "/",
            }
            .to_string();
            match data_type {
                0 => {}
                1 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(32).map(|s| s.to_owned());
                    // check if there is already an entry with this key
                    let result = match n_to_h_hashmap.get(&parent_name) {
                        Some(c) => Some(c.clone()),
                        None => None,
                    };
                    match result {
                        Some(mut ids) => {
                            for leaf in leaves {
                                let mut leaf_slice = [0u8; 32];
                                leaf_slice.copy_from_slice(&leaf);
                                ids.push(leaf_slice.clone());
                            }
                            n_to_h_hashmap.insert(parent_name, ids);
                        }
                        None => {
                            let ids: Vec<[u8; 32]> = leaves
                                .map(|leaf| {
                                    let mut leaf_slice = [0u8; 32];
                                    leaf_slice.copy_from_slice(&leaf);
                                    leaf_slice
                                })
                                .collect();
                            n_to_h_hashmap.insert(parent_name, ids);
                        }
                    }
                }
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return (Err(PeerError::InvalidPacket)),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = match leaf.get(0..32) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let mut name = name.to_vec();
                        name.retain(|&x| x != 0u8);
                        let name = String::from_utf8(name).unwrap();
                        let mut temp_name = parent_name.clone();
                        temp_name.push_str(&name);
                        let name = temp_name;
                        let leaf = match leaf.get(32..) {
                            Some(d) => d,
                            None => return (Err(PeerError::InvalidPacket)),
                        };
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        n_to_h_hashmap.insert(name, vec![leaf_slice]);
                    }
                }
                _ => {
                    println!("Not a mkfs node");
                    return (Err(PeerError::InvalidPacket));
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
            return (Err(PeerError::InvalidPacket));
        }
    }
    return Ok(());
}

#[cfg(test)]
pub mod test {
    use std::net::SocketAddr;

    use super::*;

    #[test]
    fn lib_network_store_get_child_to_parent_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        // let mut hashmap = Arc::new(Mutex::new(HashMap::<[u8; 32], [u8; 32]>::new()));
        // // get_child_to_parent_hashmap(&action, Arc::clone(&hashmap));
        // let hashmap = hashmap.lock().unwrap();
        // println!("{:?}", hashmap);
    }

    #[test]
    fn lib_network_store_get_parent_to_child_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        // let mut p_to_c_hashmap = Arc::new(Mutex::new(HashMap::<[u8; 32], Vec<[u8; 32]>>::new()));
        // get_parent_to_child_hashmap(&action, Arc::clone(&p_to_c_hashmap));
        // println!("{:?}", p_to_c_hashmap);
    }

    #[test]
    fn lib_network_store_get_hash_to_name_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        // let mut c_to_p_hashmap = Arc::new(Mutex::new(HashMap::<[u8; 32], [u8; 32]>::new()));
        // get_child_to_parent_hashmap(&action, Arc::clone(&c_to_p_hashmap));
        // let mut h_to_n_hashmap = HashMap::<[u8; 32], String>::new();
        // // get_hash_to_name_hashmap(&action, &mut h_to_n_hashmap, &c_to_p_hashmap);
        // println!("{:?}", h_to_n_hashmap);
    }

    #[test]
    fn lib_network_store_get_name_to_hash_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        // let mut c_to_p_hashmap =Arc::new(Mutex::new( HashMap::<[u8; 32], [u8; 32]>::new()));
        // get_child_to_parent_hashmap(&action, Arc::clone(&c_to_p_hashmap));
        // let mut h_to_n_hashmap = HashMap::<[u8; 32], String>::new();
        // get_hash_to_name_hashmap(&action, &mut h_to_n_hashmap, &c_to_p_hashmap);
        // let mut n_to_h_hashmap = HashMap::<String, Vec<[u8; 32]>>::new();
        // get_name_to_hash_hashmap(&action, &mut n_to_h_hashmap, c_to_p_hashmap, h_to_n_hashmap);
        // println!("{n_to_h_hashmap:?}");
    }
}
