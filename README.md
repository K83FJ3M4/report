# Report

[![MIT licensed][mit-badge]][mit-url]
[![Latest version](https://img.shields.io/crates/v/report.svg)](https://crates.io/crates/report)
[![Documentation](https://docs.rs/report/badge.svg)](https://docs.rs/report)

[mit-badge]: https://img.shields.io/badge/license-MIT-blue.svg
[mit-url]: https://github.com/K83FJ3M4/sync-lsp/blob/main/LICENSE

Report is a simple logging and error-reporting library. It is designed to be:
- **Simple**: There is almost no boilerplate required. It is not required to define error enums or implement traits.
- **Compatible**: This library will work with other libraries that don't use the custom `Result` or `Error` types from this crate.
- **Efficient**: Strings are only formatted when it is clear that they will actually be printed.

## Example

```rust
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
```

## Output

```text
╭───────────────────────────────────────────────────────────────────────────────────────╮
│ Running experiments                                                                   │
├─┬─────────────────────────────────────────────────────────────────────────────────────┤
│ ├── Running task one on data.txt                                                      │
│ │   ╰── error: No such file or directory (os error 2)                                 │
│ ╰── Running task two on Cargo.toml                                                    │
│     ├── info: File size: 552                                                          │
│     ├── info: Found section: [package]                                                │
│     ├── info: Found section: [workspace]                                              │
│     ├── info: Found section: [dependencies]                                           │
│     ╰── info: Found section: [features]                                               │
╰───────────────────────────────────────────────────────────────────────────────────────╯
```

## Feature Flags

| Flag | Description |
|------|-------------|
| `unicode` | Use unicode box drawing characters. |
| `color` | Use colors for the log level. |
| `frame` | Draw a frame around every report |