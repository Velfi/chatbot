use anyhow::Context;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;
use std::{collections::VecDeque, time::Instant};
use tracing::{debug, error, info, trace};
use wasapi::*;

// Capture loop, capture samples and send in chunks of "chunksize" frames to channel
fn capture_loop(
    tx_capt: std::sync::mpsc::SyncSender<Vec<u8>>,
    chunksize: usize,
) -> Result<(), anyhow::Error> {
    // Use `Direction::Capture` for normal capture,
    // or `Direction::Render` for loopback mode (for capturing from a playback device).
    let device = get_default_device(&Direction::Capture).unwrap();

    let mut audio_client = device.get_iaudioclient().unwrap();

    let desired_format = WaveFormat::new(32, 32, &SampleType::Float, 44100, 2);

    let blockalign = desired_format.get_blockalign();
    debug!("Desired capture format: {:?}", desired_format);

    let (def_time, min_time) = audio_client.get_periods().unwrap();
    debug!("default period {}, min period {}", def_time, min_time);

    audio_client
        .initialize_client(
            &desired_format,
            min_time as i64,
            &Direction::Capture,
            &ShareMode::Shared,
            true,
        )
        .unwrap();
    debug!("initialized capture");

    let h_event = audio_client.set_get_eventhandle().unwrap();

    let buffer_frame_count = audio_client.get_bufferframecount().unwrap();

    let render_client = audio_client.get_audiocaptureclient().unwrap();
    let mut sample_queue: VecDeque<u8> = VecDeque::with_capacity(
        100 * blockalign as usize * (1024 + 2 * buffer_frame_count as usize),
    );
    let session_control = audio_client.get_audiosessioncontrol().unwrap();

    debug!("state before start: {:?}", session_control.get_state());
    audio_client.start_stream().unwrap();
    debug!("state after start: {:?}", session_control.get_state());
    let start = Instant::now();
    let timeout = Duration::from_secs(5);
    info!("recording audio for {timeout:?}");

    loop {
        if start.elapsed() > timeout {
            info!("stopping capture after {timeout:?} timeout");
            audio_client.stop_stream().unwrap();
            break;
        }

        while sample_queue.len() > (blockalign as usize * chunksize as usize) {
            debug!("pushing samples");
            let mut chunk = vec![0u8; blockalign as usize * chunksize as usize];
            for element in chunk.iter_mut() {
                *element = sample_queue.pop_front().unwrap();
            }
            tx_capt.send(chunk)?;
        }
        trace!("capturing");
        render_client
            .read_from_device_to_deque(blockalign as usize, &mut sample_queue)
            .unwrap();
        if h_event.wait_for_event(3000).is_err() {
            error!("timeout error, stopping capture");
            audio_client.stop_stream().unwrap();
            break;
        }
    }
    Ok(())
}

// Main loop
fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt::init();

    initialize_mta()?;

    let (tx_capt, rx_capt): (
        std::sync::mpsc::SyncSender<Vec<u8>>,
        std::sync::mpsc::Receiver<Vec<u8>>,
    ) = mpsc::sync_channel(2);
    let chunksize = 4096;

    let channels = 2;
    let sample_rate = 44100;

    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create("recorded.wav", spec)?;

    // Capture
    let _handle = thread::Builder::new()
        .name("Capture".to_string())
        .spawn(move || {
            let result = capture_loop(tx_capt, chunksize);
            if let Err(err) = result {
                error!("Capture failed with error {}", err);
            }
        });

        
        loop {
            match rx_capt.recv() {
                Ok(chunk) => {
                    debug!("writing to file");
                    for sample in chunk.chunks_exact(4) {
                    let sample = f32::from_le_bytes([sample[0], sample[1], sample[2], sample[3]]);
                    writer.write_sample(sample).context("writing f32 sample")?;
                }
            }
            Err(err) => {
                error!("Some error {}", err);
                break;
            }
        }
    }
    
    info!("Saving captured raw data to 'recorded.wav'");
    writer.flush().context("flushing the WAV writer")?;
    writer.finalize().context("finalizing the WAV writer")?;

    Ok(())
}
