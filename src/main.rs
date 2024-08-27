use deku::prelude::*;
use nom::AsBytes;
use std::env;
use std::net::UdpSocket;
use totally_incomplete_dns::model::{Answer, Header, Message, Question};

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
                            message
                                .to_bytes(&mut bytes)
                                .expect("Failed to encode message");

                            let to_bytes = bytes.as_bytes();
                            match udp_send_socket.send_to(&to_bytes, &resolver) {
                                Ok(_) => {
                                    let mut forward = [0; 512];
                                    let udp_rcv_result = udp_send_socket
                                        .recv_from(&mut forward)
                                        .expect("Failed to receive");

                                    let header_result =
                                        Header::from_bytes((&forward[0..udp_rcv_result.0], 0))
                                            .expect("Failed to parse Header");

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
                        return_msg
                            .to_bytes(&mut return_bytes)
                            .expect("Failed to encode return Message");

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
