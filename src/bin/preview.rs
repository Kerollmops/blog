use std::fs;

use blog::Preview;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let username = args.get(1).expect("missing `username` (first) argument").clone();
    let publish_date = args.get(2).expect("missing `publish_date` (second) argument").clone();
    let title = args.get(3).expect("missing `title` (third) argument").clone();
    let comments_count = args.get(4).expect("missing `comments_count` (fourth) argument");

    let comment_count: u32 = comments_count.parse()?;
    let preview = Preview { username, publish_date, title, comment_count };
    fs::write("preview.png", preview.generate_png()?)?;

    Ok(())
}
