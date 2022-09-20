

use core::fmt::{Write, Result, Arguments};

use crate::sbi::*;

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! info {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        // $crate::console::print(format_args!(concat!("\x1b[1;34m", "[INFO] ", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
        $crate::console::print(format_args!(concat!("[INFO] ", $fmt, "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! warn {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        // #[cfg(not(feature = "not_debug"))]
        // $crate::console::print(format_args!(concat!("\x1b[1;33m", "[WARN] ", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! debug {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        // #[cfg(not(feature = "not_debug"))]
        // $crate::console::print(format_args!(concat!("\x1b[1;31m", "[DEBUG] ", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! error {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        // #[cfg(not(feature = "not_debug"))]
        // $crate::console::print(format_args!(concat!("\x1b[1;31m", "[ERROR] ", $fmt, "\x1b[0m", "\n") $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

// // 读入一个字符
// #[allow(unused)]
// pub fn read() -> char {
//     console_getchar()
// }

// // 无回显输入
// #[allow(unused)]
// pub fn read_line(str: &mut String) {
//     loop {
//         let c = read();
//         if c == '\n' {
//             break;
//         }
//         str.push(c);
//     }
// }

// // 有回显输入
// #[allow(unused)]
// pub fn read_line_display(str: &mut String) {
//     loop {
//         let c = read();
        
//         if c as u8 >= 0b11000000 {
//             // 获取到utf8字符 转unicode
//             console_putchar(c as u8);
//             let mut char_u32:u32 = c as u32;
//             let times = if c as u8 <= 0b11100000 {
//                 char_u32 = char_u32 & 0x1f;
//                 1
//             } else if c as u8 <= 0b11110000 {
//                 char_u32 = char_u32 & 0x0f;
//                 2
//             } else {
//                 char_u32 = char_u32 & 0x07;
//                 3
//             };
            

//             for _ in 0..times {
//                 let c = read();
//                 console_putchar(c as u8);
//                 char_u32 = char_u32 << 6;
//                 char_u32 = char_u32 | ((c as u32) & 0x3f);
//             }
            
//             str.push(char::from_u32(char_u32).unwrap());
//             continue;
//         }
        
//         match c as u8 {
//             0x0D => {       // 回车
//                 console_putchar(0xa);
//                 break;
//             },
//             0x7F => {       // 退格
//                 console_putchar(0x08);  // 回到上一格
//                 console_putchar(' ' as u8);  // 填充空格
//                 console_putchar(0x08);  // 回到上一格
//                 str.pop();
//             },
//             _ => {
//                 console_putchar(c as u8);
//                 str.push(c);
//             }
//         }
//     }
// }

struct Stdout;

// 实现输出Trait
impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> Result {
        let mut buffer = [0u8; 4];
        for c in s.chars() {
            for code_point in c.encode_utf8(&mut buffer).as_bytes().iter() {
                console_putchar(*code_point);
            }
        }
        Ok(())
    }
}

// 输出函数
pub fn puts(args: &[u8]) {
    for i in args {
        console_putchar(*i);
    }
}

// 输出函数
pub fn print(args: Arguments) {
    Stdout.write_fmt(args).unwrap();
}