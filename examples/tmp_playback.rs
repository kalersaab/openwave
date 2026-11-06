use openwave::media::NativePlayer;
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    let path = std::env::args().nth(1).unwrap();
    let mut player = NativePlayer::new()?;
    player.open(&path)?;
    player.play()?;

    let start = Instant::now();
    let mut n = 0;
    while start.elapsed() < Duration::from_secs(3) {
        if let Some(frame) = player.take_decoded_frame() {
            let nonzero = frame.pixels.iter().filter(|&&b| b != 0).count();
            let mean = frame.pixels.iter().map(|&b| b as u64).sum::<u64>()
                / frame.pixels.len().max(1) as u64;
            println!(
                "frame {n}: {}x{} pitch={} len={} nonzero={} mean={mean}",
                frame.width,
                frame.height,
                frame.pitch,
                frame.pixels.len(),
                nonzero
            );
            if n == 0 {
                let mut out = format!("P6\n{} {}\n255\n", frame.width, frame.height).into_bytes();
                out.extend_from_slice(&frame.pixels);
                std::fs::write("/tmp/native_frame0.ppm", out)?;
            }
            n += 1;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    println!("total frames: {n}");
    Ok(())
}
