// Copyright (C) 2021 Bosutech XXI S.L.
//
// nucliadb is offered under the AGPL v3.0 and as commercial software.
// For commercial licensing, contact us at info@nuclia.com.
//
// AGPL:
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <http://www.gnu.org/licenses/>.
use std::fs::File;
use std::{fs, io, process};

use clap::Parser;
#[allow(clippy::all)]
use rstack;
use rustc_demangle::demangle;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct LineInfo {
    file: String,
    line: u64,
    column: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct FrameInfo {
    ip: String,
    symbol_name: String,
    symbol_offset: String,
    line_info: Option<LineInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ThreadInfo {
    thread_id: u32,
    thread_name: String,
    frames: Vec<FrameInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
struct ProcessInfo {
    threads: Vec<ThreadInfo>,
}

fn get_binary_path(pid: u32) -> io::Result<String> {
    let exe_path = format!("/proc/{}/exe", pid);
    let path = fs::read_link(exe_path)?;
    Ok(path.to_string_lossy().to_string())
}

#[derive(Parser, Debug)]
#[clap(author="Nuclia", version, about="Dumps your stacks")]
pub struct Args {
    /// PID
    #[clap(short, long, default_value_t = 1)]
    pid: u32,

    /// Filter by thread name
    #[clap(short, long, default_value_t = String::from(""))]
    thread_name: String,

    /// Output
    #[clap(short, long, value_enum, default_value_t = Output::Plain)]
    output: Output,
}

#[derive(clap::ValueEnum, Clone, Debug, PartialEq)]
enum Output {
    Plain,
    Json,
}

impl Default for Args {
    fn default() -> Self {
        Args::new()
    }
}

impl Args {
    pub fn new() -> Args {
        Args::parse()
    }
}

fn main() {
    let args = Args::new();

    // Get the binary file
    let binary_path = get_binary_path(args.pid).unwrap();
    let binary = File::open(binary_path).unwrap();

    let map = unsafe { memmap2::Mmap::map(&binary).unwrap() };
    let file = addr2line::object::File::parse(&*map).unwrap();
    let map = addr2line::ObjectContext::new(&file).expect("debug symbols not found");

    // Get trace
    let process = match rstack::trace(args.pid) {
        Ok(threads) => threads,
        Err(e) => {
            eprintln!("error tracing threads: {}", e);
            process::exit(1);
        }
    };

    // Create ProcessInfo struct to store the JSON data
    let mut process_info = ProcessInfo { threads: vec![] };

    for thread in process.threads() {
        let thread_name = thread.name().unwrap_or("<unknown>").to_string();

        if !args.thread_name.is_empty() && thread_name != args.thread_name {
                continue;
        }


        let mut thread_info = ThreadInfo {
            thread_id: thread.id(),
            thread_name,
            frames: vec![],
        };

        for frame in thread.frames() {
            let mut frame_info = FrameInfo {
                ip: format!("{:#016x}", frame.ip()),
                symbol_name: String::new(),
                symbol_offset: String::new(),
                line_info: None,
            };

            match frame.symbol() {
                Some(symbol) => {
                    frame_info.symbol_name = demangle(symbol.name()).to_string();
                    frame_info.symbol_offset = format!("{:#x}", symbol.offset());

                    // Convert the offset to file, line, col
                    let loc = map.find_location(symbol.offset()).unwrap();

                    if let Some(loc) = loc {
                        let line_info = LineInfo {
                            file: loc.file.unwrap_or("???").to_string(),
                            line: loc.line.unwrap_or(0) as u64,
                            column: loc.column.unwrap_or(0) as u64,
                        };
                        frame_info.line_info = Some(line_info);
                    }
                }
                None => {
                    frame_info.symbol_name = "???".to_string();
                    frame_info.symbol_offset = "???".to_string();
                }
            }

            thread_info.frames.push(frame_info);
        }

        process_info.threads.push(thread_info);
    }

    if args.output == Output::Json {
        // Serialize process_info to JSON using serde_json
        let json_output = serde_json::to_string_pretty(&process_info)
            .expect("Failed to serialize process_info to JSON");

        // Print the JSON output
        println!("{}", json_output);
    } else {
        for thread in process_info.threads.iter() {
            if !args.thread_name.is_empty() && thread.thread_name != args.thread_name {
                continue;
            }

            println!("Thread [{}] {}", thread.thread_id, thread.thread_name);
            for (idx, frame) in thread.frames.iter().enumerate() {
                if let Some(line_info) = &frame.line_info {
                    println!(
                        "\t{}: {} ({})\n\tat {}:{}",
                        idx, frame.symbol_name, frame.symbol_offset, line_info.file, line_info.line
                    );
                } else {
                    println!("\t{}: {} ({})", idx, frame.symbol_name, frame.symbol_offset);
                }
            }
        }
    }
}
