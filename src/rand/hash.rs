

pub fn hash(mut seed: u32) -> u32 {
    seed ^= 2747636419;
    seed = seed.wrapping_mul(2654435769);
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(2654435769);
    seed ^= seed >> 16;
    seed = seed.wrapping_mul(2654435769);
    seed
}

#[test]
fn test() {
    let start_time = std::time::Instant::now();
    let mut delta = 0.0;
    for i in 0..100 {
        let v = (hash(i + start_time.elapsed().subsec_millis() % 10) as f64) / 4294967295.0;
        delta += 2.0 * v - v;
    }
    println!("{}", delta);
}