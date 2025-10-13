//! Loading user applications into memory

use lazy_static::lazy_static;
use alloc::vec::Vec;

use crate::println;

/// Get the total number of applications.
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        safe fn _num_app();
    }
    unsafe { (_num_app as usize as *const usize).read_volatile() }
}

/// get applications data
pub fn get_app_data(app_id: usize) -> &'static [u8] {
    unsafe extern "C" {
        safe fn _num_app();
    }
    let num_app_ptr = _num_app as usize as *const usize;
    let num_app = get_num_app();
    let app_start = unsafe { core::slice::from_raw_parts(num_app_ptr.add(1), num_app + 1) };
    assert!(app_id < num_app);
    unsafe {
        core::slice::from_raw_parts(
            app_start[app_id] as *const u8,
            app_start[app_id + 1] - app_start[app_id],
        )
    }
}

lazy_static! {
    /// 存储用户空间应用程序名称的静态变量
    static ref APP_NAMES: Vec<&'static str> = {
        let num_app = get_num_app();
        unsafe extern "C" { safe fn _app_names();}

        // 首先，初始化读取的起始位置为 _app_names 符号的地址
        let mut start = _app_names as usize as *const u8;
        let mut v = Vec::new();

        unsafe {
            // 而后，逐个读取字符串，直到读取到所有应用程序名称
            // 每个字符串以 '\0' 结尾，因此通过查找 '\0' 来确定字符串的结束位置
            for _ in 0..num_app {
                let mut end = start;
                while end.read_volatile() != '\0' as u8 {
                    end = end.add(1);
                }
                let slice = core::slice::from_raw_parts(start, end as usize - start as usize);
                v.push(core::str::from_utf8(slice).unwrap());
                // 读取完当前字符串后，将起始位置移动到下一个字符串的开始位置
                // 由于是以 '\0' 结尾，因此需要加 1
                start = end.add(1);
            }
        }
        v
    };
}

pub fn get_app_data_by_name(name: &str) -> Option<&'static [u8]> {
    let num_app = get_num_app();
    (0..num_app).find(|&index| APP_NAMES[index] == name).map(|index| get_app_data(index))
}

pub fn list_apps() {
    println!("/**** APPS ****");
    for app_name in APP_NAMES.iter() {
        println!("* {}", app_name);
    }
    println!("****************/");
}