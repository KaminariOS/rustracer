use std::path::Path;
use crate::image::{Image, TexGamma};
use std::str::FromStr;
use strum_macros::{EnumCount, EnumString};
use strum::{EnumCount};
use crate::texture::Sampler;
use anyhow::Result;

pub struct SkyBox {
    pub images : Vec<Image>,
    pub sampler: Sampler,
    pub collector: Vec<u8>
}
#[derive(Debug, PartialEq, EnumString, EnumCount)]
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
        let name = name.split(".").next().unwrap();
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
                 _ => {unimplemented!()}
             }
        };
        i as _
    }
}

impl SkyBox {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut dir_entry = resource_manager::load_cubemap(path).unwrap();
        dir_entry.sort_by_key(|d| Face::get_index(d.file_name().to_str().unwrap()));
        let mut collector: Vec<u8> = Vec::with_capacity(Face::COUNT * 4 * 2048 * 2048);
        let images = dir_entry.into_iter()
            .map(|d|
                Image::load_image(d.path(), Some(&mut collector))
            ).collect::<Result<Vec<_>>>()?;
       Ok(Self {
           images,
           sampler: Default::default(),
           collector
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