use log::{error, warn};
use std::collections::HashMap;



use crate::{action::Action, peer::PeerError};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub struct SimpleNode {
    pub name: String,
    pub hash: [u8; 32],
    pub children: Option<Vec<SimpleNode>>,
    pub data: Option<Vec<u8>>,
}

impl SimpleNode {
    pub fn flatten(&self) -> Vec<u8> {
        let mut data = Vec::<u8>::new();

        match &self.data {
            Some(d) => data.extend_from_slice(d),
            None => (),
        }

        match &self.children {
            Some(children) => {
                for c in children.into_iter() {
                    data.extend_from_slice(&c.flatten())
                }
            }
            None => (),
        }

        return data;
    }
}

pub fn build_tree_mutex() -> Arc<
    Mutex<(
        HashMap<[u8; 32], [u8; 32]>,
        HashMap<[u8; 32], Vec<[u8; 32]>>,
        HashMap<[u8; 32], String>,
    )>,
> {
    Arc::new(Mutex::new((
        HashMap::<[u8; 32], [u8; 32]>::new(),
        HashMap::<[u8; 32], Vec<[u8; 32]>>::new(),
        HashMap::<[u8; 32], String>::new(),
    )))
}

pub fn build_tree_maps(
    action: &Action,
    maps: Arc<
        Mutex<(
            HashMap<[u8; 32], [u8; 32]>,
            HashMap<[u8; 32], Vec<[u8; 32]>>,
            HashMap<[u8; 32], String>,
        )>,
    >,
) -> Result<Option<Vec<[u8; 32]>>, PeerError> {
    let mut guard = match maps.lock() {
        Ok(maps) => maps,
        Err(poison_error) => {
            error!("{poison_error}");
            panic!("Some thread panicked")
        }
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

    // let hash_to_name_map = (guard.2).clone();
    // get_name_to_hash_hashmap(
    //     action,
    //     &mut guard.3,
    //     &child_to_parent_map,
    //     &hash_to_name_map,
    // )?;

    Ok(childs)
}

pub fn get_child_to_parent_hashmap(
    action: &Action,
    hashmap: &mut HashMap<[u8; 32], [u8; 32]>,
) -> Result<(), PeerError> {
    match action {
        Action::ProcessDatum(data, _address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return Err(PeerError::InvalidPacket),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return Err(PeerError::InvalidPacket),
            };
            hash.copy_from_slice(temp);
            match data_type {
                0 => {}
                1 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return Err(PeerError::InvalidPacket),
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
                        None => return Err(PeerError::InvalidPacket),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let _name = leaf.get(0..32).unwrap();
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        hashmap.insert(leaf_slice, hash);
                    }
                }
                _ => {
                    warn!("Not a mkfs node");
                    return Err(PeerError::InvalidPacket);
                }
            }
        }
        _ => {
            warn!("Not the datum we are looking for");
            return Err(PeerError::InvalidPacket);
        }
    }
    return Ok(());
}

pub fn get_children(
    action: &Action,
    maps: Arc<
        Mutex<(
            HashMap<[u8; 32], [u8; 32]>,
            HashMap<[u8; 32], Vec<[u8; 32]>>,
            HashMap<[u8; 32], String>,
        )>,
    >,
) -> Result<SimpleNode, PeerError> {
    match action {
        Action::ProcessDatum(data, _address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return Err(PeerError::InvalidPacket),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return Err(PeerError::InvalidPacket),
            };
            hash.copy_from_slice(temp);
            let name = match maps.lock() {
                Ok(m) => match m.2.get(&hash) {
                    Some(n) => n.to_string(),
                    None => "/".to_string(),
                },
                Err(e) => {
                    error!("Could not lock hash to name map because of {e}");
                    panic!("Failed")
                }
            };
            match data_type {
                0 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return Err(PeerError::InvalidPacket),
                    };
                    let node = SimpleNode {
                        name: name,
                        hash: hash,
                        children: None,
                        data: Some(data.to_vec()),
                    };
                    return Ok(node);
                }
                1 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return Err(PeerError::InvalidPacket),
                    };
                    let leaves = data.chunks(32).map(|s| s.to_owned());
                    let children: Vec<SimpleNode> = leaves
                        .into_iter()
                        .map(|l| {
                            let mut leaf_slice = [0u8; 32];
                            leaf_slice.copy_from_slice(&l);
                            SimpleNode {
                                name: name.clone(),
                                hash: leaf_slice,
                                children: None,
                                data: None,
                            }
                        })
                        .collect();
                    let node = SimpleNode {
                        name: name,
                        hash: hash,
                        children: Some(children),
                        data: None,
                    };
                    return Ok(node);
                }
                2 => {
                    warn!("Found a directory, not a valid file.");
                    return Err(PeerError::InvalidPacket);
                }
                _ => {
                    warn!("Not a mkfs node");
                    return Err(PeerError::InvalidPacket);
                }
            }
        }
        _ => {
            warn!("Not the datum we are looking for");
            return Err(PeerError::InvalidPacket);
        }
    }
}

