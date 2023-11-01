use std::io::Write;
use chrono::format::strftime::StrftimeItems;
use serde_derive::Deserialize;


pub const WINPAD_X: i32 = 16;
pub const WINPAD_Y: i32 = 39;


#[derive(Deserialize)]
pub struct ConfigData {
	pub cfgs: Vec<Config>
}

#[derive(Deserialize, Clone)]
pub struct Config {
	pub cmd: String,  // 控制台指令
	pub window_name: String,  // 窗口名称(子集即可)
	pub client_width: i32,
	pub client_height: i32,
	#[serde(default)]
	pub foreground: bool,  // 点击是否需要置顶窗口
	#[serde(default)]
	pub alias: String,  // 别名
	#[serde(default)]
	pub match_pic_path: String,  // 查找并点击的模板
}

pub fn print_help(data: &ConfigData) {
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

pub fn looping_print_func() -> impl FnMut() -> u32 {
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

pub fn clear_line() {
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

pub fn now_str() -> String {
	let local_time = chrono::Local::now();
	let format_items = StrftimeItems::new("%Y-%m-%d %H:%M:%S");
	return local_time.format_with_items(format_items).to_string();
}
