
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::{Instant, Duration};
use mio::{Events, Ready, Poll, PollOpt, Token};
use mio::net::UdpSocket;
use bincode::deserialize;
use siege_net::Remote;
use siege_example_net::*;
use config::Config;
use state::State;
use errors::*;

pub mod packet_sender;
pub use self::packet_sender::PacketSender;

const INBOUND_READY: Token = Token(0);
const OUTBOUND_READY: Token = Token(1);

pub enum Continue {
    KeepGoing,
    Terminate
}

pub struct NetworkSystem {
    #[allow(dead_code)]
    config: Arc<Config>,
    state: Arc<State>,
    server_public_key: [u8; 32],
    remote: Arc<Mutex<Remote>>,
    socket: UdpSocket,
}

impl NetworkSystem {
    pub fn new(state: Arc<State>, config: Arc<Config>)
               -> Result<NetworkSystem>
    {
        // Load the server's public key
        let server_public_key = config.network.server_public_key.clone();

        let remote = Arc::new(
            Mutex::new(
                Remote::new(
                    config.network.server_socket_addr,
                    state.rng.clone()
                )?
            )
        );

        // Bind a UDP socket
        let unspecified_address: SocketAddr = FromStr::from_str("0.0.0.0:0")?;
        let socket = UdpSocket::bind(&unspecified_address)?;
        info!("Local socket bound at {}", socket.local_addr()?);

        if config.network.connect_on_startup {
            // Connect to server (that is, specify who the remote is for all subsequent
            // operations. As the client, we only have one remote).
            socket.connect(config.network.server_socket_addr)?;

            // Inject the Init packet
            {
                let mut remote = remote.lock().unwrap();
                state.packet_sender.send(GamePacket::Init(InitPacket::new(&mut remote)?))?;
            }
        }

        let ns = NetworkSystem {
            config: config,
            state: state,
            server_public_key: server_public_key,
            remote: remote,
            socket: socket,
        };

        Ok(ns)
    }


    pub fn run(&mut self) -> Result<()>
    {
        // Setup the mio system for pollling
        let poll = Poll::new()?;
        poll.register(&self.socket, INBOUND_READY, Ready::readable(), PollOpt::edge())?;
        poll.register(&self.state.packet_sender, OUTBOUND_READY, Ready::readable(), PollOpt::edge())?;

        let mut events = Events::with_capacity(128);
        let mut buffer: [u8; 2000] = [0; 2000];
        loop {
            poll.poll(&mut events, None)?;
            for event in events.iter() {
                match event.token() {
                    INBOUND_READY => loop {
                        let len = match self.socket.recv(&mut buffer) {
                            Err(e) => {
                                if e.kind() == ::std::io::ErrorKind::WouldBlock {
                                    break; // we have handled all packets
                                }
                                return Err(From::from(e));
                            }
                            Ok(len) => len,
                        };

                        match self.handle_incoming_packet(&mut buffer[..len]) {
                            Ok(Continue::KeepGoing) => (),
                            Ok(Continue::Terminate) => {
                                // Let other threads know that we are shutting down
                                self.state.terminating.store(true, Ordering::Relaxed);
                                return Ok(());
                            },
                            Err(e) => {
                                error!("{}", e);
                                // Let other threads know that we are shutting down
                                self.state.terminating.store(true, Ordering::Relaxed);
                                return Err(From::from(e));
                            }
                        }
                    },
                    OUTBOUND_READY => loop {
                        if let Some(packet) = self.state.packet_sender.outbound.try_pop() {
                            match self.handle_outgoing_packet(packet) {
                                Ok(Continue::KeepGoing) => (),
                                Ok(Continue::Terminate) => {
                                    // Let other threads know that we are shutting down
                                    self.state.terminating.store(true, Ordering::Relaxed);
                                    return Ok(());
                                },
                                Err(e) => {
                                    error!("{}", e);
                                    // Let other threads know that we are shutting down
                                    self.state.terminating.store(true, Ordering::Relaxed);
                                    return Err(From::from(e));
                                }
                            }
                        } else {
                            break; // we have handled all outbound messages
                        }
                    },
                    _ => unreachable!()
                }
            }
        }
    }