pub fn get_type(action: &Action) -> Result<u8, PeerError> {
    match action {
        Action::ProcessDatum(data, _address) => {
            match data.get(32) {
                Some(d) => return Ok(d.to_owned()),
                None => return Err(PeerError::InvalidPacket),
            };
        }
        _ => {
            warn!("Not the datum we are looking for");
            return Err(PeerError::InvalidPacket);
        }
    }
}

pub fn get_parent_to_child_hashmap(
    action: &Action,
    hashmap: &mut HashMap<[u8; 32], Vec<[u8; 32]>>,
) -> Result<Option<Vec<[u8; 32]>>, PeerError> {
    match action {
        Action::ProcessDatum(data, _address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return Err(PeerError::InvalidPacket),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return Err(PeerError::InvalidPacket),
            };
            hash.copy_from_slice(temp);
            match data_type {
                0 => Ok(None),
                1 => Ok(None),
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return Err(PeerError::InvalidPacket),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    let mut children: Vec<[u8; 32]> = Vec::new();
                    for leaf in leaves {
                        let _name = match leaf.get(0..32) {
                            Some(d) => d,
                            None => return Err(PeerError::InvalidPacket),
                        };
                        let leaf = match leaf.get(32..) {
                            Some(d) => d,
                            None => return Err(PeerError::InvalidPacket),
                        };
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        children.push(leaf_slice);
                    }
                    hashmap.insert(hash, children.clone());
                    Ok(Some(children))
                }
                _ => {
                    warn!("Not a mkfs node");
                    return Err(PeerError::InvalidPacket);
                }
            }
        }
        _ => {
            warn!("Not the datum we are looking for");
            return Err(PeerError::InvalidPacket);
        }
    }
}

pub fn get_hash_to_name_hashmap(
    action: &Action,
    h_to_n_hashmap: &mut HashMap<[u8; 32], String>,
    _c_to_p_hashmap: &HashMap<[u8; 32], [u8; 32]>,
) -> Result<(), PeerError> {
    match action {
        Action::ProcessDatum(data, _address) => {
            let data_type = match data.get(32) {
                Some(d) => d.to_owned(),
                None => return Err(PeerError::InvalidPacket),
            };
            let mut hash = [0u8; 32];
            let temp = match data.get(0..32) {
                Some(d) => d,
                None => return Err(PeerError::InvalidPacket),
            };
            hash.copy_from_slice(temp);

            // let parent_name = match c_to_p_hashmap.get(&hash) {
            //     Some(p) => match h_to_n_hashmap.get(p) {
            //         Some(n) => n.to_string(),
            //         None => "".to_string(),
            //     },
            //     None => "".to_string(),
            // };

            // println!("Parent name {} for data type {:?}", parent_name, data_type);

            match data_type {
                0 => {}
                1 => {}
                2 => {
                    let data = match data.get(33..) {
                        Some(d) => d,
                        None => return Err(PeerError::InvalidPacket),
                    };
                    let leaves = data.chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = match leaf.get(0..32) {
                            Some(d) => d,
                            None => return Err(PeerError::InvalidPacket),
                        };
                        let mut name = name.to_vec();
                        name.retain(|&x| x != 0u8);
                        let name = String::from_utf8(name).unwrap();
                        let leaf = match leaf.get(32..) {
                            Some(d) => d,
                            None => return Err(PeerError::InvalidPacket),
                        };
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);

                        // let mut temp_parent_name = parent_name.clone();
                        // println!("Parent name {}", parent_name);
                        // if temp_parent_name.chars().last() != Some('/') && name != "" {
                        //     temp_parent_name.push('/');
                        // }
                        // temp_parent_name.push_str(&name);
                        // println!("Full name {}", temp_parent_name);
                        // h_to_n_hashmap.insert(leaf_slice, temp_parent_name);
                        h_to_n_hashmap.insert(leaf_slice, name);
                    }
                }
                _ => {
                    warn!("Not a mkfs node");
                    return Err(PeerError::InvalidPacket);
                }
            }
        }
        _ => {
            warn!("Not the datum we are looking for");
            return Err(PeerError::InvalidPacket);
        }
    }
    // println!("Hash to name hashmap {:?}", &h_to_n_hashmap);
    return Ok(());
}

