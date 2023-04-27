use anyhow::Result;
use std::fs::DirEntry;
use std::path::PathBuf;
use std::{fs, path::Path};

const SPV_SEARCH_PATHS: [&str; 2] = ["", "./spv"];

const MODEL_SEARCH_PATHS: [&str; 5] = [
    "",
    "./assets/models",
    "/home/kosumi/Rusty/glTF-Sample-Models/2.0",
    "../../../assets/models",
    "/home/kosumi/Rusty/LGL-Tracer-Renderer.github.io/models",
];

const SKYBOX_SEARCH_PATHS: [&str; 3] = ["", "./assets/skyboxs", "../../../assets/skyboxs"];

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

fn find_gltf(search: &PathBuf) -> Option<PathBuf> {
    for entry in fs::read_dir(search).ok()?.filter_map(|e| e.ok()) {
        if entry
            .file_name()
            .to_str()
            .filter(|name| name.ends_with(".gltf") || name.ends_with(".glb"))
            .is_some()
        {
            return Some(entry.path());
        }
    }
    None
}

pub fn load_model<P: AsRef<Path>>(path: P) -> Result<PathBuf> {
    let path_ref = path.as_ref();
    let mut res = None;
    let tails = ["", "glTF"];
    for pre in MODEL_SEARCH_PATHS {
        let search = Path::new(pre).join(&path);
        for tail in tails {
            let mut path = search.clone();
            if !tail.is_empty() {
                path = path.join(tail);
            }
            if path.exists() && path.is_file() {
                res = Some(path);
            } else if let Some(p) = find_gltf(&path) {
                res = Some(p);
            }
            if res.is_some() {
                break;
            }
        }
    }
    Ok(res.unwrap_or_default())
    // res.expect(&*format!(
    //     "Couldn't find model file {}, current path: {}",
    //     path_ref.display(),
    //     Path::new(".").canonicalize().unwrap().display()
    // )
}

pub fn load_cubemap<P: AsRef<Path>>(path: P) -> Result<Vec<DirEntry>> {
    let test_fun = |p: &Path| p.exists() && p.is_dir();
    let mut abs_path = PathBuf::new();
    for pre in SKYBOX_SEARCH_PATHS {
        let search = Path::new(pre).join(&path);
        if test_fun(&search) {
            abs_path = search;
            break;
        }
    }
    let res: Vec<_> = fs::read_dir(abs_path)?
        .filter_map(|f| f.ok())
        .filter(|dir| {
            let filename = dir.file_name();
            filename
                .to_str()
                .filter(|s| s.ends_with(".png") || s.ends_with(".jpg"))
                .is_some()
        })
        .collect();
    assert_eq!(res.len(), 6);
    return Ok(res);
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
