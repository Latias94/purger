// 主包入口点 - 调用 purger-cli 的功能
// 用户通过 `cargo install purger` 安装后，可以使用 `purger` 命令

use anyhow::Result;

fn main() -> Result<()> {
    // 直接调用 purger-cli 的功能
    purger_cli::run_cli()
}


