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

const WINPAD_X: i32 = 16;
const WINPAD_Y: i32 = 39;

#[derive(Deserialize)]
struct ConfigData {
	cfgs: Vec<Config>
}

#[derive(Deserialize, Clone)]
struct Config {
	cmd: String,  // 控制台指令
	window_name: String,  // 窗口名称(子集即可)
	client_width: i32,
	client_height: i32,
	#[serde(default)] foreground: bool,  // 点击是否需要置顶窗口
	#[serde(default)] alias: String,  // 别名
	#[serde(default)] match_pic_path: String,  // 查找并点击的模板
}

fn main() {
	std::panic::set_hook(Box::new(|panic_info| {
		println!("Panic occurred:\n{:?}", panic_info);
		let _ = std::process::Command::new("cmd").arg("/c").arg("pause").status();
	}));
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
		std::io::stdout().flush().unwrap();
		let mut input = String::new();
		std::io::stdin().read_line(&mut input).unwrap();
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
		if let "f" = cmd {
			test_once();
			continue;
		}
		if let "t" | "test" = cmd {
			loop_flag.store(true, Ordering::SeqCst);
			let test_loop_c = Arc::clone(&loop_flag);
			let t = thread::spawn(move || {
				test(test_loop_c);
			});
			disable_input_when_looping(&loop_flag);
			t.join().unwrap();
			continue;
		}
		if let Some(cfg) = data.cfgs.iter().find(|c| c.cmd == cmd) {
			println!("({}) Loaded!", &cfg.alias);
			loop_flag.store(true, Ordering::SeqCst);
			let match_loop = Arc::clone(&loop_flag);
			let config = cfg.clone();
			let t = thread::spawn(move || {
				match_clicks(match_loop, config);
			});
			disable_input_when_looping(&loop_flag);
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
	let mut shift_holder = false;
	loop {
		if !message_loop::is_active() {
			break;
		}
		match receiver.next_event() {
			message_loop::Event::Keyboard {vk, action: Action::Press, ..} => {
				if vk == Vk::Shift {
					shift_holder = true;
				}
				if vk == Vk::Q && shift_holder {
					loop_flag.store(false, Ordering::SeqCst);
				}
			},
			message_loop::Event::Keyboard {vk, action: Action::Release, ..} => {
				if vk == Vk::Shift {
					shift_holder = false;
				}
			},
			_ => (),
		}
	}
}

fn test_once() {
	let entries = std::fs::read_dir("res/star_rail/").expect("Failed to read directory");

	// 迭代处理每个文件
	for entry in entries {
		if let Ok(entry) = entry {
			let path = entry.path();
			if !path.is_file() {
				continue;
			}
			let file_name = path.file_stem().unwrap().to_str().unwrap();
			let ext = path.extension().unwrap().to_str().unwrap();
			println!("File name: {}", file_name);
			println!("Extension: {}", ext);
			println!("---");
		}
	}
}

fn test(looping: Arc<AtomicBool>) {
	let mut print_dots = looping_print_func();
	loop {
		if !looping.load(Ordering::SeqCst) {
			break;
		}
		print_dots();
		thread::sleep(Duration::from_millis(100));
	}
	clear_line();
	println!("Test looping finished!");
}

fn looping_print_func() -> impl FnMut() -> u32 {
	let len = 5;
	let mut counter = 0;
	let closure = move || {
		clear_line();
		print!("Procesing");
		for _ in 0..counter {
			print!(".");
		}
		print!(" (Shift+Q to stop)");
		std::io::stdout().flush().unwrap();
		counter += 1;
		if counter > len {
			counter = 0;
		}
		counter
	};
	closure
}

fn clear_line() {
	let col = 120;
	for _ in 0..col {
		print!("\x08");
	}
	for _ in 0..col {
		print!("\x20");
	}
	for _ in 0..col {
		print!("\x08");
	}
}

fn disable_input_when_looping(looping: &Arc<AtomicBool>) {
	crossterm::terminal::enable_raw_mode().unwrap(); // 启用原始模式
	loop {
		if !looping.load(Ordering::SeqCst) {
			break;
		}
		if let crossterm::event::Event::Key(key_event) = crossterm::event::read().unwrap() {
			if key_event.code == crossterm::event::KeyCode::Esc {
				looping.store(false, Ordering::SeqCst);
				break;
			}
		}
	}
	crossterm::terminal::disable_raw_mode().unwrap(); // 禁用原始模式
}

fn match_clicks(looping: Arc<AtomicBool>, cfg: Config) {
	let win_list = window_list().unwrap();
	let window = win_list.iter().find(|i| i.window_name.contains(&cfg.window_name)).unwrap();
	// 1. Loading template images:
	let templates_path = format!("res/{}/", &cfg.match_pic_path);
	println!("{} -- Loading template images from \"{}\" ....", now_str(), &templates_path);
	let entries = std::fs::read_dir(templates_path).unwrap();
	let mut img_dict: HashMap<String, ImageBuffer<Luma<f32>, Vec<f32>>> = HashMap::new();
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
					let fname = path.file_stem().unwrap().to_string_lossy().into_owned();
					print!("({}), ", &fname);
					std::io::stdout().flush().unwrap();
					img_dict.insert(fname, img.to_luma32f());
				}
			}
		}
	}
	println!("\n{} -- Template images all loaded.", now_str());
	// 2. Start Matching & Clicking:
	set_window_rect(window.hwnd, cfg.client_width + WINPAD_X, cfg.client_height + WINPAD_Y);
	let mut matcher = TemplateMatcher::new();
	let mut print_dots = looping_print_func();
	loop {
		print_dots();
		if !looping.load(Ordering::SeqCst) {
			clear_line();
			println!("({}) Finished!", &cfg.alias);
			break;
		}
		let buf = capture_window_ex(window.hwnd, Using::PrintWindow, Area::ClientOnly, None, None).unwrap();
		let img_rgb = RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap();
		let input_image = rgba_to_luma_f32(&img_rgb);
		for (img_title, img) in img_dict.iter() {
			let img_width = img.width();
			let img_height = img.height();
			matcher.match_template(&input_image, img, MatchTemplateMethod::SumOfSquaredDifferences);
			let extremes = find_extremes(&matcher.wait_for_result().unwrap());
			if extremes.min_value < 3.0 {
				clear_line();
				println!("{} -- ({}) Found! with diff({})", now_str(), img_title, extremes.min_value);
				let real_x = extremes.min_value_location.0 + img_width / 2;
				let real_y = extremes.min_value_location.1 + img_height / 2;
				if cfg.foreground {
					foreground_window_and_click(window.hwnd, real_x as i32, real_y as i32);
				} else {
					send_click_event_to_window(window.hwnd, real_x as isize, real_y as isize);
				}
			} else if extremes.min_value < 8.0 {
				clear_line();
				println!("{} -- ({}) Nearly found. diff({})", now_str(), img_title, extremes.min_value);
			}
		}
		thread::sleep(Duration::from_millis(50));
	}
}

