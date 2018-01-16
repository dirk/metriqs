use std::net::{ToSocketAddrs, UdpSocket};
use std::str;
use std::sync::mpsc::channel;
use std::thread;

use super::parse_metrics;
use super::super::super::collector::Collector;

/// Listens for StatsD UDP datagrams.
pub struct StatsdUdpListener {
    collector: Collector,
}

impl StatsdUdpListener {
    pub fn new(collector: Collector) -> StatsdUdpListener {
        StatsdUdpListener {
            collector,
        }
    }

    /// Spawns a separate thread that listens for StatsD UDP datagrams,
    /// received datagrams are sent back to the calling thread (this will
    /// block) and the parsed metrics are recorded in the store.
    pub fn listen<A: ToSocketAddrs>(&self, addr: A) {
        let (send, recv) = channel();

        let socket = UdpSocket::bind(addr).unwrap();

        thread::spawn(move || {
            // Big enough to hold an ethernet frame:
            //   https://github.com/etsy/statsd/blob/master/docs/metric_types.md#multi-metric-packets
            let mut buf = [0; 1500];
            loop {
                let (bytes_read, _) = match socket.recv_from(&mut buf) {
                    Ok(pair) => pair,
                    Err(_) => return,
                };

                // Get a string from just the amount of bytes read.
                let message: &str = match str::from_utf8(&buf[..bytes_read]) {
                    Ok(s) => s,
                    Err(_) => return,
                };

                send.send(message.to_owned()).unwrap();
            }
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
    } // fn listen
} // impl StatsdUdpListener
