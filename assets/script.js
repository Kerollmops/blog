// Set theme to the user's preferred color scheme
function updateTheme() {
  const colorMode = window.matchMedia("(prefers-color-scheme: dark)").matches ?
    "dark" :
    "light";
  document.querySelector("html").setAttribute("data-bs-theme", colorMode);
}

document.addEventListener("DOMContentLoaded", function() {
  var spanElement = document.querySelector("#symbol");
  var symbols = ['&#9982;', '&#9919;'];
  spanElement.innerHTML = symbols[(Math.random() * symbols.length) | 0];
});

// Set theme on load
updateTheme()

// Update theme when the preferred scheme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', updateTheme)
