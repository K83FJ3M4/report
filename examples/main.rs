use report::{Result, log, report, info, bail};
use std::fs::File;
use std::io::{BufRead, BufReader};

#[report]
#[log("Running experiments")]
fn main() {
    let path = "data.txt";
    
    #[report("Running task one on {path}")]
    task_one(path).ok();

    let path = "Cargo.toml";
    #[report("Running task two on {path}")]
    task_two(path).ok();
}

fn task_one(file: &str) -> Result {
    let _file = File::open(file)?; 
    bail!("File exists, even though it should not")
}

#[report]
fn task_two(file: &str) -> Result {
    let file = File::open(file)?;
    let metadata = file.metadata()?;

    info!("File size: {}", metadata.len());
    
    for line in BufReader::new(file).lines() {
        #[report("Reading line")]
        let line = line?;

        if line.starts_with("[") {
            info!("Found section: {line}");
        }
    }

    Ok(())
}
