//! 任意 PNG → 多尺寸 ICO（命令行示例）。
//!
//! 用法：
//! ```bash
//! cargo run --example png_to_ico -- logo.png logo.ico
//! cargo run --example png_to_ico -- logo.png logo.ico 16 32 48 64 128 256
//! ```
//! 省略尺寸时用默认 16/32/48/64/128/256；源图非正方形会等比缩放并居中留透明边。

use std::process::ExitCode;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() < 2 {
        eprintln!("用法: png_to_ico <输入.png> <输出.ico> [尺寸...]");
        return ExitCode::from(2);
    }

    let sizes = match args[2..]
        .iter()
        .map(|s| s.parse::<u32>())
        .collect::<Result<Vec<u32>, _>>()
    {
        Ok(sizes) => sizes,
        Err(e) => {
            eprintln!("尺寸必须是正整数（像素）：{e}");
            return ExitCode::from(2);
        }
    };

    match multiavatar::write_png_ico(&args[0], &args[1], &sizes) {
        Ok(()) => {
            let used = if sizes.is_empty() {
                "默认 16/32/48/64/128/256".to_string()
            } else {
                sizes
                    .iter()
                    .map(u32::to_string)
                    .collect::<Vec<_>>()
                    .join("/")
            };
            println!("已写入 {}（尺寸 {used}）", args[1]);
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("生成失败：{e}");
            ExitCode::FAILURE
        }
    }
}
