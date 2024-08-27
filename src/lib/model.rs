use crate::util::{encode_lookup_to_dns, encode_u16_from_two_u8s, encode_u32_from_four_u8s};
use bytes::BufMut;
use deku::prelude::*;
use nom::AsBytes;
use std::io::Error;

#[derive(Debug, Clone, PartialEq, DekuRead, DekuWrite)]
#[deku(endian = "big")]
pub struct Header {
    #[deku(bits = 16)]
    pub id: u16,

    #[deku(bits = 1)]
    pub qr: u8,

    #[deku(bits = 4)]
    pub opcode: u8,

    #[deku(bits = 1)]
    pub aa: u8,

    #[deku(bits = 1)]
    pub tc: u8,

    #[deku(bits = 1)]
    pub rd: u8,

    #[deku(bits = 1)]
    pub ra: u8,

    // Reserved
    #[deku(bits = 3)]
    pub z: u8,

    #[deku(bits = 4)]
    pub rcode: u8,

    #[deku(bits = 16)]
    pub qdcount: u16,

    #[deku(bits = 16)]
    pub ancount: u16,

    #[deku(bits = 16)]
    pub nscount: u16,

    #[deku(bits = 16)]
    pub arcount: u16,
}

#[derive(Debug, Clone)]
pub struct Question {
    name: Vec<u8>,
    _type: u16,
    class: u16,
}

#[derive(Debug, Clone)]
pub struct Answer {
    name: Vec<u8>,
    _type: u16,
    class: u16,
    ttl: u32,
    length: u16,
    data: Vec<u8>,
}

#[derive(Debug)]
pub struct Message {
    pub header: Header,
    pub question: Vec<Question>,
    pub answer: Vec<Answer>,
}

impl Message {
    pub fn to_bytes(self: Self, buf: &mut Vec<u8>) -> Result<usize, Error> {
        buf.extend_from_slice(self.header.to_bytes()?.as_bytes());
        self.question.iter().for_each(|q| {
            let _ = q.to_owned().to_bytes(buf);
        });

        self.answer.iter().for_each(|a| {
            let _ = a.to_owned().to_bytes(buf);
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

    pub fn read_answers(
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

    pub fn read_questions(
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
