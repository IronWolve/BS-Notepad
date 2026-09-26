#[path = "../src/icon.rs"]
mod icon;

fn chunk(output: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    output.extend((data.len() as u32).to_be_bytes());
    output.extend(kind);
    output.extend(data);
    let mut crc = 0xffff_ffffu32;
    for byte in kind.iter().chain(data) {
        crc ^= *byte as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb8_8320 } else { 0 };
        }
    }
    output.extend((!crc).to_be_bytes());
}
fn main() -> std::io::Result<()> {
    // A small dependency-free PNG writer; the system icon tools compress resized entries.
    let size = 1024u32;
    let rgba = icon::rgba(size);
    let mut scanlines = Vec::with_capacity(rgba.len() + size as usize);
    for row in rgba.chunks_exact(size as usize * 4) {
        scanlines.push(0);
        scanlines.extend(row);
    }
    let mut compressed = vec![0x78, 0x01];
    let count = scanlines.len().div_ceil(65535);
    for (index, block) in scanlines.chunks(65535).enumerate() {
        compressed.push(u8::from(index + 1 == count));
        let length = block.len() as u16;
        compressed.extend(length.to_le_bytes());
        compressed.extend((!length).to_le_bytes());
        compressed.extend(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for byte in &scanlines {
        a = (a + *byte as u32) % 65521;
        b = (b + a) % 65521;
    }
    compressed.extend(((b << 16) | a).to_be_bytes());
    let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut header = Vec::new();
    header.extend(size.to_be_bytes());
    header.extend(size.to_be_bytes());
    header.extend([8, 6, 0, 0, 0]);
    chunk(&mut png, b"IHDR", &header);
    chunk(&mut png, b"IDAT", &compressed);
    chunk(&mut png, b"IEND", &[]);
    std::fs::write(std::env::args_os().nth(1).expect("output PNG path"), png)
}
