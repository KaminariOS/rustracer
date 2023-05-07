use crate::image::{Image, TexGamma};
use std::fs::DirEntry;
use std::path::Path;
use std::str::FromStr;
use std::time::Instant;
use strum_macros::{EnumCount, EnumString};

use crate::texture::Sampler;
use anyhow::Result;
use cfg_if::cfg_if;
use log::info;

pub struct SkyBox {
    pub images: Vec<Image>,
    pub sampler: Sampler,
    pub collector: Vec<u8>,
}
#[derive(Debug, PartialEq, EnumString, EnumCount)]
#[allow(non_camel_case_types)]
enum Face {
    posx = 0,
    negx,
    posy,
    negy,
    posz,
    negz,
}

impl Face {
    fn get_index(name: &str) -> usize {
        let name = name.split('.').next().unwrap();
        let index = Self::from_str(name);
        let i = if let Ok(index) = index {
            index
        } else {
            match name {
                "back" => Self::negz,
                "front" => Self::posz,
                "top" => Self::posy,
                "bottom" => Self::negy,
                "left" => Self::negx,
                "right" => Self::posx,
                _ => {
                    unimplemented!()
                }
            }
        };
        i as _
    }
}

#[cfg(not(feature = "rayon"))]
fn load_skybox(dir_entry: Vec<DirEntry>) -> Result<(Vec<Image>, Vec<u8>)> {
    let images = dir_entry
        .into_iter()
        .map(|d| Image::load_image(d.path()))
        .collect::<Result<Vec<_>>>()?;

    let collector = images
        .iter()
        .map(|i| &i.pixels)
        .flatten()
        .map(|&p| p)
        .collect();
    Ok((images, collector))
}

#[cfg(feature = "rayon")]
fn load_skybox_par(dir_entry: Vec<DirEntry>) -> Result<(Vec<Image>, Vec<u8>)> {
    use rayon::prelude::*;
    let images = dir_entry
        .into_par_iter()
        .map(|d| Image::load_image(d.path()))
        .collect::<Result<Vec<_>>>()?;

    let collector = images
        .par_iter()
        .map(|i| &i.pixels)
        .flatten()
        .map(|&p| p)
        .collect();
    Ok((images, collector))
}

impl SkyBox {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let now = Instant::now();
        let mut dir_entry = resource_manager::load_cubemap(path).unwrap();
        dir_entry.sort_by_key(|d| Face::get_index(d.file_name().to_str().unwrap()));
        // let (images, collector) =
        cfg_if! {
            if #[cfg(feature = "rayon")] {
                let (images, collector) = load_skybox_par(dir_entry)?;
            } else {
                 let (images, collector) = load_skybox(dir_entry)?;
            }
        }

        info!("Finish Skybox processing: {}s", now.elapsed().as_secs());
        Ok(Self {
            images,
            sampler: Default::default(),
            collector,
        })
    }

    pub fn get_total_size(&self) -> usize {
        self.images.iter().map(|i| i.pixels.len()).sum()
    }

    pub fn get_extents(&self) -> [u32; 2] {
        [self.images[0].width, self.images[1].height]
    }

    pub fn get_gamma(&self) -> TexGamma {
        self.images[0].gamma
    }
}

#[test]
fn test() {
    let s = SkyBox::new("LancellottiChapel").unwrap();
    let _k = SkyBox::new("/home/kosumi/Rusty/rustracer/assets/skyboxs/LancellottiChapel").unwrap();
    let images = &s.images;
    assert_eq!(images.len(), 6)
}
