use crate::utils;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use winput::{Vk, Action};
use winput::message_loop;
use windows::{Win32::Foundation::*, Win32::UI::WindowsAndMessaging::*};
use crossterm::event::*;
use crossterm::terminal::*;


#[allow(dead_code)]
pub fn get_window_resolution(hwnd: isize) -> (i32, i32) {
	let mut info = WINDOWINFO {
		cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
		..Default::default()
	};
	unsafe {
		if let Ok(_) = GetWindowInfo(HWND(hwnd), &mut info) {
			(
				info.rcWindow.right - info.rcWindow.left - utils::WINPAD_X,
				info.rcWindow.bottom - info.rcWindow.top - utils::WINPAD_Y
			)
		} else {
			(-1, -1)
		}
	}
}

pub fn set_window_rect(hwnd: isize, width: i32, height: i32) {
	unsafe {
		SetWindowPos(
			HWND(hwnd), HWND_TOP, 0, 0, width, height,
			SWP_DRAWFRAME | SWP_NOMOVE | SWP_NOZORDER
		).unwrap();
	}
}

pub fn click(hwnd: isize, x: u32, y:u32, background: bool) {
	if background {
		send_click_event_to_window(
			hwnd, x as isize, y as isize
		);
	} else {
		foreground_window_and_click(
			hwnd, x as i32, y as i32
		);
	}
}

pub fn send_click_event_to_window(hwnd: isize, x: isize, y: isize) {
	/*
		0.(optional) use spy++ to capture target window's mouse events.
		1. SendMessage or PostMessage to simulate the mouse.
	*/
	println!("{} -- send click event ({}, {}) to window({})",
		utils::now_str(), x, y, hwnd
	);
	let lpos = LPARAM(x | (y << 16));
	let lmmv = LPARAM(1 | ((WM_MOUSEMOVE as isize)<<16));
	let lmbd = LPARAM(1 | ((WM_LBUTTONDOWN as isize)<<16));
	unsafe {
		SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), lmmv);
		PostMessageW(HWND(hwnd), WM_MOUSEMOVE, WPARAM(1), lpos).unwrap();
		std::thread::sleep(std::time::Duration::from_millis(100));

		SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), lmbd);
		PostMessageW(HWND(hwnd), WM_LBUTTONDOWN, WPARAM(1), lpos).unwrap();
		std::thread::sleep(std::time::Duration::from_millis(100));

		PostMessageW(HWND(hwnd), WM_LBUTTONUP, WPARAM(1), lpos).unwrap();
	}
}

pub fn foreground_window_and_click(hwnd: isize, x: i32, y: i32) {
	/* for windows that can't simulate mouse click by PostMessages
		1. Set target window foreground.
		2. Simulate basic mouse movement and click action.
	*/
	println!("{} -- foreground window and click ({}, {})", utils::now_str(), x, y);
	let mut info = WINDOWINFO {
		cbSize: core::mem::size_of::<WINDOWINFO>() as u32,
		..Default::default()
	};
	unsafe {
		let working_hwnd = GetForegroundWindow();
		GetWindowInfo(HWND(hwnd), &mut info).unwrap();
		SetForegroundWindow(HWND(hwnd));

		std::thread::sleep(std::time::Duration::from_millis(10));

		let real_x = info.rcClient.left + x;
		let real_y = info.rcClient.top + y;
		winput::Mouse::set_position(real_x, real_y).unwrap();
		winput::send(winput::Button::Left);
		// winput::Mouse::move_relative(1, 0);

		std::thread::sleep(std::time::Duration::from_millis(100));
		SetForegroundWindow(working_hwnd);
	}
}

pub fn left_button_down(hwnd: isize) {
	unsafe {
		// SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), lmbd);
		PostMessageW(HWND(hwnd), WM_LBUTTONDOWN, WPARAM(1), LPARAM(0)).unwrap();
	}
}

pub fn right_button_down(hwnd: isize) {
	unsafe {
		// SendMessageW(HWND(hwnd), WM_SETCURSOR, WPARAM(hwnd as usize), lmbd);
		PostMessageW(HWND(hwnd), WM_RBUTTONDOWN, WPARAM(1), LPARAM(0)).unwrap();
	}
}

pub fn input_listen(loop_flag: Arc<AtomicBool>) {
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
				if vk == Vk::F9 {
					let win_list = win_screenshot::prelude::window_list().unwrap();
					let window = win_list.iter().find(
						|i| i.window_name.contains("Minecraft")
					).unwrap();
					left_button_down(window.hwnd);
				}
				if vk == Vk::F10 {
					let win_list = win_screenshot::prelude::window_list().unwrap();
					let window = win_list.iter().find(
						|i| i.window_name.contains("Minecraft")
					).unwrap();
					right_button_down(window.hwnd);
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

pub fn disable_input_when_looping(looping: &Arc<AtomicBool>) {
	enable_raw_mode().unwrap(); // 启用原始模式
	loop {
		if !looping.load(Ordering::SeqCst) {
			break;
		}
		if let Event::Key(key_event) = read().unwrap() {
			if key_event.code == KeyCode::Esc {
				looping.store(false, Ordering::SeqCst);
				break;
			}
		}
	}
	disable_raw_mode().unwrap(); // 禁用原始模式
}


