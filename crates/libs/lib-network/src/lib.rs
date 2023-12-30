pub mod action;
pub mod congestion_handler;
pub mod handle_action;
pub mod handle_packet;
pub mod packet;
pub mod peer;
pub mod process;
pub mod sender_receiver;
pub mod store;
pub mod resend;

pub mod import_export {
    use {
        prelude::*,
        std::{
            default,
            future::IntoFuture,
            /*Multi task*/
            sync::{Arc, Mutex},
            net::SocketAddr,
        },

        /*Utilities */
        anyhow::{bail, Context, Result},
        log::{debug, error, info, warn},

        /*Async/net libraries */
        futures::stream::FuturesUnordered,
        futures::{select, Future, StreamExt},
        tokio::{
            net::UdpSocket,
            test,
        },

        crate::{
            congestion_handler::*,
            handle_packet::*,
            packet::*,
        },
    };

    pub enum Error {
        Packet(PacketError),
        // Peer(PeerError),
    }

}

#[cfg(test)]
mod tests {
    use {
        std::{net::SocketAddr, path::PathBuf},

        super::*,

        nanorand::{wyrand::WyRand, BufferedRng, Rng},

        futures::join,
        import_export::*,
        std::sync::{Arc, Mutex},
        tokio::{
            self,
            net::UdpSocket,
            runtime,
            runtime::Handle,
            time::{sleep, Duration},
        },

        crate::{
            congestion_handler::*,
            handle_action::handle_action_task,
            handle_packet::handle_packet_task,
            packet::*,
            peer::peer::*,
            process::*,
            sender_receiver::*,
            store::*,
        },

        lib_file::mk_fs::{self, MktFsNode},
    };
    // #[test]
    // fn it_works() {
    //     let result = add(2, 2);
    //     assert_eq!(result, 4);
    // }
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

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn register_to_server() {
        /*0 lets the os assign the port. The port is
        then accessible with the local_addr method */
        let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
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
        {
            let packet = PacketBuilder::noop_packet();
            let sock_addr = "176.169.27.221:9157".parse().unwrap();
            Queue::lock_and_push(Arc::clone(&receive_queue), (packet, sock_addr));
            let packet = PacketBuilder::noop_packet();
            let sock_addr2 = "176.169.27.221:37086".parse().unwrap();
            Queue::lock_and_push(Arc::clone(&receive_queue), (packet, sock_addr2))
        }
        /*Do a launch tasks func ? */
        let f1 = receiver(
            Arc::clone(&sock),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
        );

        let f2 = handle_packet_task(
            Arc::clone(&pending_ids),
            Arc::clone(&receive_queue),
            Arc::clone(&receive_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
            Arc::clone(&process_queue_readers_state),
        );
        let f3 = handle_action_task(
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_state),
        );
        // let f4 = process_task(
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_state),
        //     Arc::clone(&active_peers),
        //     Arc::clone(&my_data),
        //     Arc::clone()
        // );

        let f5 = sender(
            Arc::clone(&sock),
            Arc::clone(&send_queue),
            Arc::clone(&send_queue_state),
            Arc::clone(&pending_ids),
        );

        // let metrics = Handle::current().metrics();
        // let n = metrics.active_tasks_count();
        // println!("Runtime has {} active tasks", n);

