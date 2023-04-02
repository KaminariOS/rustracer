use std::path::PathBuf;
use std::{fs, path::Path};
use std::fs::DirEntry;
use anyhow::{Result};

const SPV_SEARCH_PATHS: [&str; 2] = ["", "./spv"];

const MODEL_SEARCH_PATHS: [&str; 4] = [
    "",
    "./assets/models",
    "/home/kosumi/Rusty/glTF-Sample-Models/2.0",
    "../../../assets/models",
];

const SKYBOX_SEARCH_PATHS: [&str; 3] = [
    "",
    "./assets/skyboxs",
    "../../../assets/skyboxs",
];

pub fn load_spv<P: AsRef<Path>>(path: P) -> Vec<u8> {

    let mut res = None;

    for pre in SPV_SEARCH_PATHS {
        let search = Path::new(pre).join(&path);
        if let Ok(bytes) = fs::read(&search) {
            res = Some(bytes);
            break;
        }
    }
    res.expect(&*format!(
        "Couldn't find spv file {}, current path: {}",
        path.as_ref().display(),
        Path::new(".").canonicalize().unwrap().display()
    ))
}

pub fn load_model<P: AsRef<Path>>(path: P) -> PathBuf {
    let path_ref = path.as_ref();
    let mut res = None;

    for pre in MODEL_SEARCH_PATHS {
        let search = Path::new(pre).join(&path);
        if search.exists() {
            res = Some(search);
            break;
        }
    }
    res.expect(&*format!(
        "Couldn't find model file {}, current path: {}",
        path_ref.display(),
        Path::new(".").canonicalize().unwrap().display()
    ))
}


pub fn load_cubemap<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>> {
    let test_fun = |p: &Path| p.exists() && p.is_dir();
    let mut abs_path = PathBuf::new();
    for pre in SKYBOX_SEARCH_PATHS {
        let search = Path::new(pre).join(&path);
        if test_fun(&search) {
            abs_path = search;
            break
        }
    };
    let res: Vec<_> = fs::read_dir(abs_path)?.filter_map(|f| f.ok())
            .filter(|dir| {
                let filename = dir.file_name();
                filename.to_str().filter(|s| s.ends_with(".png") || s.ends_with(".jpg")).is_some()
            })
            .collect();
    assert_eq!(res.len(), 6);
    return Ok(res)
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
