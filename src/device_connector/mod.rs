use std::{collections::HashMap, sync::{Arc, Mutex}};

use diesel::{insert_into, result::{DatabaseErrorKind, Error}, QueryDsl, RunQueryDsl};
use packets::{ApplicationPacket, InitiateConnectionPacket, PacketReadError, RegisterDevicePacket, UnregisterDevicePacket};
use rand::Rng;
use rocket::{fairing::{Fairing, Info, Kind}, tokio::{net::{TcpListener, TcpStream, UdpSocket}, select, spawn, task::JoinHandle}, Build, Rocket};
use tokio_util::sync::CancellationToken;

use crate::{model::{Device, User}, MainDatabase};
use crate::schema::device::dsl as device_dsl;
use crate::schema::users::dsl as users_dsl;

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
						match handle_packet(packet, &mut socket, sessions.clone(), &database).await {
							Ok(_) => {
								log::debug!("Finished packet handler");
							}
							Err(PacketHandlerError::Ending) => {
								log::warn!("Connection loop received ending error. Exiting the loop.");
								return;
							}
							Err(PacketHandlerError::NonEnding) => {
								log::warn!("Connection loop received nonending error.");
							}
						};
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

/// Makes a decision to either kill the socket or not
enum PacketHandlerError {
	Ending,
	NonEnding,
}

async fn handle_packet(packet: ApplicationPacket, socket: &mut TcpStream, sessions: SessionList, database: &MainDatabase) -> Result<(), PacketHandlerError> {
	log::debug!("Got packet: {:?}", packet);
	match packet.message {
		packets::Message::RegisterDevice(data) => {
			match handle_registration(socket, database, data).await {
				Ok(_) => {
					log::debug!("Finished registration handler");
					return Ok(());
				}
				Err(_) => {
					log::debug!("Bubbling ending registration error");
					return Err(PacketHandlerError::Ending);
				}
			}
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
	return Ok(());
}

enum DeviceRegisterError {
	UserDoesNotExist,
	DatabaseError(Error),
	OtherError,
}

async fn handle_registration(socket: &mut TcpStream, database: &MainDatabase, register_packet: RegisterDevicePacket) -> Result<(), DeviceRegisterError> {
	let RegisterDevicePacket { auth_key, camera_id, mac_address, user_id} = register_packet;

	// TODO: Device ID needs to be generated in first stage, not checked. 
	if camera_id == [0; 16] {
		log::info!("Device sent empty camera ID, registering.");
		let user_query = users_dsl::users.find(user_id);
		let user = database.run(move |conn| user_query.first::<User>(conn)).await;
		match user {
			Ok(_) => {
				log::debug!("Found user for new device registration: {:?}", user_id);
				let mut new_device = Device {
					auth_key: Vec::from(auth_key), 
					device_id: Vec::from(camera_id),
					mac_address: Vec::from(mac_address),
					registration_first_stage: true,
					user_id: Vec::from(user_id),
				};
				rand::thread_rng().fill(new_device.device_id.as_mut_slice());
				loop {
					let insert_query = insert_into(device_dsl::device).values(new_device.clone());
					match database.run(|conn| insert_query.execute(conn)).await {
						Ok(_) => {
							log::info!("First stage registered new device");
							return Ok(());
						}
						Err(Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _)) => {
							log::debug!("Device ID {:?} exists in database, regenerating ID", new_device.device_id);
							rand::thread_rng().fill(new_device.device_id.as_mut_slice());
						}
						Err(err) => {
							log::error!("Error while registering device: {:?}", err);
							return Err(DeviceRegisterError::DatabaseError(err));
						}
					}
				}
			}
			Err(Error::NotFound) => {
				log::warn!("Device is trying to register with an unknown user id");
				return Err(DeviceRegisterError::UserDoesNotExist);
			}
			Err(err) => {
				log::warn!("Error while retrieving user in register handler: {:?}", err);
				return Err(DeviceRegisterError::DatabaseError(err));
			}
		}
	} else {
		let device_query = device_dsl::device.find(camera_id);
		let dev = database.run(move |conn| device_query.first::<Device>(conn)).await;
		match dev {
			Ok(device) => {
				log::info!("Device {:?} is attempting to re-register (for now)", camera_id);
				return Err(DeviceRegisterError::OtherError);
			}
			Err(Error::NotFound) => {
				log::info!("Registering new device: {:?}", camera_id);
				return Ok(());
			}
			Err(err) => {
				log::warn!("Error while retrieving device in register handler: {:?}", err);
				return Err(DeviceRegisterError::DatabaseError(err));
			}
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