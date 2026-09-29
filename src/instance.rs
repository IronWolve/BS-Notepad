use serde::{Deserialize, Serialize};
use std::{
    fs::{File, OpenOptions},
    hash::{BuildHasher, Hasher},
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
const MAX_MESSAGE: usize = 64 * 1024;
#[derive(Serialize, Deserialize)]
struct Endpoint {
    port: u16,
    token: String,
}
#[derive(Serialize, Deserialize)]
struct Message {
    token: String,
    paths: Vec<String>,
}
pub struct Guard {
    _file: File,
    root: PathBuf,
}
impl Drop for Guard {
    fn drop(&mut self) {
        release(&self.root);
    }
}
pub fn acquire(root: &Path) -> std::io::Result<Guard> {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(root.join("instance.lock"))?;
    file.try_lock().map_err(|error| match error {
        std::fs::TryLockError::WouldBlock => std::io::Error::from(std::io::ErrorKind::WouldBlock),
        std::fs::TryLockError::Error(error) => error,
    })?;
    Ok(Guard {
        _file: file,
        root: root.into(),
    })
}
fn frame(stream: &mut TcpStream, budget: Duration) -> std::io::Result<Vec<u8>> {
    let deadline = Instant::now() + budget;
    fn read_until(
        stream: &mut TcpStream,
        mut buffer: &mut [u8],
        deadline: Instant,
    ) -> std::io::Result<()> {
        while !buffer.is_empty() {
            let remaining = deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::TimedOut))?;
            stream.set_read_timeout(Some(remaining))?;
            match stream.read(buffer) {
                Ok(0) => return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof)),
                Ok(count) => buffer = &mut buffer[count..],
                Err(error) if error.kind() == std::io::ErrorKind::Interrupted => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
    let mut size = [0; 4];
    read_until(stream, &mut size, deadline)?;
    let len = u32::from_be_bytes(size) as usize;
    if len > MAX_MESSAGE {
        return Err(std::io::Error::other("Request too large"));
    }
    let mut bytes = vec![0; len];
    read_until(stream, &mut bytes, deadline)?;
    Ok(bytes)
}
fn write_frame(stream: &mut TcpStream, bytes: &[u8]) -> std::io::Result<()> {
    if bytes.len() > MAX_MESSAGE {
        return Err(std::io::Error::other("Request too large"));
    }
    stream.write_all(&(bytes.len() as u32).to_be_bytes())?;
    stream.write_all(bytes)
}
pub fn hand_off(root: &Path, paths: &[PathBuf]) -> bool {
    let run = || -> std::io::Result<()> {
        let endpoint: Endpoint =
            serde_json::from_slice(&std::fs::read(root.join("instance.port"))?)?;
        let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, endpoint.port).into();
        let mut stream = TcpStream::connect_timeout(&address, Duration::from_millis(250))?;
        stream.set_read_timeout(Some(Duration::from_millis(900)))?;
        stream.set_write_timeout(Some(Duration::from_millis(900)))?;
        write_frame(
            &mut stream,
            &serde_json::to_vec(&Message {
                token: endpoint.token,
                paths: paths
                    .iter()
                    .map(|p| {
                        p.to_str()
                            .map(str::to_owned)
                            .ok_or_else(|| std::io::Error::other("Unsupported filename encoding"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            })?,
        )?;
        if frame(&mut stream, Duration::from_millis(900))? != b"accepted" {
            return Err(std::io::Error::other("Invalid acknowledgment"));
        }
        Ok(())
    };
    run().is_ok()
}
pub fn listen<F>(root: &Path, on_request: F) -> std::io::Result<()>
where
    F: Fn(Vec<String>) + Send + 'static,
{
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, 0))?;
    let mut hash = std::collections::hash_map::RandomState::new().build_hasher();
    hash.write_u32(std::process::id());
    let token = format!(
        "{:016x}{:x}",
        hash.finish(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    crate::storage::write_private_atomic(
        &root.join("instance.port"),
        &serde_json::to_vec(&Endpoint {
            port: listener.local_addr()?.port(),
            token: token.clone(),
        })?,
    )?;
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = match stream {
                Ok(stream) => stream,
                Err(_) => {
                    std::thread::sleep(Duration::from_millis(100));
                    continue;
                }
            };
            let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
            let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
            if let Ok(bytes) = frame(&mut stream, Duration::from_millis(300)) {
                if let Ok(message) = serde_json::from_slice::<Message>(&bytes) {
                    if message.token == token
                        && message.paths.len() <= 128
                        && message
                            .paths
                            .iter()
                            .all(|p| !p.contains('\0') && p.len() <= 32768)
                    {
                        on_request(message.paths);
                        let _ = write_frame(&mut stream, b"accepted");
                    }
                }
            }
        }
    });
    Ok(())
}
pub fn release(root: &Path) {
    let _ = std::fs::remove_file(root.join("instance.port"));
}
