/*! Library for managing Wake-on-LAN packets.
# Example
```
let wol = wakey::WolPacket::from_string("01:02:03:04:05:06", ':')?;
let result = wol.send_magic();
match result {
	Ok(_) => println!("Sent the magic packet!"),
	Err(_) => println!("Failed to send the magic packet!"),
};
# Ok::<(), wakey::Error>(())

```
*/

use std::iter;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};

const MAC_SIZE: usize = 6;
const MAC_PER_MAGIC: usize = 16;
const HEADER: [u8; 6] = [0xFF; 6];

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
	Hex(hex::FromHexError),
	IO(std::io::Error),
	InvalidHexStringLength,
	InvalidHexArrayLength,
}

impl std::convert::From<std::io::Error> for Error {
	fn from(error: std::io::Error) -> Self {
		Error::IO(error)
	}
}
/// Wake-on-LAN packet
#[derive(Debug)]
pub struct WolPacket {
	/// WOL packet bytes
	packet: Vec<u8>,
}

impl WolPacket {
	/// Creates WOL packet from byte MAC representation
	/// # Example
	/// ```
	/// let wol = wakey::WolPacket::from_bytes(&vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
	/// ```
	pub fn from_bytes(mac: &[u8]) -> Result<WolPacket> {
		match mac.len() {
			MAC_SIZE => Ok(WolPacket {
				packet: WolPacket::create_packet_bytes(mac),
			}),
			_ => Err(Error::InvalidHexArrayLength),
		}

	}

	/// Creates WOL packet from string MAC representation (e.x. 00:01:02:03:04:05)
	/// # Example
	/// ```
	/// let wol = wakey::WolPacket::from_string("00:01:02:03:04:05", ':');
	/// ```
	/// # Panic
	///  Panics when input MAC is invalid (i.e. contains non-byte characters)
	pub fn from_string(data: &str, sep: char) -> Result<WolPacket> {
		let bytes = WolPacket::mac_to_byte(data, sep)?;
		WolPacket::from_bytes(&bytes)
	}

	/// Broadcasts the magic packet from / to default address
	/// Source: 0.0.0.0:0
	/// Destination 255.255.255.255:9
	/// # Example
	/// ```
	/// let wol = wakey::WolPacket::from_bytes(&vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05])?;
	/// wol.send_magic()?;
	/// # Ok::<(), wakey::Error>(())
	/// ```
	pub fn send_magic(&self) -> Result<usize> {
		self.send_magic_to(
			SocketAddr::from(([0, 0, 0, 0], 0)),
			SocketAddr::from(([255, 255, 255, 255], 9)),
		)
	}

	/// Broadcasts the magic packet from / to specified address.
	/// # Example
	/// ```
	/// use std::net::SocketAddr;
	/// let wol = wakey::WolPacket::from_bytes(&vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05])?;
	/// let src = SocketAddr::from(([0,0,0,0], 0));
	/// let dst = SocketAddr::from(([255,255,255,255], 9));
	/// wol.send_magic_to(src, dst);
	/// # Ok::<(), wakey::Error>(())
	/// ```
	pub fn send_magic_to<A: ToSocketAddrs>(&self, src: A, dst: A) -> Result<usize> {
		let udp_sock = UdpSocket::bind(src)?;
		udp_sock.set_broadcast(true)?;
		let bytes_sent = udp_sock.send_to(&self.packet, dst)?;

		Ok(bytes_sent)
	}

	/// Converts string representation of MAC address (e.x. 00:01:02:03:04:05) to raw bytes.
	fn mac_to_byte(data: &str, sep: char) -> Result<Vec<u8>> {
		let str_out = &data
			.split(sep)
			.map(|v| v.bytes())
			.flatten()
			.collect::<Vec<u8>>();

		let hex_out = hex::decode(str_out).map_err(Error::Hex)?;

		match hex_out.len() {
			MAC_SIZE => Ok(hex_out),
			_ => Err(Error::InvalidHexStringLength),
		}
	}

	/// Extends the MAC address to fill the magic packet
	fn extend_mac(mac: &[u8]) -> Vec<u8> {
		iter::repeat(mac).take(MAC_PER_MAGIC).flatten().cloned().collect()
	}

	/// Creates bytes of the magic packet from MAC address
	/// TODO: Cleanup to use refs
	fn create_packet_bytes(mac: &[u8]) -> Vec<u8> {
		let mut packet = Vec::with_capacity(HEADER.len() + MAC_SIZE * MAC_PER_MAGIC);

		packet.extend(HEADER.iter());
		packet.extend(WolPacket::extend_mac(mac));

		packet
	}
}

#[cfg(test)]
mod tests {
	#[test]
	fn extend_mac_test() {
		let mac = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];

		let extended_mac = super::WolPacket::extend_mac(&mac);

		assert_eq!(extended_mac.len(), super::MAC_PER_MAGIC * super::MAC_SIZE);
		for i in 0..super::MAC_PER_MAGIC {
			let start = super::MAC_SIZE * i;
			let end = super::MAC_SIZE * ( i + 1 );

			assert_eq!(&mac[..], &extended_mac[start..end]);
		}
		//assert!(&extended_mac.iter().zip(&mac).all(|(i, v)| *i == v));
	}

	#[test]
	fn mac_to_byte_test() {
		let mac = "01:02:03:04:05:06";
		let result = super::WolPacket::mac_to_byte(mac, ':');

		assert!(result
			.unwrap()
			.eq(&vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06]));
	}

	#[test]
	fn mac_to_byte_invalid_chars_test() {
		let mac = "ZZ:02:03:04:05:06";

		use hex;
		// TODO: FIX THIS
		if let Err(super::Error::Hex(hex::FromHexError::InvalidHexCharacter { c: 'Z', index: 0 })) =
			super::WolPacket::mac_to_byte(mac, ':')
		{
		} else {
			dbg!(super::WolPacket::mac_to_byte(mac, ':').unwrap_err());
			assert!(false);
		}
		// assert_eq!(super::WolPacket::mac_to_byte(mac, ':'), Err(super::Error::Hex(hex::FromHexError::InvalidHexCharacter('Z', 0))));
	}

	#[test]
	fn mac_to_byte_invalid_separator_test() {
		let mac = "01002:03:04:05:06";
		assert!(super::WolPacket::mac_to_byte(mac, ':').is_err());
	}

	#[test]
	fn create_packet_bytes_test() {
		let bytes = super::WolPacket::create_packet_bytes(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);

		assert_eq!(
			bytes.len(),
			super::MAC_SIZE * super::MAC_PER_MAGIC + super::HEADER.len()
		);
		assert!(bytes.iter().all(|&x| x == 0xFF));
	}

	#[test]
	fn send_test_packet() -> super::Result<()> {
		let wol = super::WolPacket::from_string("DE AD BE EF CA FE", ' ')?;
		let s = std::net::UdpSocket::bind("0.0.0.0:9").expect("Could not listen on port ::9");
		let mut buf = [0; 102];

		assert_eq!(wol.send_magic().ok(), Some(102));

		let (_amt, _src) = s.recv_from(&mut buf).expect("Could not read socket");
		for i in 0..buf.len() {
			print!("{}, ", buf[i]);
		}
		println!();
		dbg!(_amt);
		dbg!(_src);
		assert_eq!(&buf[..], &wol.packet[..]);
		Ok(())
	}
}
