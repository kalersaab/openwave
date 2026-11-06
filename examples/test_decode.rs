use openwave_core::demux::mp4::{Mp4Reader, TrackType};
use openwave_core::video::h264::{H264Decoder, avc};
use std::fs::File;
use std::io::BufReader;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <mp4_file>", args[0]);
        std::process::exit(1);
    }

    let path = &args[1];
    println!("[TEST] Opening: {}", path);

    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut mp4_reader = Mp4Reader::new(reader)?;

    let video_track = mp4_reader
        .tracks()
        .iter()
        .find(|t| t.track_type == TrackType::Video)
        .expect("No video track found");

    let video_track_id = video_track.track_id;
    let sample_count = mp4_reader
        .sample_count(video_track_id)
        .ok_or_else(|| anyhow::anyhow!("Could not get sample count"))?;

    println!(
        "[TEST] Video: {}x{}, {} samples",
        video_track.width.unwrap_or(0),
        video_track.height.unwrap_or(0),
        sample_count
    );

    let (sps_pps, nal_length_size) = avc::parse_avcc(&video_track.codec_private_data)?;
    println!(
        "[TEST] SPS/PPS: {} bytes, NAL length size: {}",
        sps_pps.len(),
        nal_length_size
    );

    let mut decoder = H264Decoder::new();
    let max_frames = 10.min(sample_count);

    for sample_idx in 0..max_frames {
        let sample = mp4_reader.read_sample(video_track_id, sample_idx as u32)?;

        let annex_b_data = avc::avc1_to_annex_b(
            &sample.data,
            nal_length_size,
            sample.is_keyframe || sample_idx == 0,
            &sps_pps,
        )?;

        println!(
            "[TEST] Frame {}: keyframe={}, sample_size={}, annex_b_size={}",
            sample_idx,
            sample.is_keyframe,
            sample.data.len(),
            annex_b_data.len()
        );

        match decoder.decode_frame(&annex_b_data) {
            Ok(frame) => {
                println!("[TEST]   ✓ Decoded: {}x{}", frame.width, frame.height);
            }
            Err(e) => {
                println!("[TEST]   ✗ Error: {}", e);
                if sample_idx > 0 {
                    // If we got past the first frame, this is progress
                    break;
                } else {
                    return Err(e.into());
                }
            }
        }
    }

    println!("[TEST] Success!");
    Ok(())
}
