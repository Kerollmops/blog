use std::collections::HashMap;
use std::env;
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};
use std::path::{Path, PathBuf};

use anyhow::Context;
use askama::Template;
use big_s::S;
use http::header::ACCEPT;
use octocrab::issues::IssueHandler;
use octocrab::models::reactions::ReactionContent;
use octocrab::models::timelines::Rename;
use octocrab::params::State;
use octocrab::{format_media_type, OctocrabBuilder};
use regex::Captures;
use rss::extension::atom::{AtomExtension, Link};
use rss::{Channel, Guid, Item};
use scraper::Html;
use serde::Deserialize;
use tokio::fs::{self, File};
use tokio::io::{self, ErrorKind};
use url::Url;

const GITHUB_BASE_URL: &str = "https://github.com/";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let owner_repo = std::env::var("GITHUB_REPOSITORY").expect("please define `GITHUB_REPOSITORY`");
    let email_address = std::env::var("EMAIL_ADDRESS").expect("please define `EMAIL_ADDRESS`");
    let (owner, repo) = owner_repo.split_once('/').unwrap();

    fs::remove_dir_all("output").await.or_else(ignore_not_found)?;
    fs::create_dir("output").await?;
    fs::create_dir("output/assets").await?;
    fs::create_dir("output/preview").await?;
    fs::create_dir("output/assets/keys").await?;

    // Copy the JS assets
    fs::copy("assets/preview/homepage.png", "output/preview/homepage.png").await?;
    fs::copy("assets/script.js", "output/assets/script.js").await?;
    fs::copy("assets/script.js", "output/assets/script.js").await?;
    fs::copy("assets/balls.js", "output/assets/balls.js").await?;
    fs::copy("assets/matter.min.js", "output/assets/matter.min.js").await?;
    fs::copy("assets/tiny-utterances.js", "output/assets/tiny-utterances.js").await?;
    fs::copy("assets/style.css", "output/assets/style.css").await?;
    fs::copy("assets/tiny-utterances.css", "output/assets/tiny-utterances.css").await?;
    fs::copy("assets/bootstrap.min.css", "output/assets/bootstrap.min.css").await?;
    fs::copy("assets/starry-night.css", "output/assets/starry-night.css").await?;

    // Copy the keys assets
    for key in ('A'..='Z').chain('0'..='9') {
        let src = format!("assets/keys/{key}.png");
        let dst = format!("output/assets/keys/{key}.png");
        fs::copy(src, dst).await?;
    }

    // force GitHub to return HTML content
    let octocrab = if let Some(token) = env::var("GITHUB_TOKEN").ok().filter(|s| !s.is_empty()) {
        eprintln!("I am authenticated!");
        OctocrabBuilder::default()
            .personal_token(token)
            .add_header(ACCEPT, format_media_type("full"))
            .build()?
    } else {
        eprintln!("I am not authenticated!");
        OctocrabBuilder::default().add_header(ACCEPT, format_media_type("full")).build()?
    };

    let user: User = octocrab::instance().get(format!("/users/{}", owner), None::<&()>).await?;
    let html_bio_owner = linkify_at_references(user.bio);

    let repository = octocrab::instance().repos(owner, repo).get().await?;
    let homepage = repository
        .homepage
        .context("You must set the homepage URL of your blog on the repository")?;
    let homepage_url = Url::parse(&homepage)?;

    let page = octocrab
        .issues(owner, repo)
        .list()
        .state(State::Open)
        .labels(&[S("article")])
        .per_page(50)
        .send()
        .await?;

    let mut items = Vec::new();
    let mut articles = Vec::new();
    for mut issue in page {
        let falback_date = issue.created_at;
        let body = issue.body.as_ref().unwrap();
        let issue_handler = octocrab.issues(owner, repo);
        let url = correct_dash_case(&issue.title);
        let synopsis = synopsis(body);

        if let Some(html) = issue.body_html {
            let (urls_to_path, html) = replace_img_srcs_with_hashes(html);
            issue.body_html = Some(html);

            let mut body_bytes = Vec::new();
            std::fs::create_dir_all("output/assets/images")?;
            for (url, path) in urls_to_path {
                body_bytes.clear();
                let resp = ureq::get(&url).call()?;
                resp.into_reader().read_to_end(&mut body_bytes)?;
                std::fs::write(Path::new("output").join(path), &body_bytes)?;
            }
        }

        // But we must also create the redirection HTML pages to redirect
        // from the previous names of the article.
        let events = issue_handler.list_timeline_events(issue.number).per_page(100).send().await?;

        let mut publish_date = None;
        for event in events {
            if let Some(from_title) = event.rename.and_then(extract_from_field_from_rename) {
                create_and_write_template_into(
                    format!("output/{}.html", correct_dash_case(from_title)),
                    RedirectTemplate { redirect_url: correct_dash_case(&issue.title) },
                )
                .await?;
            }
            if event.label.map_or(false, |e| e.name == "article") {
                publish_date = event.created_at;
            }
        }

        articles.push(ArticleInList {
            title: issue.title.clone(),
            synopsis: synopsis.clone(),
            url: url.clone(),
            publish_date: publish_date.unwrap_or(falback_date).format("%B %d, %Y").to_string(),
            comments_count: issue.comments,
            guest_user: Some(issue.user.login.clone()).filter(|u| !u.eq_ignore_ascii_case(owner)),
        });

        // Everytime we fetch an article we also fetch the author real name
        let author: User =
            octocrab::instance().get(format!("/users/{}", issue.user.login), None::<&()>).await?;
        let html_bio = linkify_at_references(author.bio);

        let mut profil_picture_url = author.avatar_url;
        profil_picture_url.set_query(Some("v=4&s=100"));
        let reaction_counts = collect_reactions(&issue_handler, issue.number).await?;

        items.push(Item {
            guid: Some(Guid { value: homepage_url.join(&url)?.to_string(), permalink: true }),
            title: Some(issue.title.clone()),
            link: Some(homepage_url.join(&url)?.to_string()),
            description: Some(synopsis.clone()),
            author: Some(format!("{email_address} ({})", author.name)),
            atom_ext: Some(AtomExtension {
                links: vec![Link {
                    rel: "related".into(),
                    href: homepage_url.join(&url)?.to_string(),
                    title: Some(issue.title.clone()),
                    ..Default::default()
                }],
            }),
            pub_date: Some(publish_date.as_ref().unwrap_or(&falback_date).to_rfc2822()),
            ..Default::default()
        });

        // We create the article HTML pages. We must do that after the redirection
        // pages to be sure to replace the final HTML page by the article.
        let post_dash_case = correct_dash_case(&issue.title);
        create_and_write_template_into(
            format!("output/{post_dash_case}.html"),
            ArticleTemplate {
                profil_picture_url,
                username: author.name.clone(),
                html_bio: html_bio.clone(),
                url: format!("{homepage}{post_dash_case}"),
                publish_date: publish_date.unwrap_or(falback_date).format("%B %d, %Y").to_string(),
                title: issue.title.clone(),
                description: synopsis,
                html_content: insert_table_class_to_table(insert_anchor_to_headers(
                    issue.body_html.unwrap(),
                )),
                comments_count: issue.comments,
                reaction_counts,
                owner: owner.to_string(),
                repository: repo.to_string(),
                issue_number: issue.number,
                preview_url: format!("{homepage}preview/{post_dash_case}.png"),
            },
        )
        .await?;

        // Generate the preview
        let preview_png = tokio::task::block_in_place(|| {
            let preview = blog::Preview {
                username: issue.user.login,
                publish_date: publish_date.unwrap_or(falback_date).format("%B %d, %Y").to_string(),
                title: issue.title.clone(),
                comment_count: issue.comments,
            };
            preview.generate_png().unwrap()
        });

        // And write it to disk
        tokio::fs::write(format!("output/preview/{post_dash_case}.png"), preview_png).await?;
    }

    let mut profil_picture_url = user.avatar_url;
    profil_picture_url.set_query(Some("v=4&s=100"));

    create_and_write_template_into(
        "output/index.html",
        IndexTemplate {
            profil_picture_url,
            username: user.name.clone(),
            description: "A chill and fun blog about Rust stuff and the journey of building my company: Meilisearch".to_string(),
            html_bio: html_bio_owner,
            url: homepage_url.clone(),
            preview_url: format!("{homepage}preview/homepage.png"),
            articles,
        },
    )
    .await?;

    let channel = Channel {
        title: format!("{}'s blog", user.name),
        items,
        link: homepage_url.to_string(),
        ..Default::default()
    };
    fs::write("output/atom.xml", channel.to_string())
        .await
        .context("writing into `output/feed.atom`")?;

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
    description: String,
    url: Url,
    preview_url: String,
    html_bio: String,
    articles: Vec<ArticleInList>,
}