    // note: only return Err on terminating conditions.
    fn handle_outgoing_packet(&self, packet: GamePacket) -> Result<Continue>
    {
        // Build the packet
        let packet_bytes = {
            let mut remote = self.remote.lock().unwrap();
            remote.serialize_packet(&packet, MAGIC, VERSION)?
        };

        // Send the packet
        trace!("Sending {:?}", &packet);
        if let Err(e) = self.socket.send(&packet_bytes) {
            error!("Error sending {} packet: {:?}", packet.name(), e);
        }

        // If the packet was 'shutdown', lets exit
        if let GamePacket::Shutdown(_) = packet {
            trace!("Shutting down (not sending shutdown packet yet).");
            return Ok(Continue::Terminate);
        }


        Ok(Continue::KeepGoing)
    }

    // note: only return Err on terminating conditions.
    fn handle_incoming_packet(&self, bytes: &mut [u8]) -> Result<Continue>
    {
        // We check magic here to discard wayward packets early.
        //
        // We shouldn't fail on a bad version right away, because it could come
        // from an attacker trying to knock us off the network.  Only after we
        // authenticate the packet should we act on it.
        let _version_is_ok = ::siege_net::packets::validate_magic_and_version(
            MAGIC, VERSION, bytes)?;

        let (body_bytes, seq, _stale) = {
            let mut remote = self.remote.lock().unwrap();
            // FIXME: don't fail here.  A ring error could be an attacker who should
            // just be ignored.
            match remote.deserialize_packet_header::<GamePacket>(&mut bytes[..]) {
                Err(e) => {
                    error!("{}", e);
                    return Ok(Continue::KeepGoing);
                },
                Ok(stuff) => stuff
            }
        };

        let packet: GamePacket = deserialize(body_bytes)?;

        // Update stats for clocksync
        {
            let mut stats = self.state.stats.write().unwrap();
            let remote = self.remote.lock().unwrap();
            if let Some(max) = remote.offset_max {
                if let Some(min) = remote.offset_min {
                    stats.network_clocksync_ms = max - min;
                }
            }
        }

        // Print the packet
        trace!("PACKET RECEIVED: [{}] {:?}",
               seq, packet);

        // Handle the packet
        match packet {
            GamePacket::UpgradeRequired(ur) => {
                error!("Upgrade Required to version {}", ur.version);
                return Ok(Continue::Terminate);
            }
            GamePacket::InitAck(initack) => self.handle_init_ack(initack),
            GamePacket::Heartbeat(hb) => self.handle_heartbeat(hb, seq),
            GamePacket::HeartbeatAck(_) => Ok(Continue::KeepGoing), // siege-net does this one
/*            GamePacket::OrbitSim(ospkt) => self.handle_orbitsim(ospkt), */
            _ => {
                let error = ::siege_net::Error::from_kind(
                    ::siege_net::ErrorKind::InvalidPacket);
                error!("{}", error);
                Ok(Continue::KeepGoing)
            }
        }
    }

    fn handle_init_ack(&self, init_ack: InitAckPacket) -> Result<Continue>
    {
        {
            let mut remote = self.remote.lock().unwrap();

            // Verify the nonce response
            remote.validate_nonce_signature(
                &init_ack.get_nonce_response(),
                &self.server_public_key)?;

            // Compute session key
            remote.compute_session_key(&init_ack.public_key)?;
            trace!("Session key is {:?}", remote.session_key);
        }

        // Send back a heartbeat (this starts the heartbeat chain)
        self.state.packet_sender.send(GamePacket::Heartbeat(HeartbeatPacket::new()))?;

        Ok(Continue::KeepGoing)
    }

    fn handle_heartbeat(&self, _heartbeat: HeartbeatPacket, _seq: u32) -> Result<Continue>
    {
        // Send back a HeartbeatAck immediately
        self.state.packet_sender.send(GamePacket::HeartbeatAck(HeartbeatAckPacket::new()))?;

        // Also send back a heartbeat after 10 seconds (to continue the heartbeat chain)
        self.state.packet_sender.send_at_future_time(
            GamePacket::Heartbeat(HeartbeatPacket::new()),
            Instant::now() + Duration::new(10,0)
        )?;

        // Write a chat message (to help test chat)
        {
            let mut chat = self.state.chat.write().unwrap();
            chat.emit_line(&self.state.ui, "thump...");
        }

        Ok(Continue::KeepGoing)
    }

/*    fn handle_orbitsim(&self, ospkt: OrbitSimPacket) -> Result<Continue>
    {
        let mut stars = self.state.stars.write().unwrap();
        stars.sync(&ospkt.checkpoint_data);
        info!("Synchronized orbit sim");
        Ok(Continue::KeepGoing)
    }
*/
}
