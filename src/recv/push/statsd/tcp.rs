use std::io::{self, BufRead, BufReader};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::mpsc::{channel, Sender};
use std::thread;
use std::time::Duration;

use super::parse_metrics;
use super::super::super::collector::Collector;

/// Listens on a TCP socket for StatsD messages.
pub struct StatsdTcpListener {
    collector: Collector,
    addr: SocketAddr,
}

impl StatsdTcpListener {
    pub fn new<A: ToSocketAddrs>(collector: Collector, addr: A) -> Result<StatsdTcpListener, io::Error> {
        addr.to_socket_addrs()
            .map(|mut addrs| addrs.next().unwrap())
            .map(|addr| {
                StatsdTcpListener {
                    collector,
                    addr,
                }
            })
    }

    pub fn listen(&mut self) {
        let (send, recv) = channel();

        let listener = TcpListener::bind(self.addr).unwrap();
        thread::spawn(move || {
            StatsdTcpListener::accept_on_listener(listener, send)
        });

        for line in recv {
            match parse_metrics(line.trim_right().as_bytes()) {
                Ok(metrics) => {
                    let aggregated_metrics = metrics.into_iter()
                        .map(|metric| metric.into())
                        .collect();

                    self.collector.push(aggregated_metrics)
                },
                Err(_) => (),
            }
        }
    }

    fn accept_on_listener(listener: TcpListener, send: Sender<String>) {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    // Clients have 30 seconds to send us data before we'll drop.
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));

                    let send = send.clone();

                    thread::spawn(move || {
                        StatsdTcpListener::handle_client(stream, send)
                    });
                },
                Err(e) => panic!("Failed to listen on TCP socket: {}", e),
            }
        }
    }

    fn handle_client(stream: TcpStream, send: Sender<String>) {
        let mut reader = BufReader::new(stream);

        loop {
            let mut line = String::new();

            match reader.read_line(&mut line) {
                Err(err) => {
                    println!("Error reading StatsD line: {:?}", err);
                    break
                },
                Ok(0) => {
                    // Close if there are no more bytes.
                    break
                },
                Ok(_) => {
                    send.send(line).unwrap()
                },
            }
        }
    } // fn handle_client
} // struct StatsdTcpListener
