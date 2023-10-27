use std::thread;
use std::fs::File;
use std::io::Write;
use std::time::Duration;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use template_matching::{find_extremes, MatchTemplateMethod, TemplateMatcher};
use image::{ImageBuffer, RgbaImage, Rgba, Luma};
use win_screenshot::prelude::*;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};
use serde_derive::Deserialize;

use winput::{Vk, Action};
use winput::message_loop;

#[derive(Deserialize)]
struct ConfigData {
	cfgs: Vec<Config>
}

#[derive(Deserialize, Clone)]
struct Config {
	cmd: String,  // 控制台指令 (必须定义)
	res_path: String,  // 资源目录 (必须定义)
	window_name: String,  // 目标窗口名称子集
	#[serde(default)] foreground: bool,  // 点击是否需要置顶窗口
	#[serde(default)] alias: String,  // 别名
	#[serde(default)] matches: Vec<String>,  // 查找并点击的模板
}

fn main() {
	let cfg_path = "configs.json";
	let file = File::open(cfg_path).expect("找不到配置文件! (configs.json)\n");
	let reader = std::io::BufReader::new(file);
	let data: ConfigData = serde_json::from_reader(reader).expect("配置文件json解析错误!\n");
	print_help(&data);
	let loop_flag = Arc::new(AtomicBool::new(false));
	let loop_flag_clone = Arc::clone(&loop_flag);
	let input_listen_thread = thread::spawn(move || {
		input_listen(loop_flag_clone);
	});
	loop {
		print!("Enter command: ");
		std::io::stdout().flush().expect("Flush ERROR");
		let mut input = String::new();
		std::io::stdin().read_line(&mut input).expect("System input ERROR");
		let cmd = input.trim();
		if let "q" | "quit" | "exit" = cmd {
			break;
		}
		if cmd == "" {
			continue;
		}
		if let "h" | "help" = cmd {
			print_help(&data);
			continue;
		}
		if let "t" | "test" = cmd {
			loop_flag.store(true, Ordering::SeqCst);
			let test_loop_c = Arc::clone(&loop_flag);
			let t = thread::spawn(move || {
				test(test_loop_c);
			});
			t.join().unwrap();
			continue;
		}
		if let Some(cfg) = data.cfgs.iter().find(|c| c.cmd == cmd) {
			println!("Config({}) Loaded!", &cfg.alias);
			loop_flag.store(true, Ordering::SeqCst);
			let match_loop = Arc::clone(&loop_flag);
			let config = cfg.clone();
			let t = thread::spawn(move || {
				match_clicks(match_loop, config);
			});
			t.join().unwrap();
			continue;
		}
		println!("未知指令. (输入 help 或 h 获取帮助)");
	}
	message_loop::stop();
	input_listen_thread.join().unwrap();
}

fn input_listen(loop_flag: Arc<AtomicBool>) {
	let receiver = message_loop::start().unwrap();
	loop {
		if !message_loop::is_active() {
			break;
		}
		match receiver.next_event() {
			message_loop::Event::Keyboard {vk, action: Action::Press, ..} => {
				if vk == Vk::Escape {
					loop_flag.store(false, Ordering::SeqCst);
				} else {
					// println!("{:?} was pressed!", vk);
				}
			},
			_ => (),
		}
	}
}

fn test(looping: Arc<AtomicBool>) {
	let mut dots = 1;
	let mut print_dots = print_dots_func();
	loop {
		if !looping.load(Ordering::SeqCst) {
			print!("\r\x1B[2K");
			break;
		}
		print_dots();
		thread::sleep(Duration::from_millis(500));
	}
}

fn print_dots_func() -> impl FnMut() -> u32 {
	let mut counter = 0;
	let mut closure = move || {
		counter += 1;
		if counter > 3 {
			counter = 0;
		}
		for _ in 0..99 {
			print!("\x08");
		}
		// print!("\r\x1B[2K");
		print!("Procesing");
		for _ in 0..counter {
			print!(".")
		}
		std::io::stdout().flush().expect("Flush ERROR");
		counter
	};
	closure
}

