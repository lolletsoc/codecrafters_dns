// Uncomment this block to pass the first stage
use std::net::UdpSocket;
use packed_struct::PackingResult;
use packed_struct::prelude::{packed_bits, PackedStruct};
use packed_struct::types::Integer;

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "msb0")]
struct Header {
    #[packed_field(bits = "0..", endian = "msb")]
    id: u16,

    #[packed_field(endian = "msb")]
    qr: Integer<u8, packed_bits::Bits<1>>,

    #[packed_field(endian = "msb")]
    opcode: Integer<u8, packed_bits::Bits<4>>,

    #[packed_field(endian = "msb")]
    aa: Integer<u8, packed_bits::Bits<1>>,

    #[packed_field(endian = "msb")]
    tc: Integer<u8, packed_bits::Bits<1>>,

    #[packed_field(endian = "msb")]
    rd: Integer<u8, packed_bits::Bits<1>>,

    #[packed_field(endian = "msb")]
    ra: Integer<u8, packed_bits::Bits<1>>,

    // Reserved
    #[packed_field(endian = "msb")]
    z: Integer<u8, packed_bits::Bits<3>>,

    #[packed_field(endian = "msb")]
    rcode: Integer<u8, packed_bits::Bits<4>>,

    #[packed_field(endian = "msb")]
    qdcount: u16,

    #[packed_field(endian = "msb")]
    ancount: u16,

    #[packed_field(endian = "msb")]
    nscount: u16,

    #[packed_field(endian = "msb")]
    arcount: u16,
}

fn main() {
    // Uncomment this block to pass the first stage
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");

    // DNS is limited to 512 bytes
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                println!("Received {} bytes from {}", size, source);

                // Header is ALWAYS 12 bytes long
                let header = Header {
                    id: 1234,
                    qr: 1.into(),
                    opcode: 0.into(),
                    aa: 0.into(),
                    tc: 0.into(),
                    rd: 0.into(),
                    ra: 0.into(),
                    z: 0.into(),
                    rcode: 0.into(),
                    qdcount: 0,
                    ancount: 0,
                    nscount: 0,
                    arcount: 0,
                };

                let packed_header = header.pack();
                match packed_header {
                    Ok(bytes) => {
                        println!("Responding with {}", header);
                        udp_socket
                            .send_to(&bytes, source)
                            .expect("Failed to send response");
                    }
                    Err(err) => {
                        eprintln!("Error receiving data: {}", err);
                        break;
                    }
                }
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
