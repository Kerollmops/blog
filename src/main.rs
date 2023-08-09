use askama::Template;
use big_s::S;
use octocrab::params::State;
use octocrab::{format_media_type, OctocrabBuilder};
use tokio::fs::{self, File};
use tokio::io::{self, ErrorKind};

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let owner_repo = std::env::var("GITHUB_REPOSITORY").expect("please define `GITHUB_REPOSITORY`");
    let (owner, repo) = owner_repo.split_once('/').unwrap();

    fs::remove_dir_all("output").await.or_else(ignore_not_found)?;
    fs::create_dir("output").await?;

    // force GitHub to return HTML content
    let octocrab = OctocrabBuilder::default()
        .add_header(http::header::ACCEPT, format_media_type("html"))
        .build()?;

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
        articles
            .push(ArticleInList { title: issue.title.clone(), url: format!("{}", issue.number) });

        let mut article_file =
            File::create(format!("output/{}.html", issue.number)).await?.into_std().await;
        let article =
            ArticleTemplate { title: issue.title, html_content: issue.body_html.unwrap() };
        article.write_into(&mut article_file)?;
    }

    let mut index_file = File::create("output/index.html").await?.into_std().await;
    let index = IndexTemplate { title: S("Kerollmops' blog"), articles };
    index.write_into(&mut index_file)?;

    Ok(())
}

fn ignore_not_found(e: io::Error) -> io::Result<()> {
    if e.kind() == ErrorKind::NotFound {
        Ok(())
    } else {
        Err(e)
    }
}
