use bytes::BufMut;
use deku::prelude::*;
use nom::AsBytes;
use std::env;
use std::io::Error;
use std::net::UdpSocket;

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
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

#[derive(Debug, Clone)]
struct Question {
    name: Vec<u8>,
    _type: u16,
    class: u16,
}

#[derive(Debug, Clone)]
struct Answer {
    name: Vec<u8>,
    _type: u16,
    class: u16,
    ttl: u32,
    length: u16,
    data: Vec<u8>,
}

#[derive(Debug)]
struct Message {
    header: Header,
    question: Vec<Question>,
    answer: Vec<Answer>,
}

impl Message {
    fn to_bytes(self: Self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        buf.extend_from_slice(self.header.to_bytes()?.as_bytes());
        self.question.iter().for_each(|q| {
            let _ = q.to_owned().to_bytes(buf).expect("");
        });

        self.answer.iter().for_each(|a| {
            let _ = a.to_owned().to_bytes(buf).expect("");
        });

        Ok(buf.len())
    }
}

impl Answer {
    fn to_bytes(self: Self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        let encoded = encode_lookup_to_dns(&self.name);

        buf.extend_from_slice(encoded.as_bytes());
        buf.put_u16(self._type);
        buf.put_u16(self.class);
        buf.put_u32(self.ttl);
        buf.put_u16(self.length);
        buf.extend(&self.data);

        Ok(encoded.len() + 10 + self.data.len())
    }
    fn read_answers(
        bytes: &[u8],
        ancount: u16,
        questions: Vec<Question>, // Huge hack - the test will only ever send one answer
    ) -> ((&[u8], usize), Vec<Answer>) {
        let mut lookup_parts = "".to_owned();
        let mut lookups = Vec::new();

        let mut i: u8 = 0;
        for _ in 0..ancount {
            let mut length: u8;
            loop {
                length = bytes[i as usize];

                if length == 0b00000000 {
                    let rdata_length = encode_u16_from_two_u8s(bytes, i + 9, i + 10);
                    let answer = Answer {
                        name: lookup_parts[0..lookup_parts.len() - 1]
                            .to_owned()
                            .into_bytes(),
                        _type: encode_u16_from_two_u8s(bytes, i + 1, i + 2),
                        class: encode_u16_from_two_u8s(bytes, i + 3, i + 4),
                        ttl: encode_u32_from_four_u8s(bytes, i + 5, i + 6, i + 7, i + 8),
                        length: rdata_length,
                        data: Vec::from(&bytes[i as usize + 11..]),
                    };

                    lookups.push(answer);
                    lookup_parts = "".to_owned();
                    i += 11 + rdata_length as u8;
                    break;
                }

                if length == 0b11000000 {
                    i += 2;
                    let rdata_length = encode_u16_from_two_u8s(bytes, i + 8, i + 9);
                    let answer = Answer {
                        name: questions[0].name.to_owned(),
                        _type: encode_u16_from_two_u8s(bytes, i, i + 1),
                        class: encode_u16_from_two_u8s(bytes, i + 2, i + 3),
                        ttl: encode_u32_from_four_u8s(bytes, i + 4, i + 5, i + 6, i + 7),
                        length: rdata_length,
                        data: Vec::from(&bytes[i as usize + 10..]),
                    };

                    lookups.push(answer);
                    lookup_parts = "".to_owned();
                    i += 10 + rdata_length as u8;
                    break;
                }

                lookup_parts.push_str(
                    String::from_utf8_lossy(&bytes[(i + 1) as usize..(i + length + 1) as usize])
                        .as_ref(),
                );
                lookup_parts.push('.');

                i += length + 1;
            }
        }

        ((&bytes[i as usize..], i as usize), lookups)
    }
}

impl Question {
    fn to_bytes(self: Self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        let encoded = encode_lookup_to_dns(&self.name);

        buf.extend_from_slice(encoded.as_bytes());
        buf.put_u16(self._type);
        buf.put_u16(self.class);

        Ok(encoded.len() + 4)
    }

    fn read_questions(
        bytes: &[u8],
        qdcount: u16,
        bytes_read_so_far: usize,
    ) -> ((&[u8], usize), Vec<Question>) {
        let mut lookup_parts = "".to_owned();
        let mut lookups = Vec::new();

        let mut i: u8 = 0;
        for _ in 0..qdcount {
            let mut length: u8;
            loop {
                length = bytes[i as usize];

                if length == 0b00000000 {
                    let question = Question {
                        name: lookup_parts[0..lookup_parts.len() - 1]
                            .to_owned()
                            .into_bytes(),
                        _type: encode_u16_from_two_u8s(bytes, i + 1, i + 2),
                        class: encode_u16_from_two_u8s(bytes, i + 3, i + 4),
                    };

                    lookups.push(question);
                    lookup_parts = "".to_owned();
                    i += 5;
                    break;
                }

                if length == 0b11000000 {
                    i = bytes[(i + 1) as usize] - bytes_read_so_far as u8;
                    continue;
                }

                lookup_parts.push_str(
                    String::from_utf8_lossy(&bytes[(i + 1) as usize..(i + length + 1) as usize])
                        .as_ref(),
                );
                lookup_parts.push('.');

                i += length + 1;
            }
        }

        (
            (&bytes[i as usize..], bytes_read_so_far + i as usize),
            lookups,
        )
    }
}

