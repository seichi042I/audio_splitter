// src/resampler.rs

use rubato::{Resampler, SincFixedIn, InterpolationType, InterpolationParameters, WindowFunction};
use hound::{WavReader, WavWriter, WavSpec};
use std::path::Path;

pub fn resample_wav(input_path: &Path, output_path: &Path, output_sample_rate: f64) -> Result<(), Box<dyn std::error::Error>> {
    // 入力WAVファイルを読み込む
    let mut reader = WavReader::open(input_path)?;
    let spec = reader.spec();
    let channels = spec.channels as usize;
    let input_sample_rate = spec.sample_rate as f64;

    // サンプルデータを読み込む
    let samples: Vec<f64> = match spec.sample_format {
        hound::SampleFormat::Float => {
            reader.samples::<f32>().map(|s| s.unwrap() as f64).collect()
        },
        hound::SampleFormat::Int => match spec.bits_per_sample {
            16 => {
                reader.samples::<i16>().map(|s| s.unwrap() as f64 / 32768.0).collect()
            },
            24 => {
                reader.samples::<i32>().map(|s| {
                    s.unwrap() as f64 / 8388608.0 // 24-bit normalization
                }).collect()
            },
            _ => panic!("Unsupported bit depth"),
        },
    };

    // チャンネルごとにデータを分割
    let mut input_samples = vec![Vec::new(); channels];
    for (i, &sample) in samples.iter().enumerate() {
        input_samples[i % channels].push(sample);
    }

    // リサンプラーの設定
    let params = InterpolationParameters {
        sinc_len: 256,
        f_cutoff: 0.95,
        interpolation: InterpolationType::Linear,
        oversampling_factor: 256,
        window: WindowFunction::BlackmanHarris2,
    };

    let mut resampler = SincFixedIn::<f64>::new(
        output_sample_rate / input_sample_rate,
        2.0,
        params,
        input_samples[0].len(),
        channels
    )?;

    // リサンプリングの実行
    let output_frames = resampler.process(&input_samples, None)?;

    // 出力WAVファイルの準備
    let spec = WavSpec {
        channels: channels as u16,
        sample_rate: output_sample_rate as u32,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = WavWriter::create(output_path, spec)?;

    // リサンプリングされたデータを書き込む
    if channels == 1 {
        for &sample in &output_frames[0] {
            writer.write_sample((sample * 32767.0) as i16)?;
        }
    } else {
        for frame in output_frames[0].iter().zip(output_frames[1].iter()) {
            writer.write_sample((frame.0 * 32767.0) as i16)?;
            writer.write_sample((frame.1 * 32767.0) as i16)?;
        }
    }

    Ok(())
}