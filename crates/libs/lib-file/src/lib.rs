pub mod mk_fs {
    //! This module contains functions to manipulate files.
    //! Its goal is to provide all utilities to extract data from files and prepare it to be exported to the REST server and sent over the network.
    use anyhow::{bail, Context, Result};
    use log::{debug, error, info, warn};
    use sha2::{Digest, Sha256};
    use std::{collections::HashMap, fmt, fs, path::PathBuf};

    /// Merkle tree node type enum.
    ///
    /// The nodes of the Merkle tree can be of three types :
    /// - `chunk` are the leaf nodes and represent the actual data blocks
    /// - `directory` represent the directories in the file system, they only hold children and no data
    /// - `bigfile` represent files bigger than the chunk size, they don't hold the data but pass it to their children
    #[derive(Debug)]
    pub enum MktFsNodeType {
        DIRECTORY,
        CHUNK,
        BIGFILE,
    }

    /// Merkle tree node representing the file system.
    #[derive(Debug)]
    pub struct MktFsNode {
        pub path: PathBuf,                    // mandatory
        pub ntype: MktFsNodeType,             // mandatory
        pub children: Option<Vec<MktFsNode>>, // optional (chunk no child)
        // pub data: Option<Vec<u8>>,            // optional (dir no data)
        pub hash: [u8; 32], // mandatory
    }

    impl fmt::Display for MktFsNode {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            writeln!(
                f,
                "Node[{:?}]({})",
                self.ntype,
                self.path.as_path().to_string_lossy()
            );
            // match &self.data {
            //     Some(d) => {
            //         if d.len() > 10 {
            //             writeln!(
            //                 f,
            //                 "\tData : {:?}...",
            //                 d.iter().take(10).collect::<Vec<&u8>>()
            //             )
            //         } else {
            //             writeln!(f, "\tData : {:?}", d)
            //         }
            //     }
            //     _ => writeln!(f, "\tData : No data"),
            // };
            writeln!(
                f,
                "\tHash : {:?}...",
                &self.hash.iter().take(10).collect::<Vec<&u8>>()
            );
            match &self.children {
                Some(c) => {
                    write!(f, "\tChildren :");
                    for s in c.iter() {
                        write!(f, "\n{}", s);
                    }
                }
                _ => {
                    write!(f, "\tChildren : No child");
                }
            };
            return Ok(());
        }
    }

    impl MktFsNode {
        /// Create a `MktFsNode` from an array of bytes.
        ///
        /// This method will adequately detect if it needs to create a `CHUNK` of `BIGFILE` type.
        /// The children are packed to minimize the global number of children as well as the depth of the Merkle tree.
        ///
        /// It can fail however if unreasonable parameters are provided such as a `chunk_size = 0`
        /// or `max_children < 2` if there needs to be children.
        pub fn try_from_bytes(
            path: &PathBuf,
            data: impl Into<Vec<u8>>,
            chunk_size: usize,
            max_children: usize,
        ) -> Result<MktFsNode> {
            // Shadow the variable to cast it in the correct type and own it.
            let data = data.into();

            // If there is data, the chunk_size cannot be 0 otherwise it is impossible to pack
            if (chunk_size == 0) & (data.len() > 0) {
                error!(
                    "Failed to create a node for path {}. This may corrupt the file.",
                    path.to_string_lossy()
                );
                bail!("Cannot pack data with chunk_size 0.");
            }

            // If the data is small enough to fit in a single chunk
            // return the chunk directly
            if data.len() <= chunk_size {
                return (Ok(MktFsNode {
                    path: path.clone(),
                    ntype: MktFsNodeType::CHUNK,
                    children: None,
                    hash: hash_bytes_prefix(&data, 0),
                    // data: Some(data),
                }));
            }
            // If the data cannot fit in a single chunk it has to be split in children nodes
            else {
                // If the max number of children is not greater than 1 then the data cannot be packed
                // as the number of chunks cannot grow
                if max_children < 2 {
                    error!(
                        "Failed to create a node for path {}. This may corrupt the file.",
                        path.to_string_lossy()
                    );
                    bail!("Cannot build a big file node with fewer than 2 children.");
                }

                // Compute the adequate number of children for optimal packing
                let n_chunks = data.len().div_ceil(chunk_size);
                // ilog is rounded down which is convenient
                let mut n_layers = n_chunks.ilog(max_children);
                // If the number of children fits perfectly in the children arrays
                // then the number of layers required is actually one fewer
                if n_chunks == max_children.pow(n_layers) {
                    n_layers -= 1;
                }

                // Generate children nodes recursively, excluding the nodes where the construction fails
                // TODO : Warn the user of such failures
                let children = data
                    .chunks(chunk_size * max_children.pow(n_layers))
                    .filter_map(|d| {
                        match MktFsNode::try_from_bytes(path, d, chunk_size, max_children) {
                            Ok(n) => Some(n),
                            Err(e) => None,
                        }
                    })
                    .collect::<Vec<MktFsNode>>();

                // Compute hash of the root node from the hashes of children
                let mut hasher = Sha256::new();

                // A BIGFILE is prefixed with 1 before hashing
                hasher.update([1]);
                for c in children.iter() {
                    hasher.update(c.hash);
                }
                let mut hash = <[u8; 32]>::default();
                hash.copy_from_slice(hasher.finalize().as_slice());

                // Generate root node with the computed children and hash
                return (Ok(MktFsNode {
                    path: path.into(),
                    ntype: MktFsNodeType::BIGFILE,
                    children: Some(children),
                    hash: hash,
                    // data: None,
                }));
            }
        }

        /// Create a `MktFsNode` from a file.
        ///
        /// This method will call the `try_from_bytes` method on the contents of the file.
        pub fn try_from_file(
            path: &PathBuf,
            chunk_size: usize,
            max_children: usize,
        ) -> Result<MktFsNode> {
            // Read the file content as a `Vec<u8>`
            let content = fs::read(path).with_context(|| {
                error!(
                    "Failed to create a node for path {}.",
                    path.to_string_lossy()
                );
                format!(
                "Failed to read the file {:#}, check that the path is a valid file and permissions",
                &path.to_string_lossy()
            )
            })?;
            return MktFsNode::try_from_bytes(path, content, chunk_size, max_children);
        }

        /// Create a `MktFsNode` from a directory.
        ///
        /// This method will recursively traverse the content of the directory, building children `MktFsNode`
        /// of the adequate types.
        pub fn try_from_path(
            path: &PathBuf,
            chunk_size: usize,
            max_children: usize,
        ) -> Result<MktFsNode> {
            // Try to read the content of the directory into an iterator of directories and files etc...
            let dir = fs::read_dir(&path).with_context(|| {
                error!(
                    "Failed to create a node for path {}.",
                    path.to_string_lossy()
                );
                format!(
                "Failed to read the directory {:#}, check that the path is a valid directory and permissions",
                &path.to_string_lossy()
            )
            })?;

            // Compute the children nodes recursively matching on whether they are files or directories
            let children: Vec<MktFsNode> = dir
                .filter_map(|d| match d {
                    Ok(path) => {
                        if path.path().is_file() {
                            match MktFsNode::try_from_file(&path.path(), chunk_size, max_children) {
                                Ok(n) => Some(n),
                                Err(e) => None,
                            }
                        } else if path.path().is_dir() {
                            match MktFsNode::try_from_path(&path.path(), chunk_size, max_children) {
                                Ok(n) => Some(n),
                                Err(e) => None,
                            }
                        } else {
                            return None;
                        }
                    }
                    Err(e) => {
                        return None;
                    }
                })
                .collect();

            let mut hasher = Sha256::new();
            hasher.update([2]);
            for c in children.iter() {
                hasher.update(c.hash);
            }
            let mut hash = <[u8; 32]>::default();
            hash.copy_from_slice(hasher.finalize().as_slice());

            // Generate root node
            return (Ok(MktFsNode {
                path: path.clone(),
                ntype: MktFsNodeType::DIRECTORY,
                children: Some(children),
                hash: hash,
                // data: None,
            }));
        }

        /// Create a hashmap linking all `MktFsNode` to its hash.
        ///
        /// This method will recursively traverse the Merkle tree to build a flat hashmap referencing the nodes by their hash.
        pub fn to_hashmap(&self) -> HashMap<[u8; 32], &MktFsNode> {
            let mut hmap = HashMap::from([(self.hash, self)]);

            match &self.children {
                None => {
                    return hmap;
                }
                Some(children) => {
                    for c in children.iter() {
                        hmap.extend(c.to_hashmap());
                    }
                }
            }
            return hmap;
        }
    }

    pub fn hash_bytes(bytes: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        let mut hash = <[u8; 32]>::default();
        hash.copy_from_slice(hasher.finalize().as_slice());
        return hash;
    }

    pub fn hash_bytes_prefix(bytes: &[u8], prefix: u8) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update([prefix]);
        hasher.update(bytes);
        let mut hash = <[u8; 32]>::default();
        hash.copy_from_slice(hasher.finalize().as_slice());
        return hash;
    }

    pub fn hash_bytes_array(bytes_array: Vec<Vec<u8>>) -> [u8; 32] {
        let mut hasher = Sha256::new();
        for bytes in bytes_array.iter() {
            hasher.update(hash_bytes(bytes));
        }
        let mut hash = <[u8; 32]>::default();
        hash.copy_from_slice(hasher.finalize().as_slice());
        return hash;
    }
}

