use std::borrow::Cow;

use deku::prelude::*;
use rocket::tokio::{io::AsyncReadExt, net::TcpStream};

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct ApplicationPacket {
	pub header: PacketHeader,
	pub message: Message,
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct PacketHeader {
	pub session_id: [u8; 16],
	#[deku(endian = "little")]
	pub buffer_size: u32,
	#[deku(bits = "1")]
	pub is_response: bool,
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(id_type = "u8", bits = 7)]
#[non_exhaustive]
pub enum Message {
	#[deku(id = "0x00")]
	NoOperation(EmptyPacket),
	#[deku(id = "0x01")]
	RegisterDevice(RegisterDevicePacket),
	#[deku(id = "0x02")]
	UnregisterDevice(UnregisterDevicePacket),
	#[deku(id = "0x03")]
	InitiateConnection(InitiateConnectionPacket),
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct EmptyPacket {}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct RegisterDevicePacket {
	pub user_id: [u8; 16],
	pub camera_id: [u8; 16],
	pub auth_key: [u8; 16],
	pub mac_address: [u8; 6],
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct InitiateConnectionPacket {
	pub camera_id: [u8; 16],
	pub auth_key: [u8; 16],
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct UnregisterDevicePacket {
	pub success: u8,
}

#[derive(Debug, Clone, Copy, DekuRead, DekuWrite, PartialEq, Eq)]
#[deku(id_type = "u8")]
pub enum ImageChunkType {
	MiddleChunk = 0x00,
	FirstChunk = 0x01,
	LastChunk = 0x02,
	OnlyChunk = 0x03,
}

#[derive(Debug, Clone, DekuRead, DekuWrite, PartialEq, Eq)]
pub struct ImageChunk {
	pub chunk_id: u32,
	pub chunk_type: ImageChunkType,
	pub session_id: [u8; 16],
	#[deku(read_all)]
	pub image_bytes: Vec<u8>,
}

/// Enum representing possible errors that can happen when reading a packet from a socket
#[derive(Debug, Clone)]
pub enum PacketReadError {
	/// Can't fully read a packet from the socket, probably because it was closed on the other end
	CantRead,
	/// Failed to parse header. Contains information about why it failed
	HeaderParseError(Cow<'static, str>),
	/// Failed to parse full packet. Contains information about why it failed
	PacketParseError(Cow<'static, str>),
}

// Incredibly convoluted reading function since deku does not support async readers
pub async fn read_packet_async(socket: &mut TcpStream) -> Result<ApplicationPacket, PacketReadError> {
	let mut header_buffer = [0; 21];
	match socket.read_exact(&mut header_buffer).await {
		Ok(read_size) => {
			log::debug!("Read {} bytes from socket", read_size);
		}
		Err(err) => {
			match err.kind() {
				std::io::ErrorKind::UnexpectedEof => {
					return Err(PacketReadError::CantRead);
				}
				_ => {
					panic!("Unhandled error while reading header: {:#?}", err);
				}
			}
		}
	}
	let header;
	match PacketHeader::from_bytes((&header_buffer, 0)) {
		Ok((_, head)) => {
			header = head;
		}
		Err(DekuError::Parse(err_string)) => {
			return Err(PacketReadError::HeaderParseError(err_string));
		}
		Err(other_err) => {
			panic!("Unhandled error while parsing header: {:#?}", other_err);
		}
	}
	assert!(header.buffer_size < 100, "Packet buffer size ({}) is suspiciously big", header.buffer_size);
	let mut packet_buffer = Vec::with_capacity(21 + header.buffer_size as usize);
	unsafe {
		// SAFETY: All possible bit values are valid for `usize`.
		// Required to fill it with `TcpStream::read_exact`
		packet_buffer.set_len(21 + header.buffer_size as usize);
	}
	match socket.read_exact(&mut packet_buffer[21..]).await {
		Ok(read_size) => {
			log::debug!("Read {} bytes from socket", read_size);
		}
		Err(err) => {
			match err.kind() {
				std::io::ErrorKind::UnexpectedEof => {
					return Err(PacketReadError::CantRead);
				}
				_ => {
					panic!("Unhandled error while reading rest of message: {:#?}", err);
				}
			}
		}
	}
	packet_buffer[0..21].copy_from_slice(&header_buffer);

	let packet;
	match ApplicationPacket::from_bytes((&packet_buffer, 0)) {
		Ok((_, pck)) => {
			packet = pck;
		}
		Err(DekuError::Parse(err_string)) => {
			return Err(PacketReadError::PacketParseError(err_string));
		}
		Err(other_err) => {
			panic!("Unhandled error while parsing header: {:#?}", other_err);
		}
	}
	
	return Ok(packet);
}

#[cfg(test)]
mod test {
    use std::{assert_matches::assert_matches, mem::size_of};

    use deku::DekuContainerWrite;

    use crate::device_connector::packets::{ApplicationPacket, Message, PacketHeader};

    use super::{ImageChunk, ImageChunkType, InitiateConnectionPacket};

	#[test]
	fn decode_noop_request() {
		let data: [u8; _] = [
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			0, 0, 0, 0,
			0b0_0000000,
		];

		let decoded = ApplicationPacket::try_from(data.as_ref()).unwrap();
		assert_matches!(decoded.message, Message::NoOperation(_));
		assert!(!decoded.header.is_response);
	}

	#[test]
	fn decode_registration_response() {
		let data: [u8; _] = [
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
			54, 0, 0, 0,
			0b1_0000001,
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
			15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
			16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30, 31, 32,
			0, 1, 2, 3, 4, 5
		];

		let decoded = ApplicationPacket::try_from(data.as_ref()).unwrap();
		assert!(decoded.header.is_response);
		assert_eq!(decoded.header.buffer_size, 54);
		assert_eq!(decoded.header.session_id, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
		assert_matches!(decoded.message, Message::RegisterDevice(_));
		if let Message::RegisterDevice(inner) = decoded.message {
			assert_eq!(inner.user_id, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
			assert_eq!(inner.camera_id, [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);
			assert_eq!(inner.auth_key, [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30, 31, 32]);
			assert_eq!(inner.mac_address, [0, 1, 2, 3, 4, 5]);
		}
	}

	#[test]
	fn encode_initcomm_response() {
		let data = ApplicationPacket {
			header: PacketHeader {
				session_id: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
				is_response: true,
				buffer_size: size_of::<InitiateConnectionPacket>() as u32,
			},
			message: Message::InitiateConnection(InitiateConnectionPacket {
				auth_key: [16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30, 31, 32],
				camera_id: [15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]
			})
		};

		let encoded = data.to_bytes().unwrap();
		assert_eq!(encoded, vec![
			0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
			32, 0, 0, 0,
			0b1_0000011, // 0x03 + response
			15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
			16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 29, 30, 31, 32
		]);
	}

	#[test]
	fn image_chunk() {
		let data = ImageChunk {
			chunk_id: 5, 
			chunk_type: ImageChunkType::MiddleChunk,
			session_id: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
			image_bytes: vec![0; 16384],
		};

		let encoded = data.to_bytes().unwrap();
		assert_eq!(encoded.len(), 4 + 1 + 16 + 16384);

		let decoded = ImageChunk::try_from(encoded.as_ref()).unwrap();
		assert_eq!(decoded.chunk_id, 5);
		assert_eq!(decoded.chunk_type, ImageChunkType::MiddleChunk);
		assert_eq!(decoded.session_id, [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15]);
		assert_eq!(decoded.image_bytes.len(), 16384);
	}
}