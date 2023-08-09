use std::fs::{self, File};
use std::io::{self, ErrorKind};

use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate<'a> {
    name: &'a str,
}

fn main() -> anyhow::Result<()> {
    fs::remove_dir("output").or_else(ignore_not_found)?;
    fs::create_dir("output")?;

    let mut index_file = File::create("output/index.html")?;
    let index = IndexTemplate { name: "world" };
    index.write_into(&mut index_file)?;

    // iterate over all the issue articles and generate the files.

    Ok(())
}

fn ignore_not_found(e: io::Error) -> io::Result<()> {
    if e.kind() == ErrorKind::NotFound {
        Ok(())
    } else {
        Err(e)
    }
}
