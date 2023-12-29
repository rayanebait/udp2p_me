use std::collections::HashMap;

use crate::action::Action;

pub fn get_child_to_parent_hashmap(
    action: Action,
    mut hashmap: HashMap<[u8; 32], [u8; 32]>,
) -> HashMap<[u8; 32], [u8; 32]> {
    match action {
        Action::ProcessDatum(data, address) => {
            // println!("Process datum");
            let data_type = data.get(32).unwrap().to_owned();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(data.get(0..32).unwrap());
            match data_type {
                0 => {
                    // println!("Chunk");
                    // println!("Hash = {hash:?}");
                }
                1 => {
                    // println!("Tree");
                    let leaves = data.get(33..).unwrap().chunks(32).map(|s| s.to_owned());
                    for (i, leaf) in leaves.enumerate() {
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        // println!("Hash [{i}] = {leaf:?}");
                        hashmap.insert(leaf_slice, hash);
                    }
                }
                2 => {
                    // println!("Directory");
                    let leaves = data.get(33..).unwrap().chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = leaf.get(0..32).unwrap();
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        hashmap.insert(leaf_slice, hash);
                    }
                }
                _ => {
                    println!("Not a mkfs node")
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
        }
    }
    return (hashmap);
}

pub fn get_parent_to_child_hashmap(
    action: Action,
    mut hashmap: HashMap<[u8; 32], Vec<[u8; 32]>>,
) -> HashMap<[u8; 32], Vec<[u8; 32]>> {
    match action {
        Action::ProcessDatum(data, address) => {
            // println!("Process datum");
            let data_type = data.get(32).unwrap().to_owned();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(data.get(0..32).unwrap());
            match data_type {
                0 => {
                    // println!("Chunk");
                    // println!("Hash = {hash:?}");
                }
                1 => {
                    // println!("Tree");
                    let leaves: Vec<[u8; 32]> = data
                        .get(33..)
                        .unwrap()
                        .chunks(32)
                        .map(|s| {
                            let mut leaf_slice = [0u8; 32];
                            leaf_slice.copy_from_slice(s);
                            leaf_slice
                        })
                        .collect();
                    hashmap.insert(hash, leaves);
                }
                2 => {
                    // println!("Directory");
                    let leaves = data.get(33..).unwrap().chunks(64).map(|s| s.to_owned());
                    let mut children: Vec<[u8; 32]> = Vec::new();
                    for leaf in leaves {
                        let name = leaf.get(0..32).unwrap();
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        children.push(leaf_slice);
                    }
                    hashmap.insert(hash, children);
                }
                _ => {
                    println!("Not a mkfs node")
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
        }
    }
    return (hashmap);
}

pub fn get_hash_to_name_hashmap(
    action: Action,
    mut name_hashmap: HashMap<[u8; 32], String>,
    parent_hashmap: HashMap<[u8; 32], [u8; 32]>,
) -> HashMap<[u8; 32], String> {
    match action {
        Action::ProcessDatum(data, address) => {
            // println!("Process datum");
            let data_type = data.get(32).unwrap().to_owned();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(data.get(0..32).unwrap());
            // If no parent is found then it is the root
            let mut parent_name: String = match parent_hashmap.get(&hash) {
                Some(p) => name_hashmap.get(p).unwrap_or(&"/".to_string()).to_string(),
                None => "/".to_string(),
            };
            if parent_name.chars().last().unwrap() != '/' {
                parent_name.push_str("/");
            };
            match data_type {
                0 => {
                    // println!("Chunk");
                    // println!("Hash = {hash:?}");
                }
                1 => {
                    // println!("Tree");
                    let leaves = data.get(33..).unwrap().chunks(32).map(|s| s.to_owned());
                    for leaf in leaves {
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        name_hashmap.insert(leaf_slice, parent_name.clone());
                    }
                }
                2 => {
                    // println!("Directory");
                    let leaves = data.get(33..).unwrap().chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = leaf.get(0..32).unwrap().to_vec();
                        let name = String::from_utf8(name).unwrap();
                        let mut temp_name = parent_name.clone();
                        temp_name.push_str(&name);
                        let name = temp_name;
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        name_hashmap.insert(leaf_slice, name);
                    }
                }
                _ => {
                    println!("Not a mkfs node")
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
        }
    }
    return (name_hashmap);
}

pub fn get_name_to_hash_hashmap(
    action: Action,
    mut name_hashmap: HashMap<String, Vec<[u8; 32]>>,
    parent_hashmap: HashMap<[u8; 32], [u8; 32]>,
    reverse_name_hashmap: HashMap<[u8; 32], String>,
) -> HashMap<String, Vec<[u8; 32]>> {
    let action = action.clone();
    match action {
        Action::ProcessDatum(data, address) => {
            // println!("Process datum");
            let data_type = data.get(32).unwrap().to_owned();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(data.get(0..32).unwrap());
            let parent_hash = parent_hashmap.get(&hash);
            let parent_name: String = match parent_hash {
                Some(h) => reverse_name_hashmap.get(h).unwrap(),
                None => "/",
            }
            .to_string();
            // If no parent is found then it is the root
            match data_type {
                0 => {
                    // println!("Chunk");
                    // println!("Hash = {hash:?}");
                }
                1 => {
                    // println!("Tree");
                    let leaves = data.get(33..).unwrap().chunks(32).map(|s| s.to_owned());
                    // check if there is already an entry with this key
                    let result = match name_hashmap.get(&parent_name) {
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
                            name_hashmap.insert(parent_name, ids);
                        }
                        None => {
                            let ids: Vec<[u8; 32]> = leaves
                                .map(|leaf| {
                                    let mut leaf_slice = [0u8; 32];
                                    leaf_slice.copy_from_slice(&leaf);
                                    leaf_slice
                                })
                                .collect();
                            name_hashmap.insert(parent_name, ids);
                        }
                    }
                }
                2 => {
                    // println!("Directory");
                    let leaves = data.get(33..).unwrap().chunks(64).map(|s| s.to_owned());
                    for leaf in leaves {
                        let name = leaf.get(0..32).unwrap().to_vec();
                        let name = String::from_utf8(name).unwrap();
                        let mut temp_name = parent_name.clone();
                        temp_name.push_str(&name);
                        let name = temp_name;
                        let leaf = leaf.get(32..).unwrap();
                        let mut leaf_slice = [0u8; 32];
                        leaf_slice.copy_from_slice(&leaf);
                        name_hashmap.insert(name, vec![leaf_slice]);
                    }
                }
                _ => {
                    println!("Not a mkfs node")
                }
            }
        }
        _ => {
            println!("Not the datum we are looking for");
        }
    }
    return (name_hashmap);
}

#[cfg(test)]
pub mod test {
    use std::net::SocketAddr;

    use super::*;

    #[test]
    fn lib_network_store_datum() {
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
        let mut parent_hashmap = HashMap::<[u8; 32], [u8; 32]>::new();
        parent_hashmap = get_child_to_parent_hashmap(action, parent_hashmap);
        let mut name_hashmap = HashMap::<[u8; 32], String>::new();
        name_hashmap = get_hash_to_name_hashmap(action, name_hashmap, parent_hashmap);
        println!("{:?}", parent_hashmap);
    }

    #[test]
    fn lib_network_get_name_to_hash_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        let mut hashmap = HashMap::<[u8; 32], Vec<[u8; 32]>>::new();
        hashmap = get_parent_to_child_hashmap(action, hashmap);
        println!("{:?}", hashmap);
    }

    #[test]
    fn lib_network_get_parent_to_children_hashmap() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        let mut hashmap = HashMap::<[u8; 32], Vec<[u8; 32]>>::new();
        hashmap = get_parent_to_child_hashmap(action, hashmap);
        println!("{:?}", hashmap);
    }
}
