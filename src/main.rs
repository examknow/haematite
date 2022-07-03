mod channel;
mod channel_user;
mod line;
mod network;
mod server;
mod user;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::str::from_utf8;

use colored::{Color, Colorize};

use channel::Channel;
use line::Line;
use network::Network;
use server::Server;
use user::User;

struct Haematite {
    network: Network,
    me: Server,
    uplink: Option<String>,
}

impl Haematite {
    fn new(me: Server) -> Self {
        Haematite {
            network: Network::new(),
            me,
            uplink: None,
        }
    }

    fn handle_line(&mut self, socket: &TcpStream, line: &Line) -> bool {
        match line.command {
            "PASS" => self.uplink = Some(line.args[3].to_string()),
            "SERVER" => {
                self.network.add_server(Server {
                    sid: self.uplink.take().unwrap(),
                    name: line.args[0].to_string(),
                    description: line.args[2].to_string(),
                    ..Default::default()
                });
            }
            "SID" => {
                let server = Server {
                    sid: line.args[2].to_string(),
                    name: line.args[0].to_string(),
                    description: line.args[3].to_string(),
                    ..Default::default()
                };
                self.network.add_server(server);
            }
            "SQUIT" => {
                let sid = line.args[0];
                self.network.del_server(sid);
            }
            //:420AAAABC QUIT :Quit: Reconnecting
            "QUIT" => {
                let uid = line.source.unwrap();
                let sid = &uid[..3];
                let server = self.network.get_server_mut(sid);
                server.del_user(uid);
            }
            //:420 EUID jess 1 1656880345 +QZaioswz a0Ob4s0oLV test. fd84:9d71:8b8:1::1 420AAAABD husky.vpn.lolnerd.net jess :big meow
            "EUID" => {
                let uid = line.args[7].to_string();
                let nickname = line.args[0].to_string();
                let username = line.args[4].to_string();
                let realname = line.args[10].to_string();
                let account = match line.args[9] {
                    "*" => None,
                    account => Some(account.to_string()),
                };
                let ip = match line.args[6] {
                    "0" => None,
                    ip => Some(ip.to_string()),
                };
                let realhost = line.args[8].to_string();
                let showhost = line.args[5].to_string();

                let server = self.network.get_server_mut(line.source.unwrap());
                server.add_user(
                    uid,
                    User::new(
                        nickname, username, realname, account, ip, realhost, showhost,
                    ),
                );
            }
            //:420AAAABC AWAY :afk
            "AWAY" => {
                let uid = line.source.unwrap();
                let sid = &uid[..3];
                let server = self.network.get_server_mut(sid);
                server.get_user_mut(uid).away = line.args.first().map(|r| r.to_string());
            }
            //:420AAAABC OPER jess admin
            "OPER" => {
                let uid = line.source.unwrap();
                let sid = &uid[..3];
                let server = self.network.get_server_mut(sid);
                server.get_user_mut(uid).oper = Some(line.args[0].to_string());
            }
            "SJOIN" => {
                //:420 SJOIN 1640815917 #gaynet +MOPnst :@00AAAAAAC 420AAAABC
                let name = line.args[1].to_string();
                let users = line.args[3].split(' ').map(|u| u.to_owned());
                self.network.add_channel(name, Channel::new(users));
            }
            "PING" => {
                let source = match line.source {
                    Some(source) => source,
                    None => line.args[0],
                };
                send(
                    socket,
                    format!(":{} PONG {} {}", self.me.sid, self.me.name, source),
                );
            }
            "NOTICE" => { /* silently eat */ }
            _ => {
                return false;
            }
        }
        true
    }
}

const PASSWORD: &str = "8m1RXdPW2HG8lakqJF53N6DYZRA6xRy0ORjIqod65RWok482rhgBQUfNTYcaJorJ";

fn send(mut socket: &TcpStream, data: String) {
    println!("> {}", data);
    socket.write_all(data.as_bytes()).expect("asd");
    socket.write_all(b"\r\n").expect("asd");
}

fn main() {
    let mut haematite = Haematite::new(Server {
        sid: String::from("111"),
        name: String::from("haematite.vpn.lolnerd.net"),
        description: String::from("haematite psuedoserver"),
        ..Default::default()
    });

    let socket = TcpStream::connect("husky.vpn.lolnerd.net:6667").expect("failed to connect");

    send(
        &socket,
        format!("PASS {} TS 6 :{}", PASSWORD, haematite.me.sid),
    );
    send(
        &socket,
        "CAPAB :BAN CHW CLUSTER ECHO ENCAP EOPMOD EUID EX IE KLN KNOCK MLOCK QS RSFNC SAVE SERVICES TB UNKLN".to_string(),
    );
    send(
        &socket,
        format!(
            "SERVER {} 1 :{}",
            haematite.me.name, haematite.me.description
        ),
    );

    let mut reader = BufReader::with_capacity(512, &socket);
    let mut buffer = Vec::<u8>::with_capacity(512);
    loop {
        let len = reader.read_until(b'\n', &mut buffer).unwrap_or(0);
        if len == 0 {
            break;
        }

        // chop off \r\n
        buffer.drain(len - 2..len);

        let line = Line::from(&buffer);
        let handled = haematite.handle_line(&socket, &line);

        let printable = from_utf8(&buffer).unwrap().to_string();
        println!(
            "< {}",
            match handled {
                true => printable.normal(),
                false => printable.color(Color::Red),
            }
        );

        buffer.clear();
    }
}