struct ArticleInList {
    title: String,
    synopsis: String,
    url: String,
    publish_date: String,
    guest_user: Option<String>,
    comments_count: u32,
}

#[derive(Template)]
#[template(path = "article.html", escape = "none")]
struct ArticleTemplate {
    profil_picture_url: Url,
    username: String,
    owner: String,
    repository: String,
    issue_number: u64,
    html_bio: String,
    url: String,
    publish_date: String,
    title: String,
    description: String,
    html_content: String,
    preview_url: String,
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

fn insert_table_class_to_table(html: impl AsRef<str>) -> String {
    regex::Regex::new(r#"(<table) (role="table">)"#)
        .unwrap()
        .replace_all(html.as_ref(), r#"$1 class="table table-striped" $2"#)
        .into_owned()
}

fn insert_anchor_to_headers(html: impl AsRef<str>) -> String {
    regex::Regex::new(r#"<(h[234]) (.*)>(.*)</(h[234])>"#)
        .unwrap()
        .replace_all(html.as_ref(), |captures: &Captures| {
            assert_eq!(&captures[1], &captures[4]);
            let header = &captures[1];
            let header_attrs = &captures[2];
            let text = &captures[3];
            let dash_case = correct_dash_case(&captures[3]);
            format!(r##"<{header} id="{dash_case}" {header_attrs}><a href="#{dash_case}">{text}</a></{header}>"##)
        })
        .into_owned()
}

fn replace_img_srcs_with_hashes(html: impl AsRef<str>) -> (HashMap<String, PathBuf>, String) {
    use kuchiki::parse_html;
    use kuchiki::traits::*;

    let mut urls_to_local_path = HashMap::new();
    let document = parse_html().one(html.as_ref());

    for a_element in document.select("a > img").unwrap() {
        let a_node = a_element.as_node().parent().unwrap();
        let a_element_ref = a_node.as_element().unwrap();
        let img_element_ref = a_element.as_node().as_element().unwrap();
        let img_src =
            img_element_ref.attributes.borrow().get("src").map(|s| s.to_string()).unwrap();

        let local_path = hash_path_from_url(&img_src);
        urls_to_local_path.insert(img_src.to_string(), local_path.clone());
        a_element_ref.attributes.borrow_mut().insert("href", local_path.display().to_string());
        img_element_ref.attributes.borrow_mut().insert("src", local_path.display().to_string());
    }

    (urls_to_local_path, document.to_string())
}

fn hash_path_from_url(url: impl AsRef<str>) -> PathBuf {
    let hasher = BuildHasherDefault::<DefaultHasher>::default();
    let hash = hasher.hash_one(url.as_ref());
    let url = Url::parse(url.as_ref()).unwrap();
    let url_path = url.path();
    let path = PathBuf::new().join("assets").join("images").join(format!("{hash:x}"));
    match Path::new(url_path).extension() {
        Some(extension) => path.with_extension(extension.to_str().unwrap()),
        None => path.with_extension("png"),
    }
}

fn synopsis(s: impl AsRef<str>) -> String {
    let html = scraper::Html::parse_fragment(s.as_ref());
    fn get_first_html_comment(document: &Html) -> Option<&str> {
        for node in document.tree.nodes() {
            if let Some(comment) = node.value().as_comment() {
                return Some(comment);
            }
        }
        None
    }

    get_first_html_comment(&html).map_or_else(String::new, ToOwned::to_owned)
}

fn correct_dash_case(s: impl AsRef<str>) -> String {
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

    if output.ends_with('-') {
        output.pop();
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
    handler: &IssueHandler<'_>,
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

async fn create_and_write_template_into(
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
