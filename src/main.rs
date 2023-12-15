use std::env;
use std::path::Path;

use anyhow::Context;
use askama::Template;
use big_s::S;
use http::header::{ACCEPT, AUTHORIZATION};
use octocrab::issues::IssueHandler;
use octocrab::models::reactions::ReactionContent;
use octocrab::models::timelines::Rename;
use octocrab::models::Event;
use octocrab::params::State;
use octocrab::{format_media_type, OctocrabBuilder};
use serde::Deserialize;
use tokio::fs::{self, File};
use tokio::io::{self, ErrorKind};
use url::Url;

const SYNOPSIS_LENGTH: usize = 200;
const GITHUB_BASE_URL: &str = "https://github.com/";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let owner_repo = std::env::var("GITHUB_REPOSITORY").expect("please define `GITHUB_REPOSITORY`");
    let (owner, repo) = owner_repo.split_once('/').unwrap();

    fs::remove_dir_all("output").await.or_else(ignore_not_found)?;
    fs::create_dir("output").await?;
    fs::create_dir("output/assets").await?;

    // Copy the JS assets
    fs::copy("assets/script.js", "output/assets/script.js").await?;
    fs::copy("assets/style.css", "output/assets/style.css").await?;
    fs::copy("assets/bootstrap.min.css", "output/assets/bootstrap.min.css").await?;
    fs::copy("assets/starry-night.css", "output/assets/starry-night.css").await?;

    // force GitHub to return HTML content
    let octocrab = if let Ok(token) = env::var("GITHUB_TOKEN") {
        OctocrabBuilder::default()
            .add_header(ACCEPT, format_media_type("html"))
            .add_header(AUTHORIZATION, format!("Bearer {}", token))
            .build()?
    } else {
        OctocrabBuilder::default().add_header(ACCEPT, format_media_type("html")).build()?
    };

    let author: User = octocrab::instance().get(format!("/users/{}", owner), None::<&()>).await?;
    let html_bio = linkify_at_references(author.bio);

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
        let date = issue.created_at.format("%B %d, %Y").to_string();
        let body_html = issue.body_html.as_ref().unwrap();

        articles.push(ArticleInList {
            title: issue.title.clone(),
            synopsis: synopsis(body_html),
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

        // Everytime we fetch are article we also fetch the author real name
        let author: User =
            octocrab::instance().get(format!("/users/{}", issue.user.login), None::<&()>).await?;

        let mut profil_picture_url = author.avatar_url;
        profil_picture_url.set_query(Some("v=4&s=100"));
        let reaction_counts = collect_reactions(octocrab.issues(owner, repo), issue.number).await?;

        // Then we create the article HTML pages. We must do that after the redirection
        // pages to be sure to replace the final HTML page by the article.
        create_and_write_into(
            format!("output/{}.html", correct_snake_case(&issue.title)),
            ArticleTemplate {
                profil_picture_url,
                username: author.name,
                html_bio: html_bio.clone(),
                date,
                title: issue.title,
                html_content: issue.body_html.unwrap(),
                article_comments_url: issue.html_url,
                comments_count: issue.comments,
                reaction_counts,
            },
        )
        .await?;
    }

    let mut profil_picture_url = author.avatar_url;
    profil_picture_url.set_query(Some("v=4&s=100"));

    create_and_write_into(
        "output/index.html",
        IndexTemplate { profil_picture_url, username: author.name, html_bio, articles },
    )
    .await?;

    Ok(())
}

#[derive(Deserialize)]
struct User {
    avatar_url: Url,
    name: String,
    bio: String,
}

#[derive(Template)]
#[template(path = "index.html", escape = "none")]
struct IndexTemplate {
    profil_picture_url: Url,
    username: String,
    html_bio: String,
    articles: Vec<ArticleInList>,
}

struct ArticleInList {
    title: String,
    synopsis: String,
    url: String,
}

#[derive(Template)]
#[template(path = "article.html", escape = "none")]
struct ArticleTemplate {
    profil_picture_url: Url,
    username: String,
    html_bio: String,
    date: String,
    title: String,
    html_content: String,
    article_comments_url: Url,
    comments_count: u32,
    reaction_counts: ReactionCounts,
}

#[derive(Template)]
#[template(path = "redirect.html", escape = "none")]
struct RedirectTemplate {
    redirect_url: String,
}

fn linkify_at_references(bio: impl AsRef<str>) -> String {
    regex::Regex::new(r"(@(\w+))")
        .unwrap()
        .replace_all(bio.as_ref(), format!("<a href=\"{GITHUB_BASE_URL}$2\">$1</a>"))
        .into_owned()
}

fn synopsis(s: impl AsRef<str>) -> String {
    let mut synopsis = String::new();
    for chunk in scraper::Html::parse_fragment(s.as_ref()).root_element().text() {
        synopsis.push_str(chunk);
        if synopsis.len() >= SYNOPSIS_LENGTH {
            break;
        }
    }
    synopsis
}

fn correct_snake_case(s: impl AsRef<str>) -> String {
    use slice_group_by::StrGroupBy;

    let mut output = String::new();
    for group in s.as_ref().linear_group_by_key(|x| x.is_ascii_alphanumeric()) {
        if let Some(x) = group.chars().next() {
            if x.is_alphanumeric() {
                output.extend(group.chars().map(|x| x.to_ascii_lowercase()));
            } else {
                output.push('-');
            }
        }
    }

    output
}

#[derive(Debug, Default)]
struct ReactionCounts {
    heart: usize,
    plus_one: usize,
    laugh: usize,
    confused: usize,
    hooray: usize,
    minus_one: usize,
    rocket: usize,
    eyes: usize,
}

async fn collect_reactions(
    handler: IssueHandler<'_>,
    issue_id: u64,
) -> anyhow::Result<ReactionCounts> {
    let mut output = ReactionCounts::default();

    for reaction in handler.list_reactions(issue_id).per_page(100).send().await? {
        match reaction.content {
            ReactionContent::Heart => output.heart += 1,
            ReactionContent::PlusOne => output.plus_one += 1,
            ReactionContent::Laugh => output.laugh += 1,
            ReactionContent::Confused => output.confused += 1,
            ReactionContent::Hooray => output.hooray += 1,
            ReactionContent::MinusOne => output.minus_one += 1,
            ReactionContent::Rocket => output.rocket += 1,
            ReactionContent::Eyes => output.eyes += 1,
        }
    }

    Ok(output)
}

async fn create_and_write_into(
    path: impl AsRef<Path>,
    template: impl Template,
) -> anyhow::Result<()> {
    let path = path.as_ref();
    let mut article_file = File::create(path)
        .await
        .with_context(|| format!("When opening {:?}", path.display()))?
        .into_std()
        .await;
    template.write_into(&mut article_file)?;
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
