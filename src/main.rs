// Uncomment this block to pass the first stage
use std::net::UdpSocket;
use nom::Slice;
use packed_struct::prelude::{packed_bits, PackedStruct};
use packed_struct::types::Integer;

#[derive(PackedStruct, Debug)]
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

struct Question {
    name: String,
    _type: u16,
    class: u16,
}

#[derive(PackedStruct, Debug)]
#[packed_struct(bit_numbering = "msb0")]
struct Message {
    #[packed_field(bytes = "0..13", endian = "msb")]
    header: Header,

    #[packed_field(endian = "msb")]
    question: [u8; 500],
}

fn construct_questions_from(bytes: &[u8]) -> String {
    let mut lookup_parts = vec![];
    let mut i: u8 = 0;
    loop {
        let length: u8 = bytes[i as usize];
        if length == b'\0' {
            break;
        }

        lookup_parts.push(String::from_utf8_lossy(&bytes[(i + 1) as usize..(length + 1) as usize]).to_string());

        i += length;
    }

    lookup_parts.join(".")
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

                match Message::unpack(&buf) {
                    Ok(mut message) => {
                        message.header.qr = 1.into();
                        let lookup = construct_questions_from(&message.question);
                        println!("Received request for {}", lookup);

                        udp_socket
                            .send_to(&message.pack().unwrap(), source)
                            .expect("Failed to send response");
                    }
                    Err(_) => {
                        continue;
                    }
                };
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