fn send_click_event_to_window(hwnd: isize, x: isize, y: isize) {
	/*
		0.(optional) use spy++ to capture target window's mouse events.
		1. SendMessage or PostMessage to simulate the mouse.
	*/
	println!("{} -- send click event ({}, {}) to window({})", now_str(), x, y, hwnd);
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
	println!("{} -- foreground the window and click ({}, {})", now_str(), x, y);
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

fn _get_window_rect(hwnd: isize) {
	let mut info = WINDOWINFO {
		cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
		..Default::default()
	};
	unsafe { GetWindowInfo(HWND(hwnd), &mut info).unwrap(); }
	println!("Client width: {}, height: {}",
		info.rcWindow.right - info.rcWindow.left,
		info.rcWindow.bottom - info.rcWindow.top
	);
}

fn set_window_rect(hwnd: isize, width: i32, height: i32) {
	unsafe {
		SetWindowPos(
			HWND(hwnd), HWND_TOP, 0, 0, width, height,
			SWP_DRAWFRAME | SWP_NOMOVE | SWP_NOZORDER
		).unwrap();
	}
}

fn now_str() -> String {
	let local_time = chrono::Local::now();
	let format_items = chrono::format::strftime::StrftimeItems::new("%Y-%m-%d %H:%M:%S");
	let formatted_time = local_time.format_with_items(format_items).to_string();
	formatted_time
}

fn print_help(data: &ConfigData) {
	println!();
	println!("------------可用指令-------------");
	for cfg in &data.cfgs {
		println!("{}: 运行{}", cfg.cmd, cfg.alias);
	}
	println!("t, test: 运行测试代码");
	println!("q, quit, exit: 退出程序");
	println!("--------------------------------");
	println!();
}


