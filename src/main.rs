use std::net::UdpSocket;
use deku::prelude::*;
use nom::AsBytes;

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Header {
    #[deku(bits = 16)]
    id: u16,

    #[deku(bits = 1)]
    qr: u8,

    #[deku(bits = 4)]
    opcode: u8,

    #[deku(bits = 1)]
    aa: u8,

    #[deku(bits = 1)]
    tc: u8,

    #[deku(bits = 1)]
    rd: u8,

    #[deku(bits = 1)]
    ra: u8,

    // Reserved
    #[deku(bits = 3)]
    z: u8,

    #[deku(bits = 4)]
    rcode: u8,

    #[deku(bits = 16)]
    qdcount: u16,

    #[deku(bits = 16)]
    ancount: u16,

    #[deku(bits = 16)]
    nscount: u16,

    #[deku(bits = 16)]
    arcount: u16,

}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Question {
    #[deku(until = "|v: &u8| *v == b'\0'")]
    name: Vec<u8>,

    #[deku(bits = 16)]
    _type: u16,

    #[deku(bits = 16)]
    class: u16,
}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
struct Answer {
    #[deku(until = "|v: &u8| *v == b'\0'")]
    name: Vec<u8>,

    #[deku(bits = 16)]
    _type: u16,

    #[deku(bits = 16)]
    class: u16,

    #[deku(bits = 32)]
    ttl: u32,

    #[deku(update = "self.data.len()")]
    length: u16,

    #[deku(count = "length")]
    data: Vec<u8>,

}

#[derive(Debug, PartialEq, DekuRead, DekuWrite)]
struct Message {
    header: Header,

    question: Question,

    answer: Answer,
}

fn construct_lookup(bytes: &[u8]) -> (String, u8) {
    let mut lookup_parts = vec![];
    let mut i: u8 = 0;
    loop {
        let length: u8 = bytes[i as usize];
        if length == b'\0' {
            break;
        }

        lookup_parts.push(String::from_utf8_lossy(&bytes[(i + 1) as usize..(i + length + 1) as usize]).to_string());

        i += length + 1;
    }

    (lookup_parts.join("."), i)
}

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");

    // DNS is limited to 512 bytes
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((size, source)) => {
                match Message::from_bytes((&buf, 0)) {
                    Ok(result) => {
                        let mut message = result.1;
                        let answer = Answer {
                            name: message.question.name.to_owned(),
                            _type: 1,
                            class: 1,
                            ttl: 60,
                            length: 4,
                            data: vec!(0x08, 0x08, 0x08, 0x08),
                        };

                        message.header.qr = 1;
                        message.header.ancount = 1;
                        message.header.rcode = if message.header.opcode == 0 { 0 } else { 4 };
                        message.answer = answer;
                        let lookup = construct_lookup(message.question.name.as_bytes());
                        println!("Looking up '{}'", lookup.0);

                        udp_socket
                            .send_to(message.to_bytes().unwrap().as_bytes(), source)
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
