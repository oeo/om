use super::{CatOutput, TreeOutput};
use std::error::Error;

pub fn output_tree(data: &TreeOutput) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}

pub fn output_cat(data: &CatOutput) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(data)?;
    println!("{}", json);
    Ok(())
}
