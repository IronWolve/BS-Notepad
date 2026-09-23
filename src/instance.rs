use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;

/// Staying resident is the largest single win on how fast the app feels: the
/// engine boots once, and later opens re-show the window that already exists.
///
/// A loopback socket carries the request, with its port written beside the
/// binary so a second launch can find it.
fn port_file(root: &Path) -> std::path::PathBuf {
    root.join("instance.port")
}

/// Hands a path to an already running copy. True when one took it.
pub fn hand_off(root: &Path, argument: Option<&Path>) -> bool {
    let Ok(text) = std::fs::read_to_string(port_file(root)) else { return false };
    let Ok(port) = text.trim().parse::<u16>() else { return false };
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, port);
    let Ok(mut stream) = TcpStream::connect(address) else {
        // Nothing listening: the file is stale and will be replaced.
        let _ = std::fs::remove_file(port_file(root));
        return false;
    };
    let payload = argument.map(|p| p.display().to_string()).unwrap_or_default();
    stream.write_all(payload.as_bytes()).is_ok()
}

/// Listens for later launches. Each request arrives as a path, or empty to
/// just raise the window.
pub fn listen<F>(root: &Path, on_request: F)
where
    F: Fn(Option<String>) + Send + 'static,
{
    let Ok(listener) = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0)) else {
        return;
    };
    let Ok(address) = listener.local_addr() else { return };
    let _ = std::fs::write(port_file(root), address.port().to_string());

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { continue };
            let mut text = String::new();
            if stream.read_to_string(&mut text).is_ok() {
                let trimmed = text.trim().to_string();
                on_request(if trimmed.is_empty() { None } else { Some(trimmed) });
            }
        }
    });
}

pub fn release(root: &Path) {
    let _ = std::fs::remove_file(port_file(root));
}
