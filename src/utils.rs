use hound::{Sample, WavSpec, WavWriter};
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

impl Normalizable for f32 {
    fn normalize(self) -> f64 {
        self as f64 // f32 の場合は既に -1.0 から 1.0 の範囲に正規化されていると仮定
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