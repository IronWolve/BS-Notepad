// A notebook and pencil, drawn from the same geometry at every icon size.
fn inside(x: f32, y: f32, points: &[(f32, f32)]) -> bool {
    let mut hit = false;
    let mut j = points.len() - 1;
    for i in 0..points.len() {
        let (a, b) = points[i];
        let (c, d) = points[j];
        if (b > y) != (d > y) && x < (c - a) * (y - b) / (d - b) + a {
            hit = !hit;
        }
        j = i;
    }
    hit
}
fn pixel(x: f32, y: f32) -> [u8; 4] {
    let mut c = [0, 0, 0, 0];
    let dx = (x - 32.).abs() - 22.;
    let dy = (y - 32.).abs() - 22.;
    if dx.max(0.).powi(2) + dy.max(0.).powi(2) <= 36. {
        c = [34, 61, 88, 255];
    }
    if (13.0..45.0).contains(&x) && (10.0..54.0).contains(&y) {
        c = [221, 236, 244, 255];
    }
    if (13.0..19.0).contains(&x) && (10.0..54.0).contains(&y) {
        c = [71, 155, 205, 255];
    }
    for line in [20., 28., 36., 44.] {
        if (24.0..39.0).contains(&x) && (line..line + 2.).contains(&y) {
            c = [115, 143, 160, 255];
        }
    }
    if inside(
        x,
        y,
        &[(31., 49.), (48., 19.), (57., 24.), (40., 54.), (29., 59.)],
    ) {
        c = [28, 44, 62, 255];
    }
    if inside(x, y, &[(33., 48.), (49., 21.), (54., 24.), (38., 51.)]) {
        c = [244, 183, 77, 255];
    }
    if inside(x, y, &[(33., 48.), (38., 51.), (31., 55.)]) {
        c = [245, 237, 213, 255];
    }
    c
}
pub fn rgba(size: u32) -> Vec<u8> {
    let mut out = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let mut sum = [0u32; 4];
            for sy in 0..4 {
                for sx in 0..4 {
                    let c = pixel(
                        (x as f32 + (sx as f32 + 0.5) / 4.) * 64. / size as f32,
                        (y as f32 + (sy as f32 + 0.5) / 4.) * 64. / size as f32,
                    );
                    for k in 0..3 {
                        sum[k] += c[k] as u32 * c[3] as u32;
                    }
                    sum[3] += c[3] as u32;
                }
            }
            for value in sum.iter().take(3) {
                out.push(value.checked_div(sum[3]).unwrap_or(0) as u8);
            }
            out.push((sum[3] / 16) as u8);
        }
    }
    out
}
