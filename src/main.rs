mod utils;
mod win;
mod imgs;

use std::thread;
use std::fs::File;
use std::io::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use template_matching::{find_extremes, MatchTemplateMethod, TemplateMatcher};


fn main() {
	// when panic, pause console:
	std::panic::set_hook(Box::new(|panic_info| {
		println!("Panic occurred:\n{:?}", panic_info);
		let _ = std::process::Command::new("cmd").arg("/c").arg("pause").status();
	}));

	// load config:
	let cfg_path = "configs.json";
	let file = File::open(cfg_path).expect("找不到配置文件! (configs.json)\n");
	let reader = std::io::BufReader::new(file);
	let data: utils::ConfigData = serde_json::from_reader(reader)
		.expect("配置文件json解析错误!\n");
	utils::print_help(&data);

	let loop_flag = Arc::new(AtomicBool::new(false));

	// start input listen thread:
	let loop_flag_clone = Arc::clone(&loop_flag);
	let input_listen_thread = thread::spawn(move || {
		win::input_listen(loop_flag_clone);
	});

	// main loop:
	loop {
		print!("Enter command: ");
		std::io::stdout().flush().unwrap();
		let mut input = String::new();
		std::io::stdin().read_line(&mut input).unwrap();
		let cmd = input.trim();
		if let "q" | "quit" | "exit" = cmd { break; }
		if let "" = cmd { continue; }
		if let "h" | "help" = cmd { utils::print_help(&data); continue; }
		if let "f" = cmd { test_once(); continue; }
		if let "t" | "test" = cmd {
			loop_flag.store(true, Ordering::SeqCst);
			let test_loop_c = Arc::clone(&loop_flag);
			let t = thread::spawn(move || {
				test(test_loop_c);
			});
			win::disable_input_when_looping(&loop_flag);
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
			win::disable_input_when_looping(&loop_flag);
			t.join().unwrap();
			continue;
		}
		println!("未知指令. (输入 help 或 h 获取帮助)");
	}
	winput::message_loop::stop();
	input_listen_thread.join().unwrap();
}

fn test_once() {
	println!("---");
}

fn test(looping: Arc<AtomicBool>) {
	let mut print_dots = utils::looping_print_func();
	loop {
		if !looping.load(Ordering::SeqCst) {
			break;
		}
		print_dots();
		thread::sleep(std::time::Duration::from_millis(100));
	}
	utils::clear_line();
	println!("Test looping finished!");
}

fn match_clicks(looping: Arc<AtomicBool>, cfg: utils::Config) {
	let win_list = win_screenshot::prelude::window_list().unwrap();
	let window = win_list.iter().find(
		|i| i.window_name.contains(&cfg.window_name)
	).unwrap();
	// 1. Loading template images:
	let templates_path = format!("res/{}/", &cfg.match_pic_path);
	println!("{} -- Loading template images from \"{}\" ...",
		utils::now_str(), &templates_path
	);
	let img_dict = imgs::read_pic_from_dir(&templates_path);
	println!("\n{} -- Template images all loaded.", utils::now_str());
	// 2. Start Matching & Clicking:
	win::set_window_rect(
		window.hwnd,
		cfg.client_width + utils::WINPAD_X,
		cfg.client_height + utils::WINPAD_Y
	);
	let mut matcher = TemplateMatcher::new();
	let mut print_dots = utils::looping_print_func();
	loop {
		print_dots();
		if !looping.load(Ordering::SeqCst) {
			utils::clear_line();
			println!("({}) Finished!", &cfg.alias);
			break;
		}
		for (img_title, img) in img_dict.iter() {
			matcher.match_template(
				&imgs::capture_window(window.hwnd), img,
				MatchTemplateMethod::SumOfSquaredDifferences
			);
			let extremes = find_extremes(&matcher.wait_for_result().unwrap());
			if extremes.min_value < 3.0 {
				utils::clear_line();
				println!("{} - ({}) Found! diff({})",
					utils::now_str(), img_title, extremes.min_value
				);
				let real_x = extremes.min_value_location.0 + img.width() / 2;
				let real_y = extremes.min_value_location.1 + img.height() / 2;
				win::click(window.hwnd, real_x, real_y, !cfg.foreground);
			} else if extremes.min_value < 8.0 {
				utils::clear_line();
				println!("{} - ({}) Nearly found. diff({})",
					utils::now_str(), img_title, extremes.min_value
				);
			}
		}
		thread::sleep(std::time::Duration::from_millis(50));
	}
}


