use deku::prelude::*;
use nom::AsBytes;
use std::net::UdpSocket;
use std::ptr::read;

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

#[derive(Debug, PartialEq, DekuWrite)]
struct Question {
    #[deku(writer = "Question::write(deku::writer, &self.name)")]
    name: Vec<u8>,

    #[deku(bits = 16)]
    _type: u16,

    #[deku(bits = 16)]
    class: u16,
}

#[derive(Debug, PartialEq, DekuWrite)]
#[deku(endian = "big")]
struct Answer {
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

    #[deku(reader = "Message::read_questions(deku::reader, (*header).qdcount)")]
    question: Vec<Question>,

    #[deku(
        reader = "Message::default_answers()",
        writer = "Message::write_answers(deku::writer, &question)"
    )]
    answer: Vec<Answer>,
}


impl Question {
    fn write<W: std::io::Write>(writer: &mut Writer<W>, bytes: &Vec<u8>) -> Result<(), DekuError> {
        let encoded = encode_lookup_to_dns(bytes);
        encoded.to_writer(writer, ())
    }
}

impl Message {
    fn read_questions<R: std::io::Read>(reader: &mut Reader<R>, qdcount: u16) -> Result<Vec<Question>, DekuError> {
        let questions: Vec<Question>;

        let bits_read = reader.bits_read;

        let unwrapped = &reader.read_bits(500).unwrap().unwrap();
        let bytes_slice = unwrapped.as_raw_slice();
        questions = construct_lookups(bytes_slice, bits_read / 8, qdcount);
        Ok(questions)
    }

    fn default_answers() -> Result<Vec<Answer>, DekuError> {
        Ok(Vec::new())
    }

    fn write_answers<W: std::io::Write>(writer: &mut Writer<W>, questions: &Vec<Question>) -> Result<(), DekuError> {
        let mut answers = Vec::with_capacity(questions.len());

        for q in questions {
            let answer = Answer {
                name: encode_lookup_to_dns(&q.name),
                _type: 1,
                class: 1,
                ttl: 60,
                length: 4,
                data: vec!(0x08, 0x08, 0x08, 0x08),
            };
            answers.push(answer);
        }

        answers.to_writer(writer, ())
    }
}

fn encode_lookup_to_dns(bytes: &Vec<u8>) -> Vec<u8> {
    let dns = String::from_utf8(bytes.to_owned()).unwrap();
    let split = dns.split('.');
    let mut encoded = Vec::new();
    for part in split {
        encoded.push(part.len() as u8);
        encoded.extend(part.as_bytes());
    }
    encoded.push(b'\0');
    encoded
}


fn construct_lookups(bytes: &[u8], read_so_far: usize, qdcount: u16) -> Vec<Question> {
    let mut lookup_parts = "".to_owned();
    let mut lookups = Vec::new();

    let mut i: u8 = 0;
    for _ in 0..qdcount {
        let mut length: u8;
        loop {
            length = bytes[i as usize];

            if length == 0b00000000 {
                let question = Question {
                    name: lookup_parts[0..lookup_parts.len() - 1].to_owned().into_bytes(),
                    _type: ((bytes[i as usize + 2] as u16) << 8) | bytes[i as usize + 1] as u16,
                    class: ((bytes[i as usize + 4] as u16) << 8) | bytes[i as usize + 3] as u16,
                };

                lookups.push(question);
                lookup_parts = "".to_owned();
                i += 5;
                break;
            }

            if length == 0b11000000 {
                i = bytes[(i + 1) as usize] - read_so_far as u8;
                continue;
            }

            lookup_parts.push_str(String::from_utf8_lossy(&bytes[(i + 1) as usize..(i + length + 1) as usize]).as_ref());
            lookup_parts.push('.');

            i += length + 1;
        }
    }

    lookups
}

fn main() {
    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");

    // DNS is limited to 512 bytes
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((_, source)) => {
                match Message::from_bytes((&buf, 0)) {
                    Ok(result) => {
                        let mut message = result.1;

                        message.header.qr = 1;
                        message.header.ancount = 2;
                        message.header.rcode = if message.header.opcode == 0 { 0 } else { 4 };

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
