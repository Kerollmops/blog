# blog
My personal blog website

## Simple Usage

You must first specify the GitHub repository identifier by using the `GITHUB_REPOSITORY` var, the `EMAIL_ADDRESS`, and preferably your `GITHUB_TOKEN`. You can find more information on the token on [the action it is used by](https://github.com/peaceiris/actions-gh-pages).

```bash
export GITHUB_REPOSITORY=Kerollmops/blog
export EMAIL_ADDRESS=your-wonderful-email-address
export GITHUB_TOKEN=your-github-token
```

Once you are ready you can run the program and you'll notice a new `output/` folder with all the files.

```bash
cargo run
```

## Advanced Tricks

### Defining the Post Synopsis

This blog tool uses HTML comments to let you define the synopsis visible on the homepage of the blog website.

```markdown
<!-- This will not be visible in the article but will be used as the synopsis on the main page -->

This is the first sentence of my blog post and this will only be visible in the article.
```

### Using Tiny-Utterances to Display Comments

I decided to use [tiny-utterances to display the user comments](https://cofx22.github.io/tiny-utterances/) under the blog post. It's a [simplified version of Utterances](https://utteranc.es/) and works great. The only thing is the hardcore GitHub rate-limiting on the API.
