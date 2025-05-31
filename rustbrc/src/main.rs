use std::fs::File;
use std::io::Read;
use std::env;
use threadpool::ThreadPool;

fn parse_row(chunk: Vec<u8>) {
    println!("Processing chunk of size: {}", chunk.len());
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

    let mut file = File::open(args.get(1)
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

        pool.execute(move || {
            parse_row(buffer); // move buffer directly
        });
    }

    pool.join(); // Wait for all threads
    Ok(())
}
