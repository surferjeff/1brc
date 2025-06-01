use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::env;
use std::os::unix::fs::MetadataExt;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use rayon::prelude;
use std::collections::HashMap;

struct Measurements {
    pub min: f32,
    pub max: f32,
    pub sum: f32,
    pub count: u32
}

// pub fn combine(m1: &Measurements, m2: &measurements) {

// }

type MeasureMap = HashMap<Vec<u8>, Measurements>;

fn parse_rows(chunk: Vec<u8>) -> MeasureMap {
    let mut result = MeasureMap::new();
    let mut i = 0usize;
    while parse_row(&chunk, &mut i, &mut result) {}
    result
}

fn parse_row(buffer: &[u8], i: &mut usize, result: &mut MeasureMap) -> bool {
    if *i >= buffer.len() { return false; }

    // Parse the name.
    let name_start = *i;
    while *i < buffer.len() && buffer[*i] != b';' {
        *i += 1;
    }
    let name = &buffer[name_start..*i];
    *i += 1;  // Advance past ;

    // Parse the measurement
    let measurement_start = *i;
    while *i < buffer.len() && buffer[*i] != b'\n' {
        *i += 1;
    }
    let n: f32 = std::str::from_utf8(&buffer[measurement_start..*i])
        .expect("Found non-utf8 measurement")
        .parse()
        .expect("Measurement couldn't be parsed as f32.");
    *i += 1;  // Advance past \n

    // Update the result.
    match result.get_mut(name) {
        Some(kv) => {
            kv.max = kv.max.max(n);
            kv.min = kv.min.min(n);
            kv.count += 1;
            kv.sum += n;
        },
        None => {
            let _ = result.insert(name.to_vec(), Measurements {
                min: n,
                max: n,
                sum: n,
                count: 1
            });
        }
    }
    
    true
}

fn main() -> std::io::Result<()> {
    // Usage:
    //   rustbrc <size of read chunk in MB> /path/to/measurements.txt
    let args: Vec<String> = env::args().collect();
    
    let measurements_path = args.get(1)
        .expect("Missing arg1: /path/to/measurements.txt");
    let metadata = std::fs::metadata(measurements_path)
        .expect("Failed to get metadata");
    let filesize = metadata.size();
    let chunk_count = num_cpus::get() as u64;
    let chunk_size = (filesize + chunk_count) / chunk_count;
    let chunks: Vec<u64> = (0..chunk_count).map(|n| n * chunk_size).collect();

    let _bytes_read: Vec<_> = chunks.par_iter().map(|offset| {
        let mut f = OpenOptions::new().read(true).open(measurements_path)
            .expect("Failed to open file.");
        f.seek(SeekFrom::Start(*offset)).expect("Failed to seek in file.");
        let mut buffer = vec![0u8; chunk_size as usize];
        let bytes_read = f.read(&mut buffer).expect("Failed to read file.");
        buffer.truncate(bytes_read);
        bytes_read
    }).collect();

    Ok(())
}
