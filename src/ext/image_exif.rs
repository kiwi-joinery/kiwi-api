use exif::{In, Tag, Value};
use image::{DynamicImage, ImageResult};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::io::Cursor;

#[derive(FromPrimitive, PartialEq, Debug)]
pub enum ExifOrientation {
    Normal = 1,
    FlipHorizontal = 2,
    Rotate180 = 3,
    FlipVertical = 4,
    Transpose = 5,
    Rotate90 = 6,
    Transverse = 7,
    Rotate270 = 8,
}

// Probably need to test that the Transpose and Transverse are actually correct!
pub fn read_image(buffer: &Vec<u8>) -> ImageResult<DynamicImage> {
    let mut img = image::load_from_memory(buffer)?;
    let reader = exif::Reader::new();
    if let Ok(exif) = reader.read_from_container(&mut Cursor::new(buffer)) {
        if let Some(field) = exif.get_field(Tag::Orientation, In::PRIMARY) {
            if let Value::Short(x) = &field.value {
                if let Some(o) = x.get(0).and_then(|x| ExifOrientation::from_u16(*x)) {
                    match o {
                        ExifOrientation::Normal => {}
                        ExifOrientation::FlipHorizontal => {
                            img = img.fliph();
                        }
                        ExifOrientation::Rotate180 => {
                            img = img.rotate180();
                        }
                        ExifOrientation::FlipVertical => img = img.flipv(),
                        ExifOrientation::Transpose => {
                            img = img.rotate90().flipv();
                        }
                        ExifOrientation::Rotate90 => {
                            img = img.rotate90();
                        }
                        ExifOrientation::Transverse => {
                            img = img.rotate270().fliph();
                        }
                        ExifOrientation::Rotate270 => {
                            img = img.rotate270();
                        }
                    }
                };
            }
        }
    }
    Ok(img)
}