fn match_clicks(looping: Arc<AtomicBool>, cfg: Config) {
	let win_list = window_list().unwrap();
	let window = win_list.iter().find(|i| i.window_name.contains(&cfg.window_name)).unwrap();
	// 1. Loading template images:
	println!("Loading template images...");
	let mut img_dict: HashMap<String, ImageBuffer<Luma<f32>, Vec<f32>>> = HashMap::new();
	for img_title in &cfg.matches {
		if let Ok(img) = image::open(format!("res/{}{}", &cfg.res_path, img_title)) {
			img_dict.insert(img_title.to_string(), img.to_luma32f());
		} else {
			continue;
		}
	}
	// 2. Start Matching & Clicking:
	let mut matcher = TemplateMatcher::new();
	let mut print_dots = print_dots_func();
	loop {
		if !looping.load(Ordering::SeqCst) {
			print!("\r\x1B[2K");
			break;
		}
		print_dots();
		let buf = capture_window_ex(window.hwnd, Using::PrintWindow, Area::ClientOnly, None, None).unwrap();
		let img_rgb = RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap();
		let input_image = rgba_to_luma_f32(&img_rgb);
		for (img_title, img) in img_dict.iter() {
			let img_width = img.width();
			let img_height = img.height();
			matcher.match_template(&input_image, img, MatchTemplateMethod::SumOfSquaredDifferences);
			let extremes = find_extremes(&matcher.wait_for_result().unwrap());
			if extremes.min_value < 2.0 {
				print!("\r\x1B[2K");
				println!("template_image({}) Found! with diff({})", img_title, extremes.min_value);
				let real_x = extremes.min_value_location.0 + img_width / 2;
				let real_y = extremes.min_value_location.1 + img_height / 2;
				if cfg.foreground {
					foreground_window_and_click(window.hwnd, real_x as i32, real_y as i32);
				} else {
					send_click_event_to_window(window.hwnd, real_x as isize, real_y as isize);
				}
			} else if extremes.min_value < 5.0 {
				// println!("template_image({}) nearly found... with diff({})", img_title, extremes.min_value);
			}
		}
		thread::sleep(Duration::from_millis(100));
	}
}

fn send_click_event_to_window(hwnd: isize, x: isize, y: isize) {
	/*
		0.(optional) use spy++ to capture target window's mouse events.
		1. SendMessage or PostMessage to simulate the mouse.
	*/
	println!("send click event ({}, {}) to window({})", x, y, hwnd);
	let lpos = LPARAM(x | (y << 16));
	unsafe {
		SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), LPARAM(1 | ((WM_MOUSEMOVE as isize)<<16)));
		PostMessageW(HWND(hwnd), WM_MOUSEMOVE, WPARAM(1), lpos).unwrap();
		thread::sleep(Duration::from_millis(100));
		SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), LPARAM(1 | ((WM_LBUTTONDOWN as isize)<<16)));
		PostMessageW(HWND(hwnd), WM_LBUTTONDOWN, WPARAM(1), lpos).unwrap();
		thread::sleep(Duration::from_millis(100));
		PostMessageW(HWND(hwnd), WM_LBUTTONUP, WPARAM(1), lpos).unwrap();
	}
}

fn foreground_window_and_click(hwnd: isize, x: i32, y: i32) {
	/* for windows that can't simulate mouse click by PostMessages
		1. Set target window foreground.
		2. Simulate basic mouse movement and click action.
	*/
	println!("foreground window({}) and click ({}, {})", hwnd, x, y);
	let mut info = WINDOWINFO {
		cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
		..Default::default()
	};
	unsafe {
		GetWindowInfo(HWND(hwnd), &mut info).unwrap();
		SetForegroundWindow(HWND(hwnd));
	}
	// println!("({}, {}) vs ({}, {})", info.rcClient.left, info.rcClient.top, info.rcWindow.left, info.rcWindow.top);
	thread::sleep(Duration::from_millis(10));
	winput::Mouse::set_position(info.rcClient.left + x, info.rcClient.top + y).unwrap();
	winput::send(winput::Button::Left);
	// winput::Mouse::move_relative(1, 0);
}

fn print_help(data: &ConfigData) {
	println!("------------可用指令-------------");
	for cfg in &data.cfgs {
		println!("{}: 运行{}", cfg.cmd, cfg.alias);
	}
	println!("t, test: 运行测试代码");
	println!("q, quit, exit: 退出程序");
	println!("--------------------------------");
	println!();
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

fn rgba_to_luma_f32(image: &ImageBuffer<Rgba<u8>, Vec<u8>>) -> ImageBuffer<Luma<f32>, Vec<f32>> {
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




