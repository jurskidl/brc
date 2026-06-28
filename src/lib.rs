#![feature(portable_simd)]
use memmap2::MmapOptions;
use std::{
    collections::HashMap,
    fs::File,
    sync::Arc,
    simd::{
        prelude::SimdPartialEq,
        Simd},
    thread};

struct Records {
    count: u32,
    sum: i32,
    min: i16,
    max: i16,
}

impl Records {
    fn update(&mut self, temp: i16) {
        self.count += 1;
        self.sum += temp as i32;
        if temp < self.min {
            self.min = temp;
        }
        if temp > self.max {
            self.max = temp;
        }
    }

    fn new(temp: i16) -> Self {
        Self {
            count: 1,
            min: temp,
            max: temp,
            sum: temp as i32,
        }
    }

    fn merge(&mut self, other: Records) {
        self.count += other.count;
        self.sum += other.sum;
        self.min = self.min.min(other.min);
        self.max = self.max.max(other.max);
    }

    fn mean(&self) -> f32 {
        self.sum as f32 / (10.0 * self.count as f32)
    }

    fn min(&self) -> f32 {
        self.min as f32 / 10.0
    }

    fn max(&self) -> f32 {
        self.max as f32 / 10.0
    }
}


const CHUNK_SIZE: usize = 256 * 1024; // assume 256KB
const CHUNK_OVERLAP: usize = 128;

#[inline(always)]
fn find_newline(buffer: &[u8]) -> usize {
    let newline = Simd::<u8, 64>::splat(b'\n');

    (0..buffer.len().min(CHUNK_OVERLAP))
        .step_by(64)
        .filter(|x| x + 64 <= buffer.len())
        .map(|x| {
            let chunk = Simd::<u8, 64>::from_slice(&buffer[x..x + 64]);
            chunk.simd_eq(newline).first_set().map(|pos| x + pos)
        })
        .find_map(|x| x)
        .unwrap_or(0)
}

#[inline(always)]
fn parse_value(buffer: &[u8]) -> i16 {
    match buffer.len() {
        // X.X
        3 => ((buffer[0] - b'0') as i16 * 10) + (buffer[2] - b'0') as i16,
        // XX.X or -X.X
        4 => {
            if buffer[0] != b'-' {
                ((buffer[0] - b'0') as i16 * 100)
                + ((buffer[1] - b'0') as i16 * 10)
                + (buffer[3] - b'0') as i16
            } else {
                -(((buffer[1] - b'0') as i16 * 10) + (buffer[3] - b'0') as i16)
            }
        }
        // -XX.X
        5 => {
            -(((buffer[1] - b'0') as i16 * 100)
                + ((buffer[2] - b'0') as i16 * 10)
                + (buffer[4] - b'0') as i16)
        }
        _ => 0,
    }
}

fn parse_chunk(buffer: &[u8]) -> HashMap<&[u8], Records> {
    let mut hash: HashMap<&[u8], Records> = HashMap::with_capacity(10_000);

    for line in buffer[0..].split(|&b| b == b'\n') {
        if line.is_empty() {
            continue;
        }
        if let Some(pos) = line.iter().position(|&b| b == b';') {
            let key = &line[..pos];
            let value = parse_value(&line[pos + 1..]);
            match hash.entry(key) {
                std::collections::hash_map::Entry::Occupied(mut e) => e.get_mut().update(value),
                std::collections::hash_map::Entry::Vacant(e) => {
                    e.insert(Records::new(value));
                }
            }
        }
    }

    hash
}

fn process_file(filename: &str) -> std::io::Result<HashMap<String, Records>> {
    let num_threads = thread::available_parallelism().unwrap().get();
    let file = File::open(filename).expect("Unable to read the file");
    let mapped_file = Arc::new(unsafe { MmapOptions::new().map(&file).unwrap() });
    let file_len = mapped_file.len();
    let step = CHUNK_SIZE - CHUNK_OVERLAP;

    let work_queue = Arc::new(
        (0..file_len)
            .step_by(step)
            .map(|start| (start, (start + CHUNK_SIZE).min(file_len)))
            .collect::<Vec<(usize, usize)>>(),
    );

    let work_split = (work_queue.len() + num_threads - 1) / num_threads;

    let handles: Vec<_> = work_queue
        .chunks(work_split)
        .map(|work| {
            let mmap = Arc::clone(&mapped_file);
            let work = work.to_vec();
            thread::spawn(move || {
                let mut local_hash: HashMap<String, Records> = HashMap::with_capacity(10_000);
                for &(start, end) in &work {
                    let buffer = &mmap[start..end];
                    let chunk_start = find_newline(&buffer[0..CHUNK_OVERLAP.min(buffer.len())]);
                    let tail_start = buffer.len().saturating_sub(CHUNK_OVERLAP);
                    let chunk_end = tail_start + find_newline(&buffer[tail_start..]);
                    if chunk_end > chunk_start {
                        let chunk_hash = parse_chunk(&buffer[chunk_start..chunk_end]);
                        for (key, record) in chunk_hash {
                            let key = String::from_utf8_lossy(key).trim().to_string();
                            match local_hash.entry(key) {
                                std::collections::hash_map::Entry::Occupied(mut entry) => entry.get_mut().merge(record),
                                std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(record); }
                            }
                        }
                    }
                }
                local_hash
            })
        })
        .collect();

    let mut global_hash: HashMap<String, Records> = HashMap::with_capacity(10_000);
    for handle in handles {
        for (key, record) in handle.join().unwrap() {
            match global_hash.entry(key) {
                std::collections::hash_map::Entry::Occupied(mut entry) => entry.get_mut().merge(record),
                std::collections::hash_map::Entry::Vacant(entry) => { entry.insert(record); }
            }
        }
    }
    
    Ok(global_hash)
}

fn print_results(entries: Vec<(String, Records)>) -> () {
    entries
        .iter()
        .for_each(|(a, b)| println!("{} = {:.1}, {:.1}, {:.1}", a, b.min(), b.mean(), b.max()));
}

pub fn brc(file: &str) -> () {
    let mut entries = process_file(file)
        .expect("Failed to process the file")
        .into_iter()
        .collect::<Vec<(String, Records)>>();
    entries.sort_by(|a, b| a.0.cmp(&b.0));

    // print_results(entries)
}