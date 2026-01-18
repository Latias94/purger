use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    // Default behavior: launch GUI if no subcommand is provided.
    if args.len() <= 1 {
        return purger_gui::run_gui();
    }

    match args[1].as_str() {
        "gui" | "--gui" | "-g" => purger_gui::run_gui(),
        _ => purger_cli::run_cli(),
    }
}
