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
	let match_loop = Arc::new(AtomicBool::new(false));
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
			test();
			continue;
		}
		if let Some(cfg) = data.cfgs.iter().find(|c| c.cmd == cmd) {
			println!("Config({}) Loaded!", &cfg.alias);
			match_loop.store(true, Ordering::SeqCst);
			let match_loop_c = Arc::clone(&match_loop);
			let config = cfg.clone();
			let t = thread::spawn(move || {
				match_clicks(match_loop_c, config);
			});
			wait_for_esc(&match_loop);  // 等待按下Esc键
			t.join().unwrap();
			continue;
		}
		println!("未知指令. (输入 help 或 h 获取帮助)");
	}
}

fn wait_for_esc(loop_flag: &Arc<AtomicBool>) {
	crossterm::terminal::enable_raw_mode().unwrap(); // 启用原始模式
	loop {
		if let crossterm::event::Event::Key(key_event) = crossterm::event::read().unwrap() {
			if key_event.code == crossterm::event::KeyCode::Esc {
				loop_flag.store(false, Ordering::SeqCst);
				break;
			}
		}
	}
	crossterm::terminal::disable_raw_mode().unwrap(); // 禁用原始模式
}

fn test() {
	let title = "星穹铁道";
	// let title = "yysls";
	let win_list = window_list().unwrap();
	// let window = win_list.iter().find(|i| i.window_name.contains(title)).unwrap();
	if let Some(window) = win_list.iter().find(|i| i.window_name.contains(title)) {
		foreground_window_and_click(window.hwnd, 100, 100);
		println!("window found {}, and click at (100, 100)", window.window_name);
	} else {
		println!("window not found");
	}
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
	println!("Start Matching & Clicking...");
	let mut matcher = TemplateMatcher::new();
	loop {
		if !looping.load(Ordering::SeqCst) {
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
				println!("template_image({}) Found! with diff({})", img_title, extremes.min_value);
				let real_x = extremes.min_value_location.0 + img_width / 2;
				let real_y = extremes.min_value_location.1 + img_height / 2;
				if cfg.foreground {
					foreground_window_and_click(window.hwnd, real_x as i32, real_y as i32);
				} else {
					send_click_event_to_window(window.hwnd, real_x as isize, real_y as isize);
				}
			} else if extremes.min_value < 20.0 {
				println!("template_image({}) nearly found... with diff({})", img_title, extremes.min_value);
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
	println!("t | test: 运行测试代码");
	println!("q | quit | exit: 退出程序");
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


