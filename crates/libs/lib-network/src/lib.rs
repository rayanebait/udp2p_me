pub mod action;
pub mod congestion_handler;
pub mod handle_action;
pub mod handle_packet;
pub mod packet;
pub mod peer;
pub mod process;
pub mod resend;
pub mod sender_receiver;
pub mod store;
pub mod task_launcher_canceller;

pub mod import_export {
    use std::pin::Pin;

    use {
        crate::{action::Action, congestion_handler::*, peer::*, store::*},
        futures::{future::join_all, Future},
        log::{debug, error, info, warn},
        prelude::*,
        std::{
            net::SocketAddr,
            /*Multi task*/
            sync::{Arc, Mutex, RwLock},
        },
    };

    /*Sends Hello then peeks the process queue to check for a helloreply, if no
    helloreply in 3 seconds, sends hello again, repeats 3 times.

    */
    pub async fn register(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_state: Arc<QueueState>,
        process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        my_data: Arc<Peer>,
    ) -> Result<(), PeerError> {
        let server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();

        handshake(
            peek_process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            server_sock_addr,
            my_data.clone(),
        );

        match peek_until_hello_reply_from(
            Arc::clone(&peek_process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            server_sock_addr,
            1000,
        )
        .await
        {
            Ok(_) => {
                keep_alive_to_peer(
                    Arc::clone(&action_queue),
                    Arc::clone(&action_queue_state),
                    *&server_sock_addr,
                    my_data.clone(),
                    30_000_000_000,
                );
                debug!("Register OK");
                return Ok(());
            }
            Err(_) => {
                debug!("Register failed");
                return Err(PeerError::ResponseTimeout);
            }
        };
    }

    pub fn keep_alive_to_peer(
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
        my_data: Arc<Peer>,
        timing: u64,
    ) {
        tokio::spawn(async move {
            loop {
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    Action::SendHello(
                        None,
                        my_data.get_name().unwrap().as_bytes().to_vec(),
                        sock_addr,
                    ),
                );
                QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));

                std::thread::sleep(Duration::from_millis(timing));
            }
        });
    }

    pub fn try_nat_traversal_with(
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        sock_addr: Vec<u8>,
        timing: u64,
    ) {
        tokio::spawn(async move {
            let server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();
            loop {
                std::thread::sleep(Duration::from_nanos(timing));
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    Action::SendNatTraversalRequest(sock_addr.clone(), server_sock_addr),
                );
                QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            }
        });
    }

    pub async fn peek_until_root_reply_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        _process_queue_state: Arc<QueueState>,
        process_queue_readers_state: Arc<QueueState>,
        _action_queue: Arc<Mutex<Queue<Action>>>,
        _action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
        timeout: u64,
    ) -> Result<Action, PeerError> {
        loop {
            /*Wait notify all from receive task */
            let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                Some(front) => front,
                None => {
                    match process_queue_readers_state.wait_timeout_ms(timeout) {
                        Ok(_) => (),
                        Err(_) => return Err(PeerError::PeerTimedOut),
                    };
                    continue;
                }
            };
            match front {
                Action::ProcessRootReply(_hash, addr) => {
                    if addr == sock_addr {
                        break Ok::<Action, PeerError>(front);
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
        }
    }
    pub async fn peek_until_hello_reply_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        _process_queue_state: Arc<QueueState>,
        process_queue_readers_state: Arc<QueueState>,
        _action_queue: Arc<Mutex<Queue<Action>>>,
        _action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
        timeout: u64,
    ) -> Result<Action, PeerError> {
        loop {
            /*Wait notify all from receive task */
            let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                Some(front) => front,
                None => {
                    match process_queue_readers_state.wait_timeout_ms(timeout) {
                        Ok(_) => (),
                        Err(_) => return Err(PeerError::PeerTimedOut),
                    };
                    continue;
                }
            };
            match front {
                Action::ProcessHelloReply(.., addr) => {
                    if addr == sock_addr {
                        break Ok::<Action, PeerError>(front);
                    } else {
                        continue;
                    }
                }
                _ => continue,
            }
        }
    }

    #[async_recursion::async_recursion]
    pub async fn fetch_subtree_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        maps: Arc<
            Mutex<(
                HashMap<[u8; 32], [u8; 32]>,
                HashMap<[u8; 32], Vec<[u8; 32]>>,
                HashMap<[u8; 32], String>,
            )>,
        >,
        hash: [u8; 32],
        sock_addr: SocketAddr,
        timeout: u64,
    ) -> Result<(), PeerError> {
        let mut subtasks = vec![];
        let children: Option<Vec<[u8; 32]>>;

        // Send a get datum with the first target hash
        Queue::lock_and_push(
            Arc::clone(&action_queue),
            Action::SendGetDatumWithHash(*&hash, *&sock_addr),
        );
        QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));

        match peek_until_datum_with_hash_from(
            Arc::clone(&peek_process_queue),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            *&hash,
            *&sock_addr,
            timeout,
        )
        .await
        {
            Ok(datum_action) => {
                // build the maps :
                // - child -> parent
                // - parent -> child
                // - hash -> name
                // and return the children if there are some
                // - chunk -> no children
                // - tree -> no children (fetching only the filesystem)
                // - directory -> children
                children = match build_tree_maps(&datum_action, Arc::clone(&maps)) {
                    Ok(childs) => match childs {
                        Some(childs) => Some(childs),
                        None => None,
                    },
                    Err(PeerError::InvalidPacket) => return Err(PeerError::InvalidPacket),
                    _ => {
                        error!("Should not happen");
                        panic!("Shouldn't happen")
                    }
                };
            }
            Err(PeerError::NoDatum) => children = None,
            Err(PeerError::ResponseTimeout) => {
                return Err(PeerError::ResponseTimeout);
                // break Err::<Action, PeerError>(PeerError::ResponseTimeout)
            }
            Err(PeerError::PeerTimedOut)=> return Err(PeerError::PeerTimedOut),
            _=> return Err(PeerError::Unknown),
        };
        match children {
            Some(childs) => {
                // let mut hash_vec = vec![];
                for child_hash in childs {
                    debug!("Asking for child {:?}", &child_hash);
                    subtasks.push(fetch_subtree_from(
                        Arc::clone(&peek_process_queue),
                        Arc::clone(&process_queue_readers_state),
                        Arc::clone(&action_queue),
                        Arc::clone(&action_queue_state),
                        Arc::clone(&maps),
                        child_hash,
                        sock_addr,
                        timeout,
                    ));
                    // hash_vec.push(Action::SendGetDatumWithHash(child_hash, sock_addr));
                }

                // Queue::lock_and_push_mul(Arc::clone(&action_queue), hash_vec);
            }
            None => {
                debug!("no childs");
                return Ok(());
            }
        };
        let completed = join_all(subtasks).await;
        for result in completed {
            match result {
                Ok(()) => continue,
                Err(e) => return Err(e),
            }
        }

        Ok(())
    }

    #[async_recursion::async_recursion]
    pub async fn download_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        maps: Arc<
            Mutex<(
                HashMap<[u8; 32], [u8; 32]>,
                HashMap<[u8; 32], Vec<[u8; 32]>>,
                HashMap<[u8; 32], String>,
            )>,
        >,
        hash: [u8; 32],
        sock_addr: SocketAddr,
        timeout: u64,
    ) -> Result<SimpleNode, PeerError> {
        let _subtasks: Vec<Pin<Box<dyn Future<Output = Result<SimpleNode, PeerError>> + Send>>> =
            vec![];
        let _children: Option<Vec<[u8; 32]>>;

        // Send a get datum with the first target hash
        Queue::lock_and_push(
            Arc::clone(&action_queue),
            Action::SendGetDatumWithHash(*&hash, *&sock_addr),
        );
        QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));

        match peek_until_datum_with_hash_from(
            Arc::clone(&peek_process_queue),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            *&hash,
            *&sock_addr,
            timeout,
        )
        .await
        {
            Ok(datum_action) => {
                // build the maps :
                // - child -> parent
                // - parent -> child
                // - hash -> name
                // and return the children if there are some
                // - chunk -> no children
                // - tree -> no children (fetching only the filesystem)
                // - directory -> children
                let data_type = get_type(&datum_action)?;
                match data_type {
                    0 | 1 => {
                        debug!("Selected hash is a file, downloading it");
                        let mut node = match get_children(&datum_action, Arc::clone(&maps)) {
                            Ok(n) => n,
                            Err(_e) => {
                                error!("Failed to download datum");
                                return Err(PeerError::InvalidPacket);
                            }
                        };
                        match node.children {
                            Some(c) => {
                                let mut subtasks = vec![];
                                for n in c.into_iter() {
                                    subtasks.push(download_from(
                                        Arc::clone(&peek_process_queue),
                                        Arc::clone(&process_queue_readers_state),
                                        Arc::clone(&action_queue),
                                        Arc::clone(&action_queue_state),
                                        Arc::clone(&maps),
                                        n.hash,
                                        sock_addr,
                                        timeout,
                                    ));
                                }
                                let completed = join_all(subtasks).await;
                                node.children = Some(
                                    completed
                                        .into_iter()
                                        .filter_map(|n| match n {
                                            Ok(r) => Some(r),
                                            Err(e) => {
                                                warn!(
                                                    "Failed to download child, file may be corrupted. {e}"
                                                );
                                                None
                                            }
                                        })
                                        .collect::<Vec<SimpleNode>>(),
                                );
                            }
                            None => (),
                        }
                        return Ok(node);
                    }
                    2 => {
                        info!("Selected hash is a directory. Fetching the file tree.");
                        let _ = fetch_subtree_from(
                            Arc::clone(&peek_process_queue),
                            Arc::clone(&process_queue_readers_state),
                            Arc::clone(&action_queue),
                            Arc::clone(&action_queue_state),
                            Arc::clone(&maps),
                            hash,
                            sock_addr,
                            timeout,
                        )
                        .await?;
                        return Err(PeerError::FileIsDirectory);
                    }
                    _ => return Err(PeerError::InvalidPacket),
                }
            }
            Err(PeerError::NoDatum) => return Err(PeerError::NoDatum),
            Err(PeerError::ResponseTimeout) => return Err(PeerError::ResponseTimeout),
            Err(_e) => return Err(PeerError::Unknown),
        };
    }

    pub async fn peek_until_datum_with_hash_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_readers_state: Arc<QueueState>,
        _action_queue: Arc<Mutex<Queue<Action>>>,
        _action_queue_state: Arc<QueueState>,
        hash: [u8; 32],
        sock_addr: SocketAddr,
        timeout: u64,
    ) -> Result<Action, PeerError> {
        /*Spawns a task that peeks the process queue waiting for a packet
        from the given address with the given peer. */

        loop {
            /*Wait notify all from receive task */
            let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                Some(front) => front,
                None => {
                    match process_queue_readers_state.wait_timeout_ms(timeout) {
                        Ok(_) => (),
                        Err(_) => return Err(PeerError::PeerTimedOut),
                    };
                    continue;
                }
            };
            match front {
                Action::ProcessDatum(datum, addr) => {
                    let mut datum_hash = [0u8; 32];
                    datum_hash.copy_from_slice(&datum.as_slice()[0..32]);
                    if (addr == sock_addr) && (datum_hash == hash) {
                        break Ok::<Action, PeerError>(Action::ProcessDatum(datum, addr));
                    } else {
                        continue;
                    }
                }
                Action::ProcessNoDatum(_addr) => break Err(PeerError::NoDatum),
                _ => continue,
            }
        }
    }

    pub fn handshake(
        _peek_process_queue: Arc<RwLock<Queue<Action>>>,
        _process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
        my_data: Arc<Peer>,
    ) {
        Queue::lock_and_push_mul(
            action_queue.clone(),
            vec![
                Action::SendHello(
                    None,
                    my_data.get_name().unwrap().as_bytes().to_vec(),
                    sock_addr,
                ),
                Action::SendRoot(None, sock_addr),
                Action::SendPublicKey(None, sock_addr),
            ],
        );
        Queue::lock_and_push_mul(
            action_queue,
            vec![
                Action::SendHello(
                    None,
                    my_data.get_name().unwrap().as_bytes().to_vec(),
                    sock_addr,
                ),
                Action::SendRoot(None, sock_addr),
                Action::SendPublicKey(None, sock_addr),
            ],
        );
        QueueState::set_non_empty_queue(action_queue_state);
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            congestion_handler::*, handle_action::handle_action_task,
            handle_packet::handle_packet_task, packet::*, peer::*, process::process_task,
            sender_receiver::*, store::*, task_launcher_canceller::task_launcher,
        },
        import_export::*,
        lib_file::mk_fs::MktFsNode,
        nanorand::{wyrand::WyRand, BufferedRng, Rng},
        std::sync::Arc,
        std::{net::SocketAddr, path::PathBuf},
        tokio::{
            self,
            net::UdpSocket,
            time::{sleep, Duration},
        },
    };
    #[tokio::test]
    async fn packet_bytes_conversion() {
        let mut rng = BufferedRng::new(WyRand::new());
        let mut rand_id: [u8; 4] = [0; 4];
        rng.fill(&mut rand_id);
        drop(rng);

        let packet = PacketBuilder::new()
            .set_id(rand_id.clone())
            .body(b"what is this".to_vec())
            .packet_type(PacketType::Datum)
            .build()
            .unwrap();

        let hello_packet = PacketBuilder::noop_packet();
        let raw_hello_packet = hello_packet.as_bytes();
        let raw_packet = packet.as_bytes();

        println!("{:?}\n{:?}", raw_packet, packet);
        println!("{:?}\n{:?}", raw_hello_packet, hello_packet);
    }

    #[tokio::test]
    async fn handshake_with_pi() {
        let packet = PacketBuilder::noop_packet();
        let packet2 = PacketBuilder::new()
            .body(b"j'ai rotey :)".to_vec())
            .gen_id()
            .build()
            .unwrap();
        let sock = UdpSocket::bind("172.20.10.7:0").await.unwrap();
        let _ = packet
            .send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap())
            .await;
        let _ = sleep(Duration::from_millis(500));
        let _ = packet2
            .send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap())
            .await;
        let _ = sleep(Duration::from_millis(500));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn register_and_export() {
        let sock4 = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        // let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());

        let tree = MktFsNode::try_from_path(
            &PathBuf::from("/home/splash/files/notes_m2/protocoles_reseaux/tp/Projet/to_export/"),
            1024,
            100,
        )
        .expect("unexisting path");

        let _map = Arc::new(tree.to_hashmap());

        let (
            receive_queue,
            send_queue,
            action_queue,
            process_queue,
            pending_ids,
            receive_queue_state,
            action_queue_state,
            send_queue_state,
            process_queue_state,
            process_queue_readers_state,
        ) = build_queues();

        let active_peers = ActivePeers::build_mutex();

        let mut my_data = Peer::new();
        my_data.set_name("nist".to_string());
        let my_data = Arc::new(my_data.clone());

        let _receiving = receiver4(
            Arc::clone(&sock4),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        let _handling = handle_packet_task(
            Arc::clone(&pending_ids),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
        );
        let _processing_two = handle_action_task(
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
        );
        let _processing_one = process_task(
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&active_peers),
            Peer::new(),
            false,
            PathBuf::from("/home/splash/"),
            // Arc::clone(&map)
        );

        let _sending = sender(
            Arc::clone(&sock4),
            Arc::clone(&sock4),
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&pending_ids),
        );

        // let metrics = Handle::current().metrics();
        // let n = metrics.active_tasks_count();
        // println!("Runtime has {} active tasks", n);
        let _registering = register(
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&my_data),
        );

        let _ = sleep(Duration::from_secs(1_000));
    }

    /*Currently seems to sometime not be able to register peer.
    Sometimes when receiving helloreply, doesn't even attempt to create peer.  */
    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn register_and_fetch_tree() {
        env_logger::init();
        let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        /*DON'T USE ::1 for ipv6. Doesn't work. Seems like it
        binds to an ipv6 compatible ipv4 address and not a true
        ipv6 address so the socket doesn't send anything. */
        let sock6 = Arc::new(
            UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 0))
                .await
                .unwrap(),
        );
        // let sock6 = Arc::new(
        //     UdpSocket::bind(SocketAddr::new(
        //         "fdb0:ccfe:b9b5:b600:47a1:849c:2d22:9ce9".parse().unwrap(),
        //         // "2001:861:36c2:cdf0:4572:dd2e:473a:4081".parse().unwrap(),
        //         0,
        //     ))
        //     .await
        //     .unwrap(),
        // );

        let maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let _receive_queue_state = Arc::clone(&queues.5);
        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);
        let process_queue = Arc::clone(&queues.3);
        let process_queue_state = Arc::clone(&queues.8);
        let process_queue_readers_state = Arc::clone(&queues.9);

        let mut my_data = Peer::new();
        my_data.set_name("nist".to_string());
        let my_data_own = my_data.clone();
        let my_data = Arc::new(my_data);

        task_launcher(
            queues,
            active_peers.clone(),
            my_data.clone(),
            my_data_own,
            sock4.clone(),
            sock6.clone(),
            false,
            PathBuf::from("/home/splash/files/dls"),
        );

        /*jch */
        let _server_sock_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        // let server_sock_addr6: SocketAddr = "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        /*yoan */
        // let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        /*Pas derriere un nat */
        let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "178.132.106.168:33313".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.65.148.210:40214".parse().unwrap();
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            _server_sock_addr4,
            my_data.clone(),
        );
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr,
            my_data.clone(),
        );
        // let hash = [
        //     211, 20, 115, 228, 84, 20, 231, 30, 31, 144, 12, 151, 66, 10, 253, 48, 29, 89, 243,
        //     191, 123, 136, 76, 8, 147, 130, 48, 109, 255, 40, 26, 48,
        // ];
        let peer_hash = match {
            let mut attempt = 0;
            loop {
                if attempt < 4 {
                    attempt += 1;
                } else {
                    break None;
                }
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    action::Action::SendRoot(None, _server_sock_addr4),
                );
                QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                process_queue_state.wait();
                sleep(Duration::from_millis(100)).await;
                let guard = active_peers.lock().unwrap();
                /*If panics here, means the packet received had invalid hash (body length<32) */
                match guard.get(*&_server_sock_addr4) {
                    Some(peer) => break peer.get_root_hash(),
                    None => continue,
                }
            }
        } {
            Some(peer_hash) => peer_hash,
            None => return,
        };

        // keep_alive_to_peer(Arc::clone(&action_queue), Arc::clone(&action_queue_state), *&sock_addr);
        let fetch1 = download_from(
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&maps),
            // yoan_hash,
            peer_hash,
            _server_sock_addr4,
            10000,
        );
        let _ = fetch1.await;
        // let fetch2 = fetch_subtree_from(
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_readers_state),
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&maps),
        //     // yoan_hash,
        //     peer_hash,
        //     sock_addr,
        // );

        // join!(fetch1, fetch2);

        // let child_hash: [u8; 32] = [
        //     115, 50, 75, 0, 182, 12, 186, 29, 161, 254, 249, 93, 93, 212, 161, 125, 206, 44, 148,
        //     135, 173, 127, 64, 161, 49, 67, 77, 10, 174, 213, 70, 250,
        // ];

        match maps.lock() {
            Ok(m) => {
                // println!("Name = {}", get_full_name(&child_hash, &m.2, &m.0));
                println!("{:?}", get_name_to_hash_hashmap(&m.0, &m.2).keys());
                // println!("Final hash to name {:?}", &m.2);
            }
            _ => (),
        };
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn register_and_fetch_file() {
        // env_logger::init();
        let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let sock6 = Arc::new(
            UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 0))
                .await
                .unwrap(),
        );
        // let sock6 = Arc::new(
        //     UdpSocket::bind(SocketAddr::new(
        //         "fdb0:ccfe:b9b5:b600:47a1:849c:2d22:9ce9".parse().unwrap(),
        //         // "2001:861:36c2:cdf0:4572:dd2e:473a:4081".parse().unwrap(),
        //         0,
        //     ))
        //     .await
        //     .unwrap(),
        // );

        let maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let _receive_queue_state = Arc::clone(&queues.5);
        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);
        let process_queue = Arc::clone(&queues.3);
        let _process_queue_state = Arc::clone(&queues.8);
        let process_queue_readers_state = Arc::clone(&queues.9);

        let mut my_data= Peer::new();
        my_data.set_name("nist".to_string());
        let my_data_own = my_data.clone();
        let my_data = Arc::new(my_data);

        task_launcher(
            queues,
            active_peers.clone(),
            my_data.clone(),
            my_data_own,
            sock4.clone(),
            sock6.clone(),
            false,
            PathBuf::from("/home/splash/files/dls"),
        );

        /*jch */
        let server_sock_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        /*droso-srv */
        let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            server_sock_addr4.clone(),
            my_data.clone(),
        );
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr.clone(),
            my_data.clone(),
        );

        let root_hash = <[u8; 32]>::try_from(
            hex::decode("19629b381026ff80f9b72ff6ea1a06ce6942081fdfa251611a950c081cd99629")
                .unwrap(),
        )
        .unwrap();

        let _file_hash = <[u8; 32]>::try_from(
            hex::decode("4080d430508e6f1c68a19ee5f94466455a6ad430b24a864a4e4344135a88f5d2")
                .unwrap(),
        )
        .unwrap();

        let file = download_from(
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&maps),
            // yoan_hash,
            root_hash,
            server_sock_addr4,
            10000,
        )
        .await;

        // let file = download_file(
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_readers_state),
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&maps),
        //     file_hash,
        //     server_sock_addr4,
        // )
        // .await;

        match file {
            Ok(f) => println!("Got file {f:?}"),
            Err(_e) => println!("Failed to get file"),
        }
        println!("FINISHED");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn keep_alive() {
        let sock6 = Arc::new(
            UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
                .await
                .unwrap(),
        );
        // let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let _maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);
        let _send_queue = Arc::clone(&queues.1);
        let _send_queue_state = Arc::clone(&queues.7);
        let process_queue = Arc::clone(&queues.3);
        let _process_queue_state = Arc::clone(&queues.8);
        let process_queue_readers_state = Arc::clone(&queues.9);

        let mut my_data= Peer::new();
        my_data.set_name("nist".to_string());
        let my_data_own = my_data.clone();
        let my_data = Arc::new(my_data);

        task_launcher(
            queues,
            active_peers.clone(),
            my_data.clone(),
            my_data_own,
            sock4.clone(),
            sock6.clone(),
            false,
            PathBuf::from("/home/splash/files/dls"),
        );

        /*jch */
        let server_sock_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        // let sock_addr: SocketAddr = "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        /*yoan */
        // let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        /*Pas derriere un nat */
        let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr ="178.132.106.168:33313".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.65.148.210:40214".parse().unwrap();
        // let sock_addr: SocketAddr = "[2a02:842a:853e:b501:7211:24ff:fe8c:d728]:28469".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.220.68.200:23834".parse().unwrap();

        /*for handshake */
        keep_alive_to_peer(
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr,
            my_data.clone(),
            /*en nanosecs */
            5000000000,
        );
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            server_sock_addr4.clone(),
            my_data.clone(),
        );
        handshake(
            process_queue.clone(),
            process_queue_readers_state.clone(),
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr.clone(),
            my_data.clone(),
        );
        // keep_alive_to_peer(
        //     action_queue,
        //     action_queue_state,
        //     sock_addr,
        //     /*en nanosecs */
        //     3000000000,
        // );
        sleep(Duration::from_secs(1000)).await;
        println!("main ends");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn attempt_nat_traversal() {
        let sock6 = Arc::new(
            UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
                .await
                .unwrap(),
        );
        let sock4 = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        // let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        let _maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);

        let mut my_data= Peer::new();
        my_data.set_name("nist".to_string());
        let my_data_own = my_data.clone();
        let my_data = Arc::new(my_data);

        task_launcher(
            queues,
            active_peers.clone(),
            my_data.clone(),
            my_data_own,
            sock4.clone(),
            sock6.clone(),
            false,
            PathBuf::from("/home/splash/files/dls"),
        );

        /*jch */
        let _server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        // let sock_addr: SocketAddr = "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        /*yoan */
        // let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        /*Pas derriere un nat */
        let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        let sock_addr_vec = vec![82, 66, 83, 225, 31, 64];
        /*derriere un nat */
        // let sock_addr: SocketAddr ="178.132.106.168:33313".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.65.148.210:40214".parse().unwrap();
        // let sock_addr: SocketAddr = "[2a02:842a:853e:b501:7211:24ff:fe8c:d728]:28469".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.220.68.200:23834".parse().unwrap();
        // let sock_addr: SocketAddr = "81.220.68.200:23834".parse().unwrap();

        /* derriere un nat*/
        // let sock_addr : SocketAddr = "81.65.148.210:40214".parse().unwrap();
        // let sock_addr_vec = vec![81, 65, 148, 210, 157, 22];
        /*derriere un nat */
        // let sock_addr_vec = b"46.193.66.81:56766".to_vec();
        // let sock_addr_vec = vec![46, 193, 66, 81, 221, 190];
        /*Peer disappears when attempt nat trav */
        // let sock_addr : SocketAddr = "80.215.153.182:2023".parse().unwrap();
        // let sock_addr_vec = vec![80, 215, 153, 182, 7, 231];

        // /*Disappears */
        // let sock_addr : SocketAddr = "90.3.112.55:36546".parse().unwrap();
        // let sock_addr_vec = vec![90, 3, 112, 55, 142, 194];

        sleep(Duration::from_secs(2)).await;

        try_nat_traversal_with(
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr_vec,
            /*en nanosecs */
            1_000_000,
        );
        keep_alive_to_peer(
            action_queue,
            action_queue_state,
            sock_addr,
            my_data.clone(),
            /*en nanosecs */
            9_000_000_000,
        );
        sleep(Duration::from_secs(1_000)).await;
        println!("main ends");
    }
}
