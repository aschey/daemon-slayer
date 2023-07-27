use daemon_slayer::build_info::vergen::EmitBuilder;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    EmitBuilder::builder()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .all_sysinfo()
        .emit()?;
    Ok(())
}