pub fn get_full_name(
    hash: &[u8; 32],
    h_to_n_hashmap: &HashMap<[u8; 32], String>,
    c_to_p_hashmap: &HashMap<[u8; 32], [u8; 32]>,
) -> String {
    let mut name = String::new();
    match c_to_p_hashmap.get(hash) {
        Some(h) => {
            let child_name = match h_to_n_hashmap.get(hash) {
                Some(n) => n.to_string(),
                None => {
                    // This hash has no name, it is probably part of a tree, get the first node of tree to get the name
                    "".to_string()
                }
            };
            let mut parent_name = get_full_name(h, h_to_n_hashmap, c_to_p_hashmap);
            if parent_name.chars().last() != Some('/') && child_name != "" {
                parent_name.push('/');
            }
            name.push_str(&parent_name);
            name.push_str(&child_name);
        }
        None => {
            return "/".to_string();
        }
    }
    return name;
}

pub fn get_name_to_hash_hashmap(
    c_to_p_hashmap: &HashMap<[u8; 32], [u8; 32]>,
    h_to_n_hashmap: &HashMap<[u8; 32], String>,
) -> HashMap<String, [u8; 32]> {
    let mut n_to_h_hashmap = HashMap::<String, [u8; 32]>::new();
    for (c, _p) in c_to_p_hashmap.into_iter() {
        n_to_h_hashmap.insert(get_full_name(c, h_to_n_hashmap, c_to_p_hashmap), c.clone());
    }
    return n_to_h_hashmap;
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

        let _action = Action::ProcessDatum(data, address);
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

        let _action = Action::ProcessDatum(data, address);
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

        let _action = Action::ProcessDatum(data, address);
        // let mut c_to_p_hashmap = Arc::new(Mutex::new(HashMap::<[u8; 32], [u8; 32]>::new()));
        // get_child_to_parent_hashmap(&action, Arc::clone(&c_to_p_hashmap));
        // let mut h_to_n_hashmap = HashMap::<[u8; 32], String>::new();
        // // get_hash_to_name_hashmap(&action, &mut h_to_n_hashmap, &c_to_p_hashmap);
        // println!("{:?}", h_to_n_hashmap);
    }

    // #[test]
    // fn lib_network_store_get_name_to_hash_hashmap() {
    //     let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
    //     let data = vec![
    //         1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 2, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 0, 0, 0, 0, 69, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 70, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 71, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 72, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //         0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
    //     ];

    //     let action = Action::ProcessDatum(data, address);
    //     // let mut c_to_p_hashmap =Arc::new(Mutex::new( HashMap::<[u8; 32], [u8; 32]>::new()));
    //     // get_child_to_parent_hashmap(&action, Arc::clone(&c_to_p_hashmap));
    //     // let mut h_to_n_hashmap = HashMap::<[u8; 32], String>::new();
    //     // get_hash_to_name_hashmap(&action, &mut h_to_n_hashmap, &c_to_p_hashmap);
    //     // let mut n_to_h_hashmap = HashMap::<String, Vec<[u8; 32]>>::new();
    //     // get_name_to_hash_hashmap(&action, &mut n_to_h_hashmap, c_to_p_hashmap, h_to_n_hashmap);
    //     // println!("{n_to_h_hashmap:?}");
    // }

    #[test]
    fn lib_network_store_flatten() {
        let mut children: Vec<SimpleNode> = (1..11)
            .map(|i| SimpleNode {
                name: i.to_string(),
                hash: [i as u8; 32],
                children: None,
                data: Some(vec![i as u8; 5]),
            })
            .collect();

        let children2: Vec<SimpleNode> = (11..21)
            .map(|i| SimpleNode {
                name: i.to_string(),
                hash: [i as u8; 32],
                children: None,
                data: Some(vec![i as u8; 5]),
            })
            .collect();

        children[0].children = Some(children2);

        let parent = SimpleNode {
            name: "root".to_string(),
            hash: [0u8; 32],
            children: Some(children),
            data: None,
        };

        println!("Node : {parent:?}");

        let data = parent.flatten();

        println!("Data : {data:?}");
    }
}
