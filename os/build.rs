// 引入文件系统操作和 I/O 相关模块
use std::fs::{File, read_dir};
use std::io::{Result, Write};

/// 构建脚本的主函数，由 Cargo 在编译前自动执行
/// 该函数的作用是：
/// 1. 设置重新构建的触发条件
/// 2. 调用 insert_app_data 函数生成应用程序链接文件
fn main() {
    // 告诉 Cargo，当用户程序源代码目录发生变化时重新运行构建脚本
    println!("cargo:rerun-if-changed=../user/src/");
    // 告诉 Cargo，当用户程序编译目标目录发生变化时重新运行构建脚本
    println!("cargo:rerun-if-changed={}", TARGET_PATH);
    // 执行应用程序数据插入操作，如果失败则 panic
    insert_app_data().unwrap();
}

/// 定义用户程序编译后的二进制文件存放路径
/// 该路径指向 RISC-V 64位架构的 release 版本编译输出目录
/// riscv64gc-unknown-none-elf 表示：
/// - riscv64gc: RISC-V 64位架构，支持 G 和 C 扩展指令集
/// - unknown: 未知厂商
/// - none: 裸机环境（无操作系统）
/// - elf: ELF 格式的可执行文件
static TARGET_PATH: &str = "../user/target/riscv64gc-unknown-none-elf/release/";

/// 应用程序数据插入函数
/// 该函数的主要功能是扫描用户程序目录，生成包含所有用户程序的汇编链接文件
///
/// 生成的 link_app.S 文件包含以下内容：
/// 1. _num_app 符号：存储用户程序的总数量
/// 2. 应用程序地址表：每个应用的起始和结束地址标识符
/// 3. 二进制数据：使用 .incbin 指令嵌入每个用户程序的二进制文件
///
/// 该文件会被内核链接时包含，使得内核能够在运行时访问和加载用户程序
///
/// 返回值：Result<()> - 成功时返回 Ok(())，失败时返回错误信息
fn insert_app_data() -> Result<()> {
    // 第一步：创建输出文件
    // 在内核的 src 目录下创建或覆盖 link_app.S 汇编文件
    let mut f = File::create("src/link_app.S").unwrap();

    // 第二步：扫描用户程序目录，获取所有应用程序名称
    // 从 ../user/src/bin 目录读取所有源文件，提取应用程序名称（去掉扩展名）
    // 这些名称将用于生成对应的符号和包含二进制文件
    let mut apps: Vec<_> = read_dir("../user/src/bin")
        .unwrap() // 如果目录不存在则 panic
        .into_iter()
        .map(|dir_entry| {
            // 获取文件名并转换为字符串
            let mut name_with_ext = dir_entry.unwrap().file_name().into_string().unwrap();
            // 移除文件扩展名，只保留应用程序名称
            // 例如：hello_world.rs -> hello_world
            name_with_ext.drain(name_with_ext.find('.').unwrap()..name_with_ext.len());
            name_with_ext
        })
        .collect();
    // 对应用程序名称进行排序，确保生成的符号顺序一致
    apps.sort();

    // 第三步：生成应用程序数量和地址表
    // 写入汇编代码头部，定义数据段对齐和全局符号
    writeln!(
        f,
        r#"
    .align 3
    .section .data
    .global _num_app
_num_app:
    .quad {}"#,
        apps.len()
    )?;

    // 生成应用程序起始地址表
    // 为每个应用程序生成一个起始地址符号引用
    // 例如：.quad app_0_start, .quad app_1_start, ...
    for i in 0..apps.len() {
        writeln!(f, r#"    .quad app_{}_start"#, i)?;
    }
    // 添加最后一个应用程序的结束地址，用于确定整个应用程序区域的边界
    writeln!(f, r#"    .quad app_{}_end"#, apps.len() - 1)?;

    // 第四步：为每个应用程序生成二进制数据段
    // 遍历所有应用程序，为每个应用生成对应的汇编代码段
    for (idx, app) in apps.iter().enumerate() {
        // 在构建时输出应用程序信息，便于调试和确认
        println!("app_{}: {}", idx, app);

        // 为每个应用程序生成独立的数据段
        // 包含起始标签、二进制数据包含指令、结束标签
        writeln!(
            f,
            r#"
    .section .data
    .global app_{0}_start
    .global app_{0}_end
    .align 3
app_{0}_start:
    .incbin "{2}{1}"
app_{0}_end:"#,
            idx, app, TARGET_PATH
        )?;
        // 参数解释：
        // {0} = idx: 应用程序索引号
        // {1} = app: 应用程序名称
        // {2} = TARGET_PATH: 二进制文件路径
        // .incbin 指令将指定路径的二进制文件直接嵌入到汇编输出中
    }

    // 函数执行成功，返回 Ok(())
    Ok(())
}
