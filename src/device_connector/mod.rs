use std::{collections::HashMap, sync::{Arc, Mutex}};

use packets::{ApplicationPacket, InitiateConnectionPacket, PacketReadError, RegisterDevicePacket, UnregisterDevicePacket};
use rocket::{fairing::{Fairing, Info, Kind}, tokio::{net::{TcpListener, TcpStream, UdpSocket}, select, spawn, task::JoinHandle}, Build, Rocket};
use tokio_util::sync::CancellationToken;

use crate::MainDatabase;

pub mod packets;

type SessionList = Arc<Mutex<HashMap<[u8; 16], Session>>>;

pub struct Session;

pub struct DeviceBridge {
	tcp_listening_task: Option<JoinHandle<()>>,
	udp_socket_task: Option<JoinHandle<()>>,
	port: u16,
	sessions: SessionList,
	canceller: CancellationToken,
	database: Arc<MainDatabase>,
}

impl DeviceBridge {
	pub fn new(port: u16, database: MainDatabase) -> Self {
		let sessions = Arc::new(Mutex::new(HashMap::new()));

		let mut result = Self {
			tcp_listening_task: None,
			udp_socket_task: None,
			port,
			sessions,
			canceller: CancellationToken::new(),
			database: Arc::new(database),
		};
		result.init();

		return result;
	}

	pub fn init(&mut self) {
		let tcp_socket = std::net::TcpListener::bind(("0.0.0.0", self.port)).unwrap();
		tcp_socket.set_nonblocking(true).unwrap();
		let tcp_socket = TcpListener::from_std(tcp_socket).unwrap();

		let udp_listener = std::net::UdpSocket::bind(("0.0.0.0", self.port)).unwrap();
		udp_listener.set_nonblocking(true).unwrap();
		let udp_listener = UdpSocket::from_std(udp_listener).unwrap();

		let session_clone = self.sessions.clone();
		let canceller = self.canceller.clone();
		let db_clone = self.database.clone();
		let tcp_listening_task = spawn(async move {
			loop {
				select! {
					_ = canceller.cancelled() => {
						return;
					}
					connection = tcp_socket.accept() => {
						let (stream, _) = connection.unwrap();
						let db_clone = db_clone.clone();
						spawn(handle_connection(stream, session_clone.clone(), canceller.clone(), db_clone));
					}
				}
			}
		});

		let udp_socket_task = spawn(async move {
			let mut buffer = Vec::new();
			let packet_addr = udp_listener.recv_from(&mut buffer).await.unwrap().1;
			println!("packet from {}", packet_addr);
		});

		self.tcp_listening_task = Some(tcp_listening_task);
		self.udp_socket_task = Some(udp_socket_task);
	}

	pub fn fairing(port: u16) -> DeviceBridgeFairing {
		return DeviceBridgeFairing { port };
	}
}
	
async fn handle_connection(mut socket: TcpStream, sessions: SessionList, canceller: CancellationToken, database: Arc<MainDatabase>) {
	loop {
		select! {
			_ = canceller.cancelled() => {
				return;
			}
			read_result = packets::read_packet_async(&mut socket) => {
				match read_result {
					Ok(packet) => {
						handle_packet(packet, &mut socket, sessions.clone(), &database).await;
					},
					Err(PacketReadError::CantRead) => {
						log::warn!("Couldn't finish reading packet, TCP stream likely to have ended from the other end. Ending handler.");
						return;
					},
					Err(PacketReadError::HeaderParseError(data) | PacketReadError::PacketParseError(data)) => {
						log::warn!("Packet parsing failure: {}. Likely to require killing the stream anyway.", data);
					}
				}
			}
		}
	}
}

async fn handle_packet(packet: ApplicationPacket, socket: &mut TcpStream, sessions: SessionList, database: &MainDatabase) {
	log::debug!("Got packet: {:?}", packet);
	match packet.message {
		packets::Message::RegisterDevice(RegisterDevicePacket { auth_key, camera_id, mac_address, user_id}) => {
			//
		}
		packets::Message::InitiateConnection(InitiateConnectionPacket { auth_key, camera_id }) => {
			//
		}
		packets::Message::NoOperation(_) => {
			//
		}
		packets::Message::UnregisterDevice(UnregisterDevicePacket { success }) => {
			//
		}
	}
}

pub struct DeviceBridgeFairing {
	port: u16
}

#[rocket::async_trait]
impl Fairing for DeviceBridgeFairing {
	fn info(&self) -> rocket::fairing::Info {
		return Info {
			name: "Device bridge initialization",
			kind: Kind::Ignite
		};
	}

	async fn on_ignite(&self, rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
		let db = MainDatabase::get_one(&rocket).await.unwrap();
		return Ok(rocket.manage(DeviceBridge::new(self.port, db)));
	}
}