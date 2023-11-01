use std::collections::HashMap;

use image::{ImageBuffer, RgbaImage, Rgba, Luma};
use win_screenshot::prelude::*;

pub type MatchImg = ImageBuffer<Luma<f32>, Vec<f32>>;


pub fn capture_window(hwnd: isize) -> MatchImg {
	let buf = capture_window_ex(
		hwnd, Using::PrintWindow, Area::ClientOnly, None, None
	).unwrap();
	let img_rgb = RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap();
	return rgba_to_luma_f32(&img_rgb);
}

pub fn read_pic_from_dir(path: &str) -> HashMap<String, MatchImg> {
	let entries = std::fs::read_dir(path).unwrap();
	let mut img_dict: HashMap<String, MatchImg> = HashMap::new();
	for entry in entries {
		if let Ok(entry) = entry {
			let path = entry.path();
			if !path.is_file() {
				continue;
			}
			if let Some(ext) = path.extension() {
				if ext != "png" && ext != "jpg" && ext != "jpeg" {
					continue;
				}
				if let Ok(img) = image::open(&path) {
					let fname = path.file_stem().unwrap()
						.to_string_lossy().into_owned();
					img_dict.insert(fname, img.to_luma32f());
				}
			}
		}
	}
	img_dict
}

fn rgba_to_luma_f32(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> MatchImg {
	let (width, height) = image.dimensions();
	let mut result = ImageBuffer::new(width, height);
	for y in 0..height {
		for x in 0..width {
			let rgba_pixel = image.get_pixel(x, y);
			let luma_value = rgba_to_luma_f32_pixel(rgba_pixel);
			result.put_pixel(x, y, luma_value);
		}
	}
	result
}

fn rgba_to_luma_f32_pixel(rgba_pixel: &Rgba<u8>) -> Luma<f32> {
	let red = f32::from(rgba_pixel[0]) / 255.0;
	let green = f32::from(rgba_pixel[1]) / 255.0;
	let blue = f32::from(rgba_pixel[2]) / 255.0;
	let alpha = f32::from(rgba_pixel[3]) / 255.0;
	// calculate gray value:
	let luma_value = (0.299 * red + 0.587 * green + 0.114 * blue) * alpha;
	Luma([luma_value])
}


