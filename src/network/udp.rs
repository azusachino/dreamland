// multicast examples
#[cfg(test)]
mod udp_tests {
    use std::net::{Ipv4Addr, SocketAddrV4, UdpSocket};
    use std::str::FromStr;
    use std::time::Duration;

    #[test]
    fn udp_sender() -> anyhow::Result<()> {
        let multicast_group = Ipv4Addr::from_str("239.0.0.1")?; // A common private multicast address
        let multicast_port = 8080;
        let send_addr = SocketAddrV4::new(multicast_group, multicast_port);

        // Bind to the sender's specific IP if available, or 0.0.0.0 if not strictly needed
        // Binding to 0.0.0.0 lets the OS pick the source interface based on routing.
        // If you have multiple network cards and want to ensure it uses 172.8.0.100:
        let local_sender_ip = Ipv4Addr::from_str("127.0.0.1")?;
        let socket = UdpSocket::bind(SocketAddrV4::new(local_sender_ip, 0))?; // Bind to sender's IP, ephemeral port

        // Set multicast loopback (true means sender also receives its own messages, good for testing)
        socket.set_multicast_loop_v4(true)?;
        // Set Time-To-Live (TTL): 1 means it won't cross routers (stays on local network)
        // Higher values allow it to cross routers, up to the TTL value.
        socket.set_multicast_ttl_v4(1)?;

        println!("UDP Multicast Sender bound to: {}", socket.local_addr()?);
        println!("Sending to multicast group: {}", send_addr);

        let message = "Hello from 127.0.0.1 via Multicast!";
        let data = message.as_bytes();

        loop {
            println!("Sending message...");
            socket.send_to(data, send_addr)?;
            std::thread::sleep(Duration::from_secs(2)); // Send every 2 seconds
        }
    }

    #[test]
    fn udp_receiver() -> anyhow::Result<()> {
        let multicast_group = Ipv4Addr::from_str("239.0.0.1")?;
        let multicast_port = 8080;

        // We must bind to 0.0.0.0 (or the specific interface IP if multiple exist)
        // and the multicast port.
        let listen_addr = SocketAddrV4::new(Ipv4Addr::from_str("0.0.0.0")?, multicast_port);
        let socket = UdpSocket::bind(listen_addr)?;
        println!("UDP Multicast Receiver bound to: {}", socket.local_addr()?);

        // Crucial step: Join the multicast group on a specific interface.
        // Use Ipv4Addr::UNSPECIFIED (0.0.0.0) if you want to join on all available interfaces.
        // Or, specify your machine's IP (e.g., Ipv4Addr::from_str("172.8.0.101")?)
        let interface_ip = Ipv4Addr::UNSPECIFIED; // Or your specific machine IP
        socket.join_multicast_v4(&multicast_group, &interface_ip)?;
        println!(
            "Joined multicast group {} on interface {}",
            multicast_group, interface_ip
        );

        let mut buf = [0; 1024]; // A buffer to hold incoming data

        loop {
            match socket.recv_from(&mut buf) {
                Ok((number_of_bytes, src_addr)) => {
                    let received_data = String::from_utf8_lossy(&buf[..number_of_bytes]);
                    println!("Received '{}' from {} (multicast)", received_data, src_addr);
                }
                Err(e) => {
                    eprintln!("Error receiving: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
