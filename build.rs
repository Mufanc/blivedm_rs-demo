use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    prost_build::compile_protos(
        &["src/live/proto/INTERACT_WORD_V2.proto"],
        &["src/live/proto"],
    )?;

    Ok(())
}
