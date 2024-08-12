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
