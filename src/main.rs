use hound::{Sample, SampleFormat, WavReader, WavSpec};
use resampler::resample_wav;
use utils::{Normalizable, calculate_rms_db, save_part,setup_progress_bar};
mod resampler;
mod utils;
use std::any::type_name;
use std::fmt::Debug;
use std::fs::{self, File};
use std::path::{PathBuf, Path};
use std::io::{self, BufReader, Error, Write};
use structopt::StructOpt;


#[derive(StructOpt, Debug)]
#[structopt(name = "audio_splitter")]
struct Opt {
    /// input file path. Itmast be wav file.
    #[structopt(short="i",long="input", default_value = "input.wav")]
    input_filepath: PathBuf,

    /// output directory path
    #[structopt(short="o",long="output", default_value = ".")]
    output_dirpath: PathBuf,

    /// Minimum silence duration in milliseconds
    #[structopt(long="min-silence-duration", default_value = "500")]
    min_silence_duration: u64,

    #[structopt(long="min-sound-duration", default_value = "500")]
    min_sound_duration: u64,

    /// Chunk size in milliseconds
    #[structopt(short="c",long="chunk_length", default_value = "20")]
    chunk_length: u64,

    /// Threshold in dB
    #[structopt(short="t",long="threshold", default_value = "-80",parse(try_from_str))]
    threshold: i32,
}


fn process_samples<T: Normalizable + Sample + Debug + Copy>(
    reader: &mut WavReader<BufReader<File>>,
    spec: &WavSpec,
    opt: &Opt,
) -> Result<(), Error> {
    // ログファイルを開きます
    let mut log_file = File::create("process_log.txt").expect("Failed to create log file");
    let mut chunk_size = ((spec.sample_rate / 1000) as u64 * opt.chunk_length) as usize;
    
    writeln!(
        log_file,
        "min_silence_duration: {} ms",
        opt.min_silence_duration
    );
    writeln!(log_file, "chunk_length: {} ms", opt.chunk_length);
    writeln!(log_file, "threshold: {} dB", opt.threshold);
    
    writeln!(log_file, "sample type: {}", type_name::<T>());
    
    writeln!(log_file, "Chunk Size: {}", chunk_size).expect("Failed to write to log file");

    // specの詳細をログファイルに書き込みます
    writeln!(log_file, "WavSpec Details: Channels = {}, \nSample Rate = {}, \nBits per Sample = {}, \nSample Format = {:?}",
    spec.channels, spec.sample_rate, spec.bits_per_sample, spec.sample_format).expect("Failed to write WavSpec to log file");
    
    let mut chunk_buffer = Vec::with_capacity(chunk_size);
    let mut audio_buffer = Vec::new();
    let mut part_index = 0;
    let mut silence_duration = 0;
    let mut sound_duration = 0;
    let mut keep_silence: bool = true;
    let mut num_samples = 0;

    let mut outdir_16kHz = opt.output_dirpath.clone();
    outdir_16kHz.push("16kHz");
    let mut outdir_original = opt.output_dirpath.clone();
    outdir_original.push("original");

    if !outdir_16kHz.exists() {
        fs::create_dir_all(&outdir_16kHz)?;
    }
    if !outdir_original.exists() {
        fs::create_dir_all(&outdir_original)?;
    }

    writeln!(log_file, "make tmp file");

    // プログレスバーを表示
    let total_samples = reader.len();  // 総サンプル数を取得
    let pb = setup_progress_bar(total_samples as u64);
    
    for sample_result in reader.samples::<T>() {
        let sample = sample_result.map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        
        // audio_bufferにすべてのサンプルを保存します
        audio_buffer.push(sample);
        
        // chunk_bufferにサンプルを追加します
        chunk_buffer.push(sample);
        
        // chunk_bufferがchunk_sizeサンプルに達したら処理を行います
        if chunk_buffer.len() == chunk_size {
            // calculate_rms_db関数を呼び出して結果をresultに格納します
            let result = calculate_rms_db(&chunk_buffer);
            writeln!(log_file, "Chunk RMS dB: {}", result).expect("Failed to write to log file");
            
            if result < -40.0 {
                silence_duration += opt.chunk_length;
                if silence_duration > opt.min_silence_duration {
                    if !keep_silence {
                        let outpath_original = format!("{}/output_{}.wav",outdir_original.to_string_lossy(),part_index);
                        let outpath_16kHz = format!("{}/output_{}.wav",outdir_16kHz.to_string_lossy(),part_index);
                        if let Err(e) = save_part(&audio_buffer, &spec, outpath_original.clone()) {
                            eprintln!("Error saving WAV part: {}", e);
                        }
                        // 16kHzにリサンプリングしたものを出力
                        let _ = resample_wav(Path::new(&outpath_original), Path::new(&outpath_16kHz), 16000.0);

                        pb.inc(audio_buffer.len() as u64);  // プログレスバーをchunk_size分だけ進める

                        part_index += 1;
                        audio_buffer.clear();
                    }
                    keep_silence = true;
                }
            } else {
                sound_duration += opt.chunk_length;
                if sound_duration > opt.min_sound_duration {
                    keep_silence = false;
                    silence_duration = 0;
                }
            }
            
            // chunk_bufferをクリアして次のバッチの準備をします
            chunk_buffer.clear();
        }
    }
    
    writeln!(log_file, "num_samples: {}", num_samples).expect("Failed to write to log file");
    
    // 最後のバッチを処理します（chunk_bufferに残っているデータが512未満の場合）
    if !audio_buffer.is_empty() {
        let outpath_original = format!("{}/output_{}.wav",outdir_original.to_string_lossy(),part_index);
        let outpath_16kHz = format!("{}/output_{}.wav",outdir_16kHz.to_string_lossy(),part_index);
        if let Err(e) = save_part(&audio_buffer, &spec, outpath_original.clone()) {
            eprintln!("Error saving WAV part: {}", e);
        }
        // 16kHzにリサンプリングしたものを出力
        let _ = resample_wav(Path::new(&outpath_original), Path::new(&outpath_16kHz), 16000.0);
    }
    
    Ok(())
}





fn main() -> Result<(), Error> {
    // option引数を取得
    let opt = Opt::from_args();

    let input_filepath: PathBuf = opt.input_filepath.clone();
    
    let reader_result = WavReader::open(input_filepath);
    let mut reader = match reader_result {
        Ok(reader) => reader,
        Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
    };
    let spec = reader.spec();

    match spec.sample_format {
        SampleFormat::Float => process_samples::<f32>(&mut reader, &spec, &opt),
        SampleFormat::Int => {
            match spec.bits_per_sample {
                16 => process_samples::<i16>(&mut reader, &spec, &opt),
                24 => process_samples::<i32>(&mut reader, &spec, &opt), // Handling 24-bit as i32
                _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported bit depth")),
            }
        }
        _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "Unsupported sample format")),
    }
}