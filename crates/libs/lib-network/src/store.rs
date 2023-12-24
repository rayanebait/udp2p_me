use crate::action::Action;
use lib_file::mk_fs;

pub fn get_fs_tree(action: Action) {
    match action {
        Action::ProcessDatum(data, address) => {
            println!("Process datum");
            let data_type = data.get(32).unwrap().to_owned();
            let hash = data.get(0..32).to_owned();
            match data_type {
                0 => {
                    println!("Chunk");
                    println!("Hash = {hash:?}");
                }
                1 => {
                    println!("Tree");
                    let leaves = data.get(33..).unwrap().chunks(32).map(|s| s.to_owned());
                    for (i, leaf) in leaves.enumerate() {
                        println!("Hash [{i}] = {leaf:?}");
                    }
                }
                2 => {
                    println!("Directory");
                    let leaves = data.get(33..).unwrap().chunks(64).map(|s| s.to_owned());
                    for (i, leaf) in leaves.enumerate() {
                        println!("Name [{i}] = {:?}", leaf.get(0..32).unwrap());
                        println!("Hash [{i}] = {:?}", leaf.get(32..).unwrap());
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
}

#[cfg(test)]
pub mod test {
    use std::net::SocketAddr;

    use crate::action::ActionBuilder;

    use super::*;

    #[test]
    fn lib_network_store_datum() {
        let address = "127.0.0.1:8080".parse::<SocketAddr>().unwrap();
        let data = vec![
            1, 2, 3, 4, 5, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let action = Action::ProcessDatum(data, address);
        get_fs_tree(action);
    }
}