        join!(
            f1, f2, f3, //  f4,
            f5
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 10)]
    async fn register_to_server2() {
        /*0 lets the os assign the port. The port is
        then accessible with the local_addr method */
        // let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());

        let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());
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

        join!(
            receiving,
            handling,
            processing_one,
            processing_two,
            sending,
            registering
        );
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

        join!(
            receiving,
            handling,
            processing_one,
            processing_two,
            sending,
            registering
        );
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 100)]
    async fn register_and_fetch_tree() {
        let sock = Arc::new(UdpSocket::bind("192.168.1.90:40000").await.unwrap());
        // let sock = Arc::new(UdpSocket::bind("0.0.0.0:0").await.unwrap());

        let maps = build_tree_mutex();

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

        // let receiving = receiver(
        //     Arc::clone(&sock),
        //     Arc::clone(&receive_queue),
        //     Arc::clone(&receive_queue_state),
        // );

        // let handling = handle_packet_task(
        //     Arc::clone(&pending_ids),
        //     Arc::clone(&receive_queue),
        //     Arc::clone(&receive_queue_state),
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_state),
        //     Arc::clone(&process_queue_readers_state),
        // );
        // let processing_two = handle_action_task(
        //     Arc::clone(&send_queue),
        //     Arc::clone(&send_queue_state),
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_state),
        // );
        // let processing_one = process_task(
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_state),
        //     Arc::clone(&active_peers),
        //     Arc::clone(&my_data),
        //     // Arc::clone(&map)
        // );

        // let sending = sender(
        //     Arc::clone(&sock),
        //     Arc::clone(&send_queue),
        //     Arc::clone(&send_queue_state),
        //     Arc::clone(&pending_ids),
        // );

        let active_peers_c = Arc::clone(&active_peers);
        let my_data_c = Arc::clone(&my_data);
        let sock_c = Arc::clone(&sock);
        let receive_queue_c = Arc::clone(&receive_queue);
        let send_queue_c = Arc::clone(&send_queue);
        let action_queue_c = Arc::clone(&action_queue);
        let process_queue_c = Arc::clone(&process_queue);
        let pending_ids_c = Arc::clone(&pending_ids);
        let receive_queue_state_c = Arc::clone(&receive_queue_state);
        let action_queue_state_c = Arc::clone(&action_queue_state);
        let send_queue_state_c = Arc::clone(&send_queue_state);
        let process_queue_state_c = Arc::clone(&process_queue_state);
        let process_queue_readers_state_c = Arc::clone(&process_queue_readers_state);

        tokio::spawn(async move {
            let receiving = receiver(
                Arc::clone(&sock_c),
                Arc::clone(&receive_queue_c),
                Arc::clone(&receive_queue_state_c),
            );

            let handling = handle_packet_task(
                Arc::clone(&pending_ids_c),
                Arc::clone(&receive_queue_c),
                Arc::clone(&receive_queue_state_c),
                Arc::clone(&process_queue_c),
                Arc::clone(&process_queue_state_c),
                Arc::clone(&process_queue_readers_state_c),
            );
            let processing_two = handle_action_task(
                Arc::clone(&send_queue_c),
                Arc::clone(&send_queue_state_c),
                Arc::clone(&action_queue_c),
                Arc::clone(&action_queue_state_c),
                Arc::clone(&process_queue_c),
                Arc::clone(&process_queue_state_c),
            );
            let processing_one = process_task(
                Arc::clone(&action_queue_c),
                Arc::clone(&action_queue_state_c),
                Arc::clone(&process_queue_c),
                Arc::clone(&process_queue_state_c),
                Arc::clone(&active_peers_c),
                Arc::clone(&my_data_c),
                // Arc::clone(&map)
            );

            let sending = sender(
                Arc::clone(&sock_c),
                Arc::clone(&send_queue_c),
                Arc::clone(&send_queue_state_c),
                Arc::clone(&pending_ids_c),
            );

            join!(receiving, handling, processing_one, processing_two, sending);
        });

        // let metrics = Handle::current().metrics();
        // let n = metrics.active_tasks_count();
        // println!("Runtime has {} active tasks", n);
        // let registering = register(
        //     Arc::clone(&process_queue),
        //     Arc::clone(&process_queue_state),
        //     Arc::clone(&process_queue_readers_state),
        //     Arc::clone(&action_queue),
        //     Arc::clone(&action_queue_state),
        //     Arc::clone(&my_data),
        // );

        // let sock_addr: SocketAddr = "81.194.27.155:8443".parse().unwrap();
        let sock_addr: SocketAddr ="86.246.24.173:63801".parse().unwrap();
        {
            Queue::lock_and_push(
                Arc::clone(&action_queue),
                action::Action::SendHello(None, vec![97, 110, 105, 116], *&sock_addr),
            );
        }
        sleep(Duration::from_secs(1)).await;
        // let hash : [u8;32] = {
        //     let peers = active_peers.lock().unwrap();
        //     let peer = peers.addr_map.get(&sock_addr).unwrap();
        //     peer.get_root_hash().unwrap()
        // };
        let hash = [
            211, 20, 115, 228, 84, 20, 231, 30, 31, 144, 12, 151, 66, 10, 253, 48, 29, 89, 243,
            191, 123, 136, 76, 8, 147, 130, 48, 109, 255, 40, 26, 48,
        ];
        let yoan_hash = {
            Queue::lock_and_push(Arc::clone(&action_queue), action::Action::SendRoot(None, sock_addr));
            QueueState::set_non_empty_queue(Arc::clone(&action_queue_state));
            sleep(Duration::from_secs(1)).await;
            let guard = active_peers.lock().unwrap();
            let yoan = guard.addr_map.get(&sock_addr).unwrap();
            yoan.get_root_hash().unwrap()
        };

        fetch_subtree_from(
            Arc::clone(&process_queue),
            Arc::clone(&process_queue_readers_state),
            Arc::clone(&action_queue),
            Arc::clone(&action_queue_state),
            Arc::clone(&maps),
            yoan_hash,
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
}