#[cfg(test)]
mod tests {

    use crate::mk_fs::*;
    use hex;
    use std::path::PathBuf;

    #[test]
    fn lib_file_hash_bytes() {
        let hash = hash_bytes(b"hello");
        assert_eq!(
            hash,
            hex::decode("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
                .unwrap()[0..32]
        );
    }

    #[test]
    fn lib_file_hash_array() {
        let hash = hash_bytes_array(Vec::from([b"hello".to_vec(), b"world".to_vec()]));
        assert_eq!(
            hash,
            hex::decode("7305db9b2abccd706c256db3d97e5ff48d677cfe4d3a5904afb7da0e3950e1e2")
                .unwrap()[0..32]
        );
    }

    #[test]
    fn lib_file_node_from_small_chunk() {
        let data = b"abc";
        let path = PathBuf::from("/root");
        let node = MktFsNode::try_from_bytes(&path, data, 4, 2).unwrap();
        assert_eq!(
            node.hash,
            [
                96, 159, 110, 54, 210, 64, 85, 133, 24, 141, 92, 253, 118, 31, 64, 124, 124, 196,
                106, 125, 63, 49, 76, 136, 39, 4, 105, 221, 227, 21, 252, 209
            ]
        )
    }

    #[test]
    fn lib_file_node_from_big_chunk() {
        //           | 4 | 4 | 4 | 4 |2|
        //           |       |       | |
        //           |               | |
        //           |                 |
        let data = b"abcdefghijklmabcde";
        let path = PathBuf::from("/root");
        let node = MktFsNode::try_from_bytes(&path, data, 4, 2).unwrap();
        assert_eq!(
            node.hash,
            [
                34, 51, 8, 184, 150, 210, 92, 191, 251, 19, 222, 157, 179, 159, 210, 155, 130, 30,
                98, 251, 142, 55, 111, 159, 195, 146, 118, 194, 231, 240, 11, 170
            ]
        );
    }

    #[test]
    #[should_panic]
    fn lib_file_node_from_path_fail() {
        let path = PathBuf::from("/var/lib/libvirt/images");
        let read_dir = MktFsNode::try_from_path(&path, 2, 2).unwrap();
    }

    #[test]
    fn lib_file_node_from_path() {
        let path = PathBuf::from("/tmp/abc");
        let node = MktFsNode::try_from_path(&path, 1024, 32).unwrap();
        // println!("{node}");
    }

    #[test]
    fn lib_file_node_to_hashmap() {
        let path = PathBuf::from("/tmp/abc");
        let node = MktFsNode::try_from_path(&path, 1024, 32).unwrap();
        let map = node.to_hashmap();
        let hash = [
            21, 208, 20, 204, 73, 14, 59, 170, 131, 72, 237, 69, 189, 154, 4, 38, 76, 225, 124,
            157, 143, 164, 78, 78, 6, 169, 69, 111, 79, 182, 40, 196,
        ];
        let result = map.get(&hash);
        match result {
            Some(r) => println!("{:?}", r),
            None => println!("Node not found for hash {:?}", hash),
        }
    }
}
