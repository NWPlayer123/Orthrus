// The identification system will get very bulky since it staticly links every function so it gets its own file
use orthrus_core::prelude::*;
use orthrus_ncompress::prelude::*;

static SHALLOW_SCAN: [IdentifyFn; 1] = [yay0::Yay0::identify];

static DEEP_SCAN: [IdentifyFn; 1] = [yay0::Yay0::identify_deep];

pub(crate) fn identify_file(input: String, deep_scan: bool) {
    let data = std::fs::read(&input).expect("Unable to open file for identification!");

    let mut identified_types: Vec<FileInfo> = vec![];
    if deep_scan == false {
        for identifier in SHALLOW_SCAN {
            if let Some(identity) = identifier(&data) {
                identified_types.push(identity);
            }
        }
        if identified_types.len() == 0 {
            println!("{input}: data");
        } else if identified_types.len() == 1 {
            println!("{input}: {}", identified_types[0].info);
        } else {
            println!("{input}: Multiple possible filetypes identified:");
            for info in identified_types {
                println!("- {}", info.info);
            }
        }
    } else {
        for identifier in DEEP_SCAN {
            if let Some(identity) = identifier(&data) {
                identified_types.push(identity);
            }
        }
    }
}
