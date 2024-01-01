// The identification system will get very bulky since it staticly links every function so it gets
// its own file
use orthrus_core::prelude::*;
use orthrus_ncompress::prelude::*;

static SHALLOW_SCAN: [IdentifyFn; 2] = [yay0::Yay0::identify, yaz0::Yaz0::identify];

static DEEP_SCAN: [IdentifyFn; 2] = [yay0::Yay0::identify_deep, yaz0::Yaz0::identify_deep];

pub(crate) fn identify_file(input: String, deep_scan: bool) {
    let data = std::fs::read(&input).expect("Unable to open file for identification!");

    let mut identified_types: Vec<FileInfo> = vec![];
    let scan_list = if deep_scan { &DEEP_SCAN } else { &SHALLOW_SCAN };

    for identifier in scan_list {
        if let Some(identity) = identifier(&data) {
            identified_types.push(identity);
        }
    }

    match identified_types.len() {
        0 => println!("{input}: data"),
        1 => {
            println!("{input}: {}", identified_types[0].info);
            if let Some(payload) = identified_types[0].payload.as_ref() {
                identify_deep(payload, 1);
            }
        }
        _ => {
            println!("{input}: Multiple possible filetypes identified:");
            for info in identified_types {
                println!("- {}", info.info);
                if let Some(payload) = info.payload.as_ref() {
                    identify_deep(payload, 1);
                }
            }
        }
    }
}

fn identify_deep(data: &Box<[u8]>, indent: usize) {
    let mut identified_types: Vec<FileInfo> = vec![];

    for identifier in DEEP_SCAN {
        if let Some(identity) = identifier(&data) {
            identified_types.push(identity);
        }
    }

    let indentation = "    ".repeat(indent);

    match identified_types.len() {
        0 => println!("{}- data", indentation),
        1 => {
            println!("{}- {}", indentation, identified_types[0].info);
            if let Some(payload) = identified_types[0].payload.as_ref() {
                identify_deep(payload, indent + 1);
            }
        }
        _ => {
            println!("{}- Multiple possible filetypes identified:", indentation);
            for info in identified_types {
                println!("- {}", info.info);
                if let Some(payload) = info.payload.as_ref() {
                    identify_deep(payload, indent + 1);
                }
            }
        }
    }
}
