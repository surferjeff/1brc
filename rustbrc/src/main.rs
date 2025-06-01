use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom};
use std::env;
use std::os::unix::fs::MetadataExt;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

struct Measurements {
    pub min: f32,
    pub max: f32,
    pub sum: f32,
    pub count: u32
}

type MeasureMap = fxhash::FxHashMap<Vec<u8>, Measurements>;

fn parse_rows(chunk: &[u8]) -> MeasureMap {
    let mut result = MeasureMap::with_hasher(fxhash::FxBuildHasher::default());
    let mut i = 0usize;
    while parse_row(chunk, &mut i, &mut result) {}
    result
}

fn parse_row(buffer: &[u8], i: &mut usize, result: &mut MeasureMap) -> bool {
    if *i >= buffer.len() { return false; }

    // Parse the name.
    let Some(pos) = buffer.iter().skip(*i).position(|c| *c == b';') else {
        return false;  // Incomplete line.
    };
    let name = &buffer[*i..*i+pos];
    *i += pos + 1;

    // Parse the measurement
    let Some(pos) = buffer.iter().skip(*i).position(|c| *c == b'\n') else {
        return false;  // Incomplete line.
    };
    let n: f32 = std::str::from_utf8(&buffer[*i..*i+pos])
        .expect("Found non-utf8 measurement")
        .parse()
        .expect("Measurement couldn't be parsed as f32.");
    *i += pos + 1;

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
        let mut buffer = Vec::<u8>::new();
        if *offset > 0 {
            // Read 100 bytes before the start of the chunk to get the 
            // line spanning the chunk boundary.
            f.seek(SeekFrom::Start(*offset - 100)).expect("Failed to seek in file.");
            buffer.resize(chunk_size as usize + 100, 0);
            let bytes_read = f.read(&mut buffer).expect("Failed to read file.");
            buffer.truncate(bytes_read);
            // Find the boundary between lines.
            let pos = buffer.iter().skip(100).rev().position(|c| *c == b'\n')
                .expect("Failed to find line spanning chunks.");
            parse_rows(&buffer[100-pos..])
        } else {
            buffer.resize(chunk_size as usize, 0);
            let bytes_read = f.read(&mut buffer).expect("Failed to read file.");
            buffer.truncate(bytes_read);
            parse_rows(&buffer)
        }
    }).collect();

    Ok(())
}
