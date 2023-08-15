use std::env;
use std::io::Cursor;
use std::path::Path;

use askama::Template;
use big_s::S;
use octocrab::models::timelines::Rename;
use octocrab::models::Event;
use octocrab::params::State;
use octocrab::{format_media_type, OctocrabBuilder};
use reqwest::IntoUrl;
use tokio::fs::{self, File};
use tokio::io::{self, ErrorKind};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let owner_repo = std::env::var("GITHUB_REPOSITORY").expect("please define `GITHUB_REPOSITORY`");
    let (owner, repo) = owner_repo.split_once('/').unwrap();

    fs::remove_dir_all("output").await.or_else(ignore_not_found)?;
    fs::create_dir("output").await?;
    fs::create_dir("output/assets").await?;

    // Copy the JS assets
    fs::copy("assets/script.js", "output/assets/script.js").await?;

    // force GitHub to return HTML content
    let octocrab = if let Ok(token) = env::var("GITHUB_TOKEN") {
        OctocrabBuilder::default()
            .add_header(http::header::ACCEPT, format_media_type("html"))
            .add_header(http::header::AUTHORIZATION, format!("Bearer {}", token))
            .build()?
    } else {
        OctocrabBuilder::default()
            .add_header(http::header::ACCEPT, format_media_type("html"))
            .build()?
    };

    let page = octocrab
        .issues(owner, repo)
        .list()
        .state(State::Open)
        .labels(&[S("article")])
        .per_page(50)
        .send()
        .await?;

    let mut articles = Vec::new();
    for issue in page {
        articles.push(ArticleInList {
            title: issue.title.clone(),
            url: correct_snake_case(&issue.title),
        });

        // But we must also create the redirection HTML pages to redirect
        // from the previous names of the article.
        let events = octocrab
            .issues(owner, repo)
            .list_timeline_events(issue.number)
            .per_page(100)
            .send()
            .await?;

        for event in events.into_iter().filter(|e| e.event == Event::Renamed) {
            if let Some(from_title) = event.rename.and_then(extract_from_field_from_rename) {
                create_and_write_into(
                    format!("output/{}.html", correct_snake_case(from_title)),
                    RedirectTemplate { redirect_url: correct_snake_case(&issue.title) },
                )
                .await?;
            }
        }

        // Then we create the article HTML pages. We must do that after the redirection
        // pages to be sure to replace the final HTML page by the article.
        create_and_write_into(
            format!("output/{}.html", correct_snake_case(&issue.title)),
            ArticleTemplate { title: issue.title, html_content: issue.body_html.unwrap() },
        )
        .await?;
    }

    create_and_write_into(
        "output/index.html",
        IndexTemplate { title: S("Kerollmops' blog"), articles },
    )
    .await?;

    fs::create_dir("output/style").await?;
    // Download starry-night for code-highlighting
    fetch_url(
        "https://raw.githubusercontent.com/wooorm/starry-night/2.1.1/style/both.css",
        "output/style/both.css",
    )
    .await?;

    Ok(())
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    title: String,
    articles: Vec<ArticleInList>,
}

struct ArticleInList {
    title: String,
    url: String,
}

#[derive(Template)]
#[template(path = "article.html", escape = "none")]
struct ArticleTemplate {
    title: String,
    html_content: String,
}

#[derive(Template)]
#[template(path = "redirect.html", escape = "none")]
struct RedirectTemplate {
    redirect_url: String,
}

fn correct_snake_case(s: impl AsRef<str>) -> String {
    use convert_case::{Boundary, Case, Converter};
    Converter::new()
        .remove_boundaries(&[Boundary::UpperLower, Boundary::LowerUpper])
        .to_case(Case::Kebab)
        .convert(s)
}

async fn create_and_write_into(
    path: impl AsRef<Path>,
    template: impl Template,
) -> anyhow::Result<()> {
    let mut article_file = File::create(path).await?.into_std().await;
    template.write_into(&mut article_file)?;
    Ok(())
}

async fn fetch_url(url: impl IntoUrl, file_name: impl AsRef<Path>) -> anyhow::Result<()> {
    let response = reqwest::get(url).await?;
    let mut file = File::create(file_name).await?;
    let mut content = Cursor::new(response.bytes().await?);
    io::copy(&mut content, &mut file).await?;
    Ok(())
}

fn ignore_not_found(e: io::Error) -> io::Result<()> {
    if e.kind() == ErrorKind::NotFound {
        Ok(())
    } else {
        Err(e)
    }
}

/// Because the Rename struct only has private field we are
/// forced to serialize/deserialize-trick to extract the from field, for now.
fn extract_from_field_from_rename(rename: Rename) -> Option<String> {
    match serde_json::to_value(rename).unwrap()["from"] {
        serde_json::Value::String(ref s) => Some(s.clone()),
        _ => None,
    }
}
