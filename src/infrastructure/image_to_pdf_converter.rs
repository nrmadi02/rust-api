use std::io::Write;
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use lopdf::{Document, Object, Stream, dictionary};

use crate::domain::image_to_pdf_converter::ImageToPdfConverter;

type DynError = Box<dyn std::error::Error + Send + Sync>;

pub struct LopImageToPdfConverter;

impl Default for LopImageToPdfConverter {
    fn default() -> Self {
        Self
    }
}

impl LopImageToPdfConverter {
    pub fn new() -> Self {
        Self
    }
}

const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const MARGIN: f32 = 20.0;

impl ImageToPdfConverter for LopImageToPdfConverter {
    fn convert(&self, image_paths: &[&Path], output: &Path) -> Result<(), DynError> {
        if image_paths.is_empty() {
            return Err("No images provided".into());
        }

        let mut doc = Document::with_version("1.4");
        let pages_id = doc.new_object_id();
        let mut page_ids = Vec::new();

        for path in image_paths {
            let img_bytes = std::fs::read(path)?;
            let img = image::load_from_memory(&img_bytes)?;
            let rgb = img.to_rgb8();
            let (img_w, img_h) = rgb.dimensions();
            let raw_data = rgb.into_raw();

            let compressed = compress_data(&raw_data)?;

            let image_stream = Stream::new(
                dictionary! {
                    "Type" => "XObject",
                    "Subtype" => "Image",
                    "Width" => img_w as i64,
                    "Height" => img_h as i64,
                    "ColorSpace" => "DeviceRGB",
                    "BitsPerComponent" => 8,
                    "Filter" => "FlateDecode",
                },
                compressed,
            );
            let image_id = doc.add_object(Object::Stream(image_stream));

            let avail_w = PAGE_WIDTH - 2.0 * MARGIN;
            let avail_h = PAGE_HEIGHT - 2.0 * MARGIN;
            let scale = (avail_w / img_w as f32).min(avail_h / img_h as f32);
            let draw_w = img_w as f32 * scale;
            let draw_h = img_h as f32 * scale;
            let x_off = (PAGE_WIDTH - draw_w) / 2.0;
            let y_off = (PAGE_HEIGHT - draw_h) / 2.0;

            let content_text = format!(
                "q\n{:.4} 0 0 {:.4} {:.4} {:.4} cm\n/Im0 Do\nQ\n",
                draw_w, draw_h, x_off, y_off
            );
            let content_stream = Stream::new(dictionary! {}, content_text.into_bytes());
            let content_id = doc.add_object(Object::Stream(content_stream));

            let resources = dictionary! {
                "XObject" => dictionary! {
                    "Im0" => image_id,
                },
            };
            let page = dictionary! {
                "Type" => "Page",
                "Parent" => pages_id,
                "MediaBox" => vec![Object::Integer(0), Object::Integer(0), Object::Real(PAGE_WIDTH), Object::Real(PAGE_HEIGHT)],
                "Contents" => content_id,
                "Resources" => resources,
            };
            let page_id = doc.add_object(page);
            page_ids.push(Object::Reference(page_id));
        }

        let page_count = page_ids.len() as i64;
        let pages = dictionary! {
            "Type" => "Pages",
            "Kids" => page_ids,
            "Count" => page_count,
        };
        doc.objects.insert(pages_id, Object::Dictionary(pages));

        let catalog_id = doc.add_object(dictionary! {
            "Type" => "Catalog",
            "Pages" => pages_id,
        });
        doc.trailer.set("Root", catalog_id);

        if let Some(parent) = output.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut buffer = Vec::new();
        doc.save_to(&mut buffer)?;
        std::fs::write(output, &buffer)?;

        Ok(())
    }
}

fn compress_data(data: &[u8]) -> Result<Vec<u8>, std::io::Error> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
    encoder.write_all(data)?;
    encoder.finish()
}
