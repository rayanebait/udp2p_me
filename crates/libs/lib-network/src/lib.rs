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
    use {
        crate::{
            action::Action, congestion_handler::*, handle_packet::*, packet::*, peer::peer::*,
            store::*,
        },
        futures::{future::join_all, stream::FuturesUnordered, Future},
        prelude::*,
        std::{
            default,
            net::SocketAddr,
            /*Multi task*/
            sync::{Arc, Mutex, RwLock},
        },
        tokio::{
            join, select,
            task::{JoinError, JoinHandle},
            time::{sleep, Sleep},
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
    ) {
        let server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();

        for attempt in 0..10 {
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                Action::SendHello(None, my_data.get_name().unwrap(), server_sock_addr),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            println!("attempt : {}", attempt);

            match peek_until_hello_reply_from(
                Arc::clone(&peek_process_queue),
                Arc::clone(&process_queue_state),
                Arc::clone(&process_queue_readers_state),
                Arc::clone(&action_queue),
                Arc::clone(&action_queue_state),
                server_sock_addr,
            )
            .await
            {
                Ok(_) => {
                    keep_alive_to_peer(
                        Arc::clone(&action_queue),
                        Arc::clone(&action_queue_state),
                        *&server_sock_addr,
                        1000,
                    );
                    break;
                }
                Err(_) => continue,
            }
        }
        println!("here")
    }

    pub fn keep_alive_to_peer(
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
        timing: u64,
    ) {
        tokio::spawn(async move {
            loop {
                Queue::lock_and_push(
                    Arc::clone(&action_queue),
                    Action::SendHello(None, vec![97, 110, 105, 116], sock_addr),
                );
                QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
                std::thread::sleep(Duration::from_nanos(timing));
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

    pub async fn peek_until_hello_reply_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_state: Arc<QueueState>,
        process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        sock_addr: SocketAddr,
    ) -> Result<Action, PeerError> {
        /*Repeatedly peeks process queue to check  */
        let task_handle = tokio::spawn(async move {
            loop {
                /*Wait notify all from receive task */
                let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                    Some(front) => front,
                    None => {
                        process_queue_readers_state.wait();
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
        });
        let abort_task_handle = task_handle.abort_handle();

        let timeout_handle = tokio::spawn(async move {
            let timeout = sleep(Duration::from_secs(3)).await;
            if abort_task_handle.is_finished() {
                /*Never happens */
                return Err::<Action, PeerError>(PeerError::ResponseTimeout);
            } else {
                abort_task_handle.abort();
                return Err::<Action, PeerError>(PeerError::ResponseTimeout);
            }
        });
        /* Return when either response time out or received a packet
        corresponding */
        select! {
            timed_out = timeout_handle => {
                match timed_out {
                    Ok(result)=> match result {
                        Err(PeerError::ResponseTimeout)=> return Err(PeerError::ResponseTimeout),
                        _=>panic!("Shouldn't happen"),
                    },
                    Err(_)=> panic!("time out task panicked"),
                }
            }
            action = task_handle => {
                match action {
                    Ok(result)=> match result {
                        Ok(action)=> return Ok(action),
                        _=>panic!("Shouldn't happen"),
                    },
                    Err(_)=> panic!("time out task panicked"),
                }
            }
        };
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
    ) -> Result<(), PeerError> {
        let mut subtasks = vec![];
        let children: Option<Vec<[u8; 32]>>;

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
        )
        .await
        {
            Ok(datum_action) => {
                children = match build_tree_maps(&datum_action, Arc::clone(&maps)) {
                    Ok(childs) => match childs {
                        Some(childs) => Some(childs),
                        None => None,
                    },
                    Err(PeerError::InvalidPacket) => return Err(PeerError::InvalidPacket),
                    _ => panic!("Shouldn't happen"),
                };
            }
            Err(PeerError::ResponseTimeout) => {
                return Err(PeerError::ResponseTimeout);
                // break Err::<Action, PeerError>(PeerError::ResponseTimeout)
            }
            _ => todo!(),
        };
        match children {
            Some(childs) => {
                // let mut hash_vec = vec![];
                for child_hash in childs {
                    println!("Asking for child {:?}", &child_hash);
                    subtasks.push(fetch_subtree_from(
                        Arc::clone(&peek_process_queue),
                        Arc::clone(&process_queue_readers_state),
                        Arc::clone(&action_queue),
                        Arc::clone(&action_queue_state),
                        Arc::clone(&maps),
                        child_hash,
                        sock_addr,
                    ));
                    // hash_vec.push(Action::SendGetDatumWithHash(child_hash, sock_addr));
                }

                // Queue::lock_and_push_mul(Arc::clone(&action_queue), hash_vec);
            }
            None => {
                println!("no childs");
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

    /*Given a hash it downloads all files under it  */
    pub async fn download(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        active_peers: Arc<Mutex<ActivePeers>>,
        hash: [u8; 32],
        sock_addr: SocketAddr,
    ) {
        /*Peer has to have been handshaked before */
        /*Constructs the whole tree structure without storing it */
        // let hash_trees_or_error = fetch_subtree_from(
        //     Arc::clone(&peek_process_queue),
        //     Arc::clone(&process_queue_state),
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&active_peers),
        //     hash,
        //     sock_addr,
        // )
        // .await;

        // let (parent_to_child_map, child_to_parent_map) = match hash_trees_or_error {
        //     Ok(hash_trees)=> hash_trees,
        //     Err(PeerError::ResponseTimeout)=> todo!(),
        //     _=>todo!(),
        // };
    }

    pub async fn peek_until_datum_with_hash_from(
        peek_process_queue: Arc<RwLock<Queue<Action>>>,
        process_queue_readers_state: Arc<QueueState>,
        action_queue: Arc<Mutex<Queue<Action>>>,
        action_queue_state: Arc<QueueState>,
        hash: [u8; 32],
        sock_addr: SocketAddr,
    ) -> Result<Action, PeerError> {
        /*Spawns a task that peeks the process queue waiting for a packet
        from the given address with the given peer. */
        let task_handle = tokio::spawn(async move {
            loop {
                /*Wait notify all from receive task */
                let front = match Queue::read_lock_and_peek(Arc::clone(&peek_process_queue)) {
                    Some(front) => front,
                    None => {
                        process_queue_readers_state.wait();
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
                    _ => continue,
                }
            }
        });
        /*Handle that can be sent to a task and used to abort a given task. */
        let abort_task_handle = task_handle.abort_handle();

        /*Sleeps for the given duration then aborts the given task. */
        let timeout_handle = tokio::spawn(async move {
            let timeout = sleep(Duration::from_secs(3)).await;
            if abort_task_handle.is_finished() {
                /*Never happens */
                return Err::<Action, PeerError>(PeerError::ResponseTimeout);
            } else {
                abort_task_handle.abort();
                return Err::<Action, PeerError>(PeerError::ResponseTimeout);
            }
        });
        /* Return when either response time out or received a packet
        corresponding */
        select! {
            timed_out = timeout_handle => {
                match timed_out {
                    Ok(result)=> match result {
                        Err(PeerError::ResponseTimeout)=> return Err(PeerError::ResponseTimeout),
                        _=>panic!("Shouldn't happen"),
                    },
                    Err(_)=> panic!("time out task panicked"),
                }
            }
            action = task_handle => {
                match action {
                    Ok(result)=> match result {
                        Ok(action)=> return Ok(action),
                        _=>panic!("Shouldn't happen"),
                    },
                    Err(_)=> panic!("time out task panicked"),
                }
            }
        };
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{
            congestion_handler::*, handle_action::handle_action_task,
            handle_packet::handle_packet_task, packet::*, peer::peer::*, process::process_task,
            sender_receiver::*, store::*, task_launcher_canceller::task_launcher,
        },
        futures::join,
        import_export::*,
        lib_file::mk_fs::{self, MktFsNode},
        nanorand::{wyrand::WyRand, BufferedRng, Rng},
        std::sync::{Arc, Mutex},
        std::{net::SocketAddr, path::PathBuf},
        tokio::{
            self,
            net::UdpSocket,
            runtime,
            runtime::Handle,
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
        packet
            .send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap())
            .await;
        sleep(Duration::from_millis(500));
        packet2
            .send_to_addr(&sock, &"176.169.27.221:9157".parse().unwrap())
            .await;
        sleep(Duration::from_millis(500));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn register_and_export() {
        let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        // let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());

        let tree = MktFsNode::try_from_path(
            &PathBuf::from("/home/splash/files/notes_m2/protocoles_reseaux/tp/Projet/to_export/"),
            1024,
            100,
        )
        .expect("unexisting path");

        let map = Arc::new(tree.to_hashmap());

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
        my_data.set_name(vec![97, 110, 105, 116]);
        let my_data = Arc::new(my_data.clone());

        let receiving = receiver(
            Arc::clone(&sock),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        let handling = handle_packet_task(
            Arc::clone(&pending_ids),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
        );
        let processing_two = handle_action_task(
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
        );
        let processing_one = process_task(
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&active_peers),
            Arc::clone(&my_data),
            // Arc::clone(&map)
        );

        let sending = sender(
            Arc::clone(&sock),
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&pending_ids),
        );

        // let metrics = Handle::current().metrics();
        // let n = metrics.active_tasks_count();
        // println!("Runtime has {} active tasks", n);
        let registering = register(
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&my_data),
        );

        sleep(Duration::from_secs(1000));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn register_and_fetch_tree() {
        // let sock = Arc::new(UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000) ).await.unwrap());
        let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
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
        my_data.set_name(vec![97, 110, 105, 116]);
        let my_data = Arc::new(my_data);

        task_launcher(queues, active_peers.clone(), my_data.clone(), sock.clone());

        /*jch */
        // let sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        // let sock_addr: SocketAddr = "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        /*yoan */
        // let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        /*Pas derriere un nat */
        // let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr ="178.132.106.168:33313".parse().unwrap();
        /*derriere un nat */
        let sock_addr: SocketAddr = "81.65.148.210:40214".parse().unwrap();
        {
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                action::Action::SendHello(None, vec![97, 110, 105, 116], *&sock_addr),
            );
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                action::Action::SendHello(None, vec![97, 110, 105, 116], *&sock_addr),
            );
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                action::Action::SendHello(None, vec![97, 110, 105, 116], *&sock_addr),
            );
        }
        sleep(Duration::from_secs(1)).await;
        // let hash = [
        //     211, 20, 115, 228, 84, 20, 231, 30, 31, 144, 12, 151, 66, 10, 253, 48, 29, 89, 243,
        //     191, 123, 136, 76, 8, 147, 130, 48, 109, 255, 40, 26, 48,
        // ];
        let peer_hash = {
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                action::Action::SendRoot(None, sock_addr),
            );
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            receive_queue_state.wait();
            let guard = active_peers.lock().unwrap();
            let peer = guard.addr_map.get(&sock_addr).unwrap();
            peer.get_root_hash().unwrap()
        };

        // keep_alive_to_peer(Arc::clone(&action_queue), Arc::clone(&action_queue_state), *&sock_addr);
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

        let child_hash: [u8; 32] = [
            115, 50, 75, 0, 182, 12, 186, 29, 161, 254, 249, 93, 93, 212, 161, 125, 206, 44, 148,
            135, 173, 127, 64, 161, 49, 67, 77, 10, 174, 213, 70, 250,
        ];

        match maps.lock() {
            Ok(m) => {
                println!("Name = {}", get_full_name(&child_hash, &m.2, &m.0));
                println!("{:?}", get_name_to_hash_hashmap(&m.0, &m.2));
            }
            _ => (),
        };
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn keep_alive() {
        // let sock = Arc::new(
        //     UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
        //         .await
        //         .unwrap(),
        // );
        let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        let maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);
        let send_queue = Arc::clone(&queues.1);
        let send_queue_state = Arc::clone(&queues.7);

        let mut my_data = Peer::new();
        my_data.set_name(vec![97, 110, 105, 116]);
        let my_data = Arc::new(my_data);

        task_launcher(queues, active_peers.clone(), my_data.clone(), sock.clone());

        /*jch */
        let server_sock_addr4: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        // let sock_addr: SocketAddr = "[2001:660:3301:9200::51c2:1b9b]:8443".parse().unwrap();
        /*yoan */
        // let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        /*Pas derriere un nat */
        // let sock_addr: SocketAddr = "82.66.83.225:8000".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr ="178.132.106.168:33313".parse().unwrap();
        /*derriere un nat */
        let sock_addr: SocketAddr = "81.65.148.210:40214".parse().unwrap();
        // let sock_addr: SocketAddr = "[2a02:842a:853e:b501:7211:24ff:fe8c:d728]:28469".parse().unwrap();
        /*derriere un nat */
        // let sock_addr: SocketAddr = "81.220.68.200:23834".parse().unwrap();

        /*for handshake */
        keep_alive_to_peer(
            action_queue.clone(),
            action_queue_state.clone(),
            server_sock_addr4,
            /*en nanosecs */
            30000000000,
        );
        {
            Queue::lock_and_push(
                send_queue.clone(),
                (
                    PacketBuilder::hello_packet(None, my_data.get_name().unwrap()),
                    sock_addr,
                ),
            );
            QueueState::set_non_empty_queue(send_queue_state.clone());
        }
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
        // let sock = Arc::new(
        //     UdpSocket::bind(SocketAddr::new("::1".parse().unwrap(), 40000))
        //         .await
        //         .unwrap(),
        // );
        let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
        // let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        let maps = build_tree_mutex();
        let queues = build_queues();
        let active_peers = ActivePeers::build_mutex();

        let action_queue = Arc::clone(&queues.2);
        let action_queue_state = Arc::clone(&queues.6);

        let mut my_data = Peer::new();
        my_data.set_name(vec![97, 110, 105, 116]);
        let my_data = Arc::new(my_data);

        task_launcher(queues, active_peers.clone(), my_data.clone(), sock.clone());

        /*jch */
        let server_sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();
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

        keep_alive_to_peer(
            action_queue.clone(),
            action_queue_state.clone(),
            server_sock_addr,
            /*en nanosecs */
            1000000000,
        );
        sleep(Duration::from_secs(2)).await;

        try_nat_traversal_with(
            action_queue.clone(),
            action_queue_state.clone(),
            sock_addr_vec,
            /*en nanosecs */
            1000000,
        );
        keep_alive_to_peer(
            action_queue,
            action_queue_state,
            sock_addr,
            /*en nanosecs */
            9000000000,
        );
        sleep(Duration::from_secs(1000)).await;
        println!("main ends");
    }
}