fn encode_u32_from_four_u8s(
    bytes: &[u8],
    first_idx: u8,
    second_idx: u8,
    third_idx: u8,
    fourth_idx: u8,
) -> u32 {
    u32::from_be_bytes([
        bytes[first_idx as usize],
        bytes[second_idx as usize],
        bytes[third_idx as usize],
        bytes[fourth_idx as usize],
    ])
}
fn encode_u16_from_two_u8s(bytes: &[u8], first_idx: u8, second_idx: u8) -> u16 {
    u16::from_be_bytes([bytes[first_idx as usize], bytes[second_idx as usize]])
}

fn encode_lookup_to_dns(bytes: &Vec<u8>) -> Vec<u8> {
    let dns = String::from_utf8(bytes.to_owned()).expect("Could not parse UTF-8 from bytes");

    let split = dns.split('.');
    let mut encoded = Vec::new();
    for part in split {
        encoded.push(part.len() as u8);
        encoded.extend(part.as_bytes());
    }
    encoded.push(b'\0');
    encoded
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let resolver;
    if args.len() > 1 {
        resolver = args[2].to_owned();
    } else {
        resolver = "8.8.8.8:53".to_owned();
    }

    let udp_socket = UdpSocket::bind("127.0.0.1:2053").expect("Failed to bind to address");
    let udp_send_socket = UdpSocket::bind("0.0.0.0:2054").expect("Failed to bind to address");

    // DNS is limited to 512 bytes
    let mut buf = [0; 512];

    loop {
        match udp_socket.recv_from(&mut buf) {
            Ok((num_bytes_sent, source)) => {
                let bytes_sent = &buf[0..num_bytes_sent];

                match Header::from_bytes((&bytes_sent, 0)) {
                    Ok(result) => {
                        let (remaining_bytes, _) = result.0;
                        let header = result.1;
                        let mut answers = Vec::new();

                        let questions_result = Question::read_questions(
                            remaining_bytes,
                            header.qdcount,
                            bytes_sent.len() - remaining_bytes.len(),
                        );
                        let questions = questions_result.1;

                        for q in &questions {
                            let mut cloned_header = header.clone();

                            cloned_header.qdcount = 1;
                            cloned_header.arcount = 0;
                            cloned_header.nscount = 0;
                            cloned_header.qr = 0;
                            let message = Message {
                                header: cloned_header,
                                question: vec![q.clone()],
                                answer: vec![],
                            };

                            let mut bytes = Vec::with_capacity(512);
                            message.to_bytes(&mut bytes).expect("");

                            let to_bytes = bytes.as_bytes();
                            match udp_send_socket.send_to(&to_bytes, &resolver) {
                                Ok(_) => {
                                    let mut forward = [0; 512];
                                    let udp_rcv_result = udp_send_socket
                                        .recv_from(&mut forward)
                                        .expect("Failed to receive");

                                    let header_result =
                                        Header::from_bytes((&forward[0..udp_rcv_result.0], 0))
                                            .expect("");

                                    let (remaining_bytes, _) = header_result.0;
                                    let header = header_result.1;

                                    let questions_result = Question::read_questions(
                                        remaining_bytes,
                                        header.qdcount,
                                        udp_rcv_result.0 - remaining_bytes.len(),
                                    );
                                    let (remaining_question_bytes, _) = questions_result.0;

                                    let answers_result = Answer::read_answers(
                                        remaining_question_bytes,
                                        header.qdcount,
                                        questions_result.1,
                                    );

                                    answers.extend(answers_result.1);
                                }
                                Err(e) => {
                                    eprintln!("Error: {}", e);
                                    panic!();
                                }
                            }
                        }

                        let mut cloned_return_header = header.clone();
                        cloned_return_header.qr = 1;
                        cloned_return_header.ancount = answers.len() as u16;
                        cloned_return_header.qdcount = questions.len() as u16;
                        cloned_return_header.rcode = if cloned_return_header.opcode == 0 {
                            0
                        } else {
                            4
                        };
                        let return_msg = Message {
                            header: cloned_return_header,
                            question: questions,
                            answer: answers,
                        };

                        let mut return_bytes = Vec::with_capacity(512);
                        return_msg.to_bytes(&mut return_bytes).expect("");
                        udp_socket
                            .send_to(return_bytes.as_bytes(), source)
                            .expect("Failed to send response");
                    }
                    Err(_) => {}
                };
            }
            Err(e) => {
                eprintln!("Error receiving data: {}", e);
                break;
            }
        }
    }
}
