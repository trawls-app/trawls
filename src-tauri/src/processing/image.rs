use std::path::Path;
use std::unimplemented;
use std::fs::File;
use std::io::BufReader;
use std::cmp::max;
use std::sync::{Arc, Mutex};


pub struct Image {
    pub raw_image_data: Vec<u16>,
    pub height: usize,
    pub width: usize,
}


impl Image {
    pub fn load_from_raw(path: &Path, intensity: f32) -> Result<Image, &str> {
        let image = rawloader::decode_file(path).unwrap();

        if let rawloader::RawImageData::Integer(data) = image.data {
            assert_eq!(data.len(), image.width * image.height, "Mismatch between raw data-size and image resolution.");
            Ok(Image {
                raw_image_data: data.iter().map(|x| (*x as f32 * intensity) as u16).collect(),
                height: image.height,
                width: image.width,
            })
        } else {
            unimplemented!("Can't parse RAWs with non-integer data, yet.");
        }
    }

    pub fn merge(&self, other: Image) -> Image {
        let res = self.raw_image_data.iter()
            .zip(other.raw_image_data)
            .map(|(x, y)| max(*x, y))
            .collect();

        Image {
            raw_image_data: res,
            height: self.height,
            width: self.width,
        }
    }
}