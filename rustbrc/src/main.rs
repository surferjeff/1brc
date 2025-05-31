use std::fs::File;
use std::io::Read;
use std::env;
use threadpool::ThreadPool;
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
    let chunk_mb: usize = args.get(1)
        .expect("Missing arg1: size of chunk in MB")
        .parse()
        .expect("Expected arg1 to be an integer.");
    let chunk_size = chunk_mb * 1024 * 1024;

    let mut file = File::open(args.get(2)
        .expect("Missing arg2: /path/to/measurements.txt"))?;

    let num_threads = num_cpus::get();
    let pool = ThreadPool::new(num_threads);

    let mut next_buffer = Vec::<u8>::new();
    loop {
        let mut buffer = next_buffer;
        buffer.resize(chunk_size, 0);
        let bytes_read = file.read(&mut buffer)?;

        if bytes_read == 0 {
            break;
        }

        buffer.truncate(bytes_read); // Trim to actual size read

        // Find the last newline
        let last_lf = buffer.iter().rev().position(|c| *c == b'\n').expect(
            "failed to find line feed character in buffer.");
        next_buffer = buffer[buffer.len()-last_lf..].to_vec();
        buffer.truncate(buffer.len() - last_lf);

        pool.execute(move || {
            parse_rows(buffer); // move buffer directly
        });
    }

    pool.join(); // Wait for all threads
    Ok(())
}
