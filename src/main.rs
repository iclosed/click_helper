use serde_derive::Deserialize;
use std::fs::File;
use std::io::BufReader;

#[derive(Deserialize)]
struct ConfigData {
    cfgs: Vec<Config>
}

#[derive(Deserialize)]
struct Config {
    cmd: String,  // 控制台指令 (必须定义)
    res_path: String,  // 资源目录 (必须定义)
    #[serde(default)] alias: String,  // 别名
    #[serde(default)] find_clicks: Vec<String>,  // 查找并点击的模板
}

fn main() {
	let path = "res/configs.json";

	let file = File::open(path).unwrap();
	let reader = BufReader::new(file);

	let data: ConfigData = serde_json::from_reader(reader).unwrap();

	// 访问数据
	for cfg in data.cfgs {
        println!("~~~~~~~~~~~~~~~");
		println!("Name: {}", cfg.alias);
		println!("cmd: {}", cfg.cmd);
		println!("res_path: {}", cfg.res_path);
		for img in cfg.find_clicks {
			println!("loop img: {}", img);
		}
	}
}