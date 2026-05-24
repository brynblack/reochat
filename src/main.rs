fn main() -> anyhow::Result<()> {
    env_logger::init();
    reochat::run()?;
    Ok(())
}
