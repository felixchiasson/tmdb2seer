document.addEventListener("DOMContentLoaded", () => {
  const releasesContainer = document.getElementById("releases-container");
  if (releasesContainer) {
    // Get all release cards
    const cards = Array.from(
      releasesContainer.getElementsByClassName("release-card"),
    );

    // Remove cards that are in hidden media
    cards.forEach((card) => {
      const mediaType = card.querySelector(".request-button").dataset.mediaType;
      const id = parseInt(card.dataset.id);

      if (isHidden(mediaType, id)) {
        card.remove();
      }
    });
  }

  setupButtons();
});

async function refreshData() {
  const button = document.getElementById("refreshButton");
  const lastUpdateEl = document.getElementById("lastUpdate");
  const releasesContainer = document.getElementById("releases-container");

  button.disabled = true;
  button.textContent = "Refreshing...";

  try {
    const data = await fetchFromAPI("/api/refresh", {
      method: "POST",
    });

    if (data.success) {
      // Filter out hidden media before creating cards
      const visibleReleases = data.releases.filter(
        (release) => !isHidden(release.media_type, release.id),
      );

      lastUpdateEl.textContent = `Last updated: ${new Date(data.lastUpdate).toUTCString()}`;
      releasesContainer.innerHTML = visibleReleases
        .map((release) => createReleaseCard(release))
        .join("");

      setupButtons();
      showNotification("Data refreshed successfully", "success");
    } else {
      throw new Error(data.error || "Refresh failed");
    }
  } catch (error) {
    console.error("Refresh failed:", error);
    showNotification("Failed to refresh data: " + error.message, "error");
  } finally {
    button.disabled = false;
    button.textContent = "Refresh Data";
  }
}

function createReleaseCard(release) {
  return `
        <div class="release-card" data-id="${release.id}">
            <div class="image-wrapper">
                <img src="${release.poster_url}"
                     alt="${release.title} poster"
                     loading="lazy">
                <div class="tmdb-overlay">
                    <a href="${release.tmdb_url}"
                       class="tmdb-link"
                       target="_blank">
                        View on TMDB
                    </a>
                </div>
            </div>
            <div class="release-info">
                <h3>${release.title}</h3>
                <p>Release Date: ${release.release_date}</p>
                <p>Type: ${release.media_type}</p>
                <p>Rating: ${release.vote_average} (${release.vote_count} votes)</p>
                <div class="button-group">
                    <button class="request-button"
                            data-media-type="${release.media_type}"
                            data-id="${release.id}">
                        Request
                    </button>
                    <button class="hide-button"
                            data-media-type="${release.media_type}"
                            data-id="${release.id}">
                        Hide
                    </button>
                </div>
            </div>
        </div>
    `;
}

// Hide buttons:

function setupButtons() {
  // Setup request buttons
  document.querySelectorAll(".request-button").forEach((button) => {
    // Remove existing listeners to prevent duplicates
    const newButton = button.cloneNode(true);
    button.parentNode.replaceChild(newButton, button);

    newButton.addEventListener("click", async (event) => {
      const button = event.target;
      const card = button.closest(".release-card");
      const { mediaType, id } = button.dataset;

      try {
        button.disabled = true;
        button.textContent = "Requesting...";

        await requestMedia(mediaType, id);

        // Animate and remove the card
        card.style.transition = "all 0.3s ease";
        card.style.opacity = "0";
        card.style.transform = "scale(0.9)";

        setTimeout(() => {
          card.remove();
        }, 300);
      } catch (error) {
        console.error("Request failed:", error);
        button.disabled = false;
        button.textContent = "Request";
        showNotification("Failed to request media: " + error.message, "error");
      }
    });
  });

  // Setup hide buttons
  document.querySelectorAll(".hide-button").forEach((button) => {
    const newButton = button.cloneNode(true);
    button.parentNode.replaceChild(newButton, button);

    newButton.addEventListener("click", async (event) => {
      const button = event.target;
      const card = button.closest(".release-card");
      const { mediaType, id } = button.dataset;

      try {
        button.disabled = true;
        button.textContent = "Hiding...";

        if (await hideMedia(mediaType, parseInt(id))) {
          card.style.transition = "all 0.3s ease";
          card.style.opacity = "0";
          card.style.transform = "scale(0.9)";

          setTimeout(() => {
            card.remove();
          }, 300);
        }
      } catch (error) {
        console.error("Hide failed:", error);
        button.disabled = false;
        button.textContent = "Hide";
      }
    });
  });
}
