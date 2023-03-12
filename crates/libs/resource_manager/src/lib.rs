use std::{fs, path::Path};
use std::path::PathBuf;

const SPV_SEARCH_PATHS: [&str; 1] = [
    "./spv"
];

const MODEL_SEARCH_PATHS: [&str; 2] = [
    "./assets/models",
    "/home/kosumi/Rusty/glTF-Sample-Models/2.0"
];

pub fn load_spv<P: AsRef<Path>>(path: P) -> Vec<u8> {
    if let Ok(bytes) = fs::read(&path) {
        bytes
        } else {
        let mut res = None;

        for pre in SPV_SEARCH_PATHS {
            let search = Path::new(pre).join(&path);
            if let Ok(bytes) = fs::read(&search) {
                res = Some(bytes);
                break
            }
        }
        res.expect(&*format!("Couldn't find spv file {}, current path: {}",
                             path.as_ref().display(),
                              Path::new(".").canonicalize().unwrap().display()))
    }
}


pub fn load_model<P: AsRef<Path>>(path: P) -> PathBuf {
    if path.as_ref().exists() {
        let mut p = PathBuf::new();
        p.push(path.as_ref());
        p
    } else {
        let mut res = None;

        for pre in MODEL_SEARCH_PATHS {
            let search = Path::new(pre).join(&path);
            if search.exists() {
                res = Some(search);
                break
            }
        }
        res.expect(&*format!("Couldn't find model file {}, current path: {}",
                             path.as_ref().display(),
                             Path::new(".").canonicalize().unwrap().display()))
    }
}

// pub fn select_gltf<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
//     use native_dialog::FileDialog;
//     FileDialog::new()
//         .add_filter("gltf", &["gltf", "glb"])
//         .set_location(&path)
//         .show_open_single_file().unwrap()
// }
//
#[test]
fn test_load_spv() {
    load_spv("./src/main.rs");
}