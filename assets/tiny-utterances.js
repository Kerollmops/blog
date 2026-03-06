const fetchComments = async (
    repositoryOwner, repositoryName, issueNumber, maxNumberOfComments
) => {
  const response = await fetch(
      `https://api.github.com/repos/${repositoryOwner}/${repositoryName}/issues/${issueNumber}/comments` +
      `?per_page=${maxNumberOfComments}`,
      {
          method: "GET",
          headers: {
              "Accept": "application/vnd.github.html+json"
          }
      }
  );

  if (response.status == 200) {
      return response.json();
  }

  throw new Error("Unexpected status: " + response.status);
}

const renderComment = comment => {
  const createdAt = new Date(comment.created_at).toLocaleString('en-US', { dateStyle: "medium" });

  return `<div class="tu-comment">
    <div class="tu-header">
        <img class="tu-avatar" src=${comment.user.avatar_url} />
        <span class="tu-login"><a class="user-mention" href="${comment.user.html_url}">${comment.user.login}</a></span>
        <span class="tu-created-at">commented on <a href="${comment.html_url}">${createdAt}</a></span>
    </div>
    ${comment.body_html}
  </div>`;
}

const renderButton = (noComments, repoName, repoOwner, issueNumber) => {
  const text = noComments ? "Be the first to comment on GitHub" : "Join the discussion on GitHub";
  const url = `https://github.com/${repoOwner}/${repoName}/issues/${issueNumber}#comment-composer-heading`;

  return `<a class="tu-button" href="${url}">${text}</a>`;
}

var elements = document.querySelectorAll(".tiny-utterances");
elements.forEach(element => {
  const dataset = element.dataset;
  const repoOwner = dataset.repoOwner;
  const repoName = dataset.repoName;
  const issueNumber = Number(dataset.issueNumber);
  fetchComments(repoOwner, repoName, issueNumber, Number(dataset.maxComments)).then(comments => {
      const renderedComments = comments.map(renderComment).join("");
      const renderedJoinButton = renderButton(comments.length == 0, repoName, repoOwner, issueNumber);
      element.innerHTML = renderedComments + renderedJoinButton;
  }).catch(console.error);
});
