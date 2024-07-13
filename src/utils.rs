use hound::{Sample, WavSpec, WavWriter};
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self,Error};
use std::fmt::Debug;

pub trait Normalizable {
    fn normalize(self) -> f64;
}

impl Normalizable for i16 {
    fn normalize(self) -> f64 {
        let max_i16_as_f64 = i16::MAX as f64;
        (self as f64) / max_i16_as_f64
    }
}

impl Normalizable for i32 {
    fn normalize(self) -> f64 {
        let max_24bit_as_f64 = 8388607.0; // 24-bit max value
        (self as f64) / max_24bit_as_f64
    }
}

impl Normalizable for f32 {
    fn normalize(self) -> f64 {
        self as f64 // f32の場合は既に-1.0から1.0の範囲に正規化されていると仮定
    }
}

pub fn calculate_rms_db<T: Normalizable + Debug + Copy>(block: &[T]) -> f64 {
    let sum_squares: f64 = block
    .iter()
    .map(|&s| {
        let normalized_sample = s.normalize(); // Normalizable トレイトを使用
        normalized_sample.powi(2)
    })
    .sum();

let rms = (sum_squares / block.len() as f64).sqrt();
20.0 * rms.log10()
}

pub fn save_part<T: Sample + Copy>(samples: &[T], spec: &WavSpec, outpath:String) -> Result<(), Error> {
    
    let mut writer =
    WavWriter::create(outpath, *spec).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    
    for &sample in samples {
        // Dereference sample here with &sample which gives us T from &T
        writer
        .write_sample(sample)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
}

writer
.finalize()
.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
Ok(())
}

pub fn setup_progress_bar(total_samples: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_samples);
    let style = ProgressStyle::default_bar()
        .template("{spinner:.green} {elapsed_precise} {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
        .expect("Invalid progress bar template");  // ここでエラーチェック
    pb.set_style(style.progress_chars("##-"));  // スタイルを設定

    pb
}