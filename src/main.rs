use regex::Regex;
use prometheus::{IntCounter, IntCounterVec, register_int_counter, register_int_counter_vec, Encoder, TextEncoder};
use std::fs::File;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;

fn main() {
    let dhcpd_leases_total_counter = register_int_counter!(
        "dhcpd_leases_total",
        "Total number of active DHCP leases"
    )
    .unwrap();

    let dhcpd_leases_subnet_counter = register_int_counter_vec!(
        "dhcpd_leases_subnet",
        "Number of active DHCP leases per subnet",
        &["subnet"]
    )
    .unwrap();

    let listener = TcpListener::bind("127.0.0.1:1337").expect("Failed to bind to port 1337!");

    for stream in listener.incoming() {
        let stream = stream.expect("Failed to establish connection!");
        let total_counter = dhcpd_leases_total_counter.clone();
        let subnet_counter = dhcpd_leases_subnet_counter.clone();
        thread::spawn(move || {
            handle_connection(stream, total_counter, subnet_counter);
        });
    }
}

fn handle_connection(
    mut stream: TcpStream,
    dhcpd_leases_total_counter: IntCounter,
    dhcpd_leases_subnet_counter: IntCounterVec,
) {
    let mut content = String::new();
    let mut file = File::open("/var/dhcpd/var/db/dhcpd.leases").expect("Failed to open DHCPd leasefile!");
    file.read_to_string(&mut content).expect("Failed to read DHCPd leasefile!");

    let regex_pattern = r"lease (\d+\.\d+\.\d+\.\d+) \{[^}]*\shardware\sethernet ([^;]+);[^}]*\}";
    let regex = Regex::new(regex_pattern).expect("Failed to compile regex");

    for capture in regex.captures_iter(&content) {
        let subnet = capture.get(1).unwrap().as_str();
        dhcpd_leases_total_counter.inc();
        dhcpd_leases_subnet_counter.with_label_values(&[subnet]).inc();
    }

    let metric_families = prometheus::gather();
    let encoder = TextEncoder::new();
    let mut buffer = vec![];

    encoder.encode(&metric_families, &mut buffer).expect("Failed to encode metrics!");
    stream.write_all(&buffer).expect("Failed to write metrics to stream!");
}
