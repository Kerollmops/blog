// Set theme to the user's preferred color scheme
function updateTheme() {
  const colorMode = window.matchMedia("(prefers-color-scheme: dark)").matches ?
    "dark" :
    "light";
  document.querySelector("html").setAttribute("data-bs-theme", colorMode);
}

document.addEventListener("DOMContentLoaded", function() {
  var symbolElements = document.querySelectorAll(".symbol");
  var symbols = ['&#x261C;', '&#x261E;', '&#x261F;'];
  symbolElements.forEach(function(spanElement) {
    spanElement.innerHTML = symbols[(Math.random() * symbols.length) | 0];
  });
});

// Set theme on load
updateTheme()

// Update theme when the preferred scheme changes
window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', updateTheme)
