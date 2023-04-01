use std::collections::HashSet;
use crate::error::*;
use crate::Name;
use gltf::image::{Format, Source};

#[derive(Debug, Clone)]
pub struct Image {
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub source: Name,
    pub index: usize,
    pub gamma: TexGamma
}


#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum TexGamma {
    Linear,
    Srgb,
}

impl Default for Image {
    fn default() -> Self {
        Self {
            pixels: vec![1; 4],
            width: 1,
            height: 1,
            source: None,
            index: 0,
            gamma: TexGamma::Srgb,
        }
    }
}

impl Image {
    pub fn update_info(&mut self, info: gltf::Image, linear: &HashSet<usize>) {
        self.index = info.index() + 1;
        if linear.contains(&self.index) {
            self.gamma = TexGamma::Linear;
        }
        self.source = match info.source() {
            Source::View { .. } => None,
            Source::Uri { uri, .. } => Some(uri.to_string()),
        };
    }
}

impl TryFrom<&gltf::image::Data> for Image {
    type Error = Error;

    fn try_from(image: &gltf::image::Data) -> Result<Self> {
        let width = image.width;
        let height = image.height;

        let pixels = PixelIter::new(image)?.flatten().collect::<Vec<_>>();

        Ok(Self {
            pixels,
            width,
            height,
            ..Default::default()
        })
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PixelIter<'a> {
    pixels: &'a [u8],
    format: Format,
    pixel_size: usize,
    position: usize,
}

impl<'a> PixelIter<'a> {
    pub(crate) fn new(image: &'a gltf::image::Data) -> Result<Self> {
        use Format::*;
        let pixels = &image.pixels;
        let format = image.format;
        let pixel_size = match format {
            R8 => 1,
            R8G8 => 2,
            R8G8B8 => 3,
            R8G8B8A8 => 4,
            _ => return Err(Error::Support("16 bytes images".to_string())),
        };

        Ok(Self {
            pixels,
            format,
            pixel_size,
            position: 0,
        })
    }
}

impl<'a> Iterator for PixelIter<'a> {
    type Item = [u8; 4];

    fn next(&mut self) -> Option<Self::Item> {
        if self.position == self.pixels.len() {
            return None;
        }

        let pixel = match self.format {
            Format::R8 => {
                let r = self.pixels[self.position];
                Some([r, 0, 0, u8::MAX])
            }
            Format::R8G8 => {
                let r = self.pixels[self.position];
                let g = self.pixels[self.position + 1];
                Some([r, g, 0, u8::MAX])
            }
            Format::R8G8B8 => {
                let r = self.pixels[self.position];
                let g = self.pixels[self.position + 1];
                let b = self.pixels[self.position + 2];
                Some([r, g, b, u8::MAX])
            }
            Format::R8G8B8A8 => {
                let r = self.pixels[self.position];
                let g = self.pixels[self.position + 1];
                let b = self.pixels[self.position + 2];
                let a = self.pixels[self.position + 3];
                Some([r, g, b, a])
            }
            _ => unreachable!("Self::new already checks"),
        };

        self.position += self.pixel_size;

        pixel
    }
}
