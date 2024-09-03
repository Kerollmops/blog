use std::fs;
use std::sync::Arc;

use anyhow::Context;
use askama::Template;
use resvg::render;
use tiny_skia::{Pixmap, Transform};
use unicode_segmentation::UnicodeSegmentation;
use usvg::{ImageHrefResolver, ImageKind, Options, Tree};

use crate::Spans::*;

pub const WIDTH: u32 = 1200;
pub const HEIGHT: u32 = 630;

#[derive(Template)]
#[template(path = "blog-post-preview.svg", escape = "none")]
struct PreviewTemplate {
    username: String,
    publish_date: String,
    title_spans: Spans,
    comments_text: String,
}

enum Spans {
    One(String),
    Two(String, String),
    Three(String, String, String),
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let username = args.get(1).expect("missing `username` (first) argument").clone();
    let publish_date = args.get(2).expect("missing `publish_date` (second) argument").clone();
    let title = args.get(3).expect("missing `title` (third) argument").clone();
    let comments_count = args.get(4).expect("missing `comments_count` (fourth) argument");

    let comments_count: u32 = comments_count.parse()?;
    let comments_text = if comments_count > 1 {
        format!("{comments_count} comments")
    } else {
        format!("{comments_count} comment")
    };

    let title_spans = cut_title(&title);
    let template = PreviewTemplate { username, publish_date, title_spans, comments_text };
    let svg = template.to_string();

    // Create a new pixmap buffer to render to
    let mut pixmap = Pixmap::new(WIDTH, HEIGHT).context("Pixmap allocation error")?;

    // Use default settings
    let mut options = Options {
        dpi: 192.0,
        text_rendering: usvg::TextRendering::GeometricPrecision,
        shape_rendering: usvg::ShapeRendering::CrispEdges,
        image_href_resolver: ImageHrefResolver {
            resolve_string: Box::new(move |path: &str, _| {
                let response = ureq::get(path).call().ok()?;
                let content_type = response.header("content-type")?;
                match content_type {
                    "image/png" => {
                        let mut image_buffer = Vec::new();
                        response.into_reader().read_to_end(&mut image_buffer).ok()?;
                        Some(ImageKind::PNG(Arc::new(image_buffer)))
                    }
                    // ... excluding other content types
                    _ => None,
                }
            }),
            ..Default::default()
        },
        ..Default::default()
    };

    options.fontdb_mut().load_font_data(include_bytes!("../../Inter.ttc").to_vec());

    // Build our string into a svg tree
    let tree = Tree::from_str(&svg, &options)?;

    // Render our tree to the pixmap buffer, using default fit and transformation settings
    render(&tree, Transform::default(), &mut pixmap.as_mut());

    // Write the result
    fs::write("preview.png", pixmap.encode_png()?)?;

    Ok(())
}

fn cut_title(title: &str) -> Spans {
    const MAX_LINE_CHARS: usize = 28;

    let mut acc = 0;
    let mut previous_stop = 0;
    let mut parts = Vec::new();

    for (indice, word) in title.split_word_bound_indices() {
        if acc + word.len() > MAX_LINE_CHARS {
            parts.push(&title[previous_stop..indice]);
            previous_stop = indice;
            acc = 0;
        } else {
            acc += word.len();
        }
    }

    let remaining = &title[previous_stop..];
    if !remaining.is_empty() {
        parts.push(remaining);
    }

    match parts.len() {
        1 => Spans::One(parts[0].to_string()),
        2 => Spans::Two(parts[0].to_string(), parts[1].to_string()),
        _ => {
            let ellipsis = if parts.len() > 3 { "..." } else { "" };
            let part = format!("{}{ellipsis}", parts[2]);
            Spans::Three(parts[0].to_string(), parts[1].to_string(), part)
        }
    }
}
