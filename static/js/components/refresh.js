document.addEventListener("DOMContentLoaded", () => {
  // Setup initial cards
  const releasesContainer = document.getElementById("releases-container");
  if (releasesContainer) {
    const cards = Array.from(releasesContainer.children);
    const releases = cards.map((card) => ({
      id: card.dataset.id,
      title: card.querySelector("h3").textContent,
      poster_url: card.querySelector("img").src,
      release_date: card
        .querySelector(".release-info p:nth-child(2)")
        .textContent.replace("Release Date: ", ""),
      media_type: card
        .querySelector(".release-info p:nth-child(3)")
        .textContent.replace("Type: ", ""),
      vote_average: parseFloat(
        card
          .querySelector(".release-info p:nth-child(4)")
          .textContent.split("(")[0]
          .replace("Rating: ", ""),
      ),
      vote_count: parseInt(
        card
          .querySelector(".release-info p:nth-child(4)")
          .textContent.match(/\((\d+)/)[1],
      ),
      tmdb_url: card.querySelector("a").href,
    }));

    releasesContainer.innerHTML = releases
      .map((release) => createReleaseCard(release))
      .join("");
  }

  // Add refresh button listener
  const refreshButton = document.getElementById("refreshButton");
  if (refreshButton) {
    refreshButton.addEventListener("click", refreshData);
  }

  // Setup request buttons
  setupRequestButtons();
});

async function refreshData() {
  const button = document.getElementById("refreshButton");
  const lastUpdateEl = document.getElementById("lastUpdate");
  const releasesContainer = document.getElementById("releases-container");

  // Add loading states
  button.disabled = true;
  button.textContent = "Refreshing...";
  button.classList.add("refreshing");
  releasesContainer.classList.add("releases-loading");

  try {
    const data = await fetchFromAPI("/api/refresh", {
      method: "POST",
    });

    if (data.success) {
      // Prepare new content but don't insert it yet
      const newContent = data.releases
        .map((release) => createReleaseCard(release))
        .join("");

      // Update timestamp
      lastUpdateEl.textContent = `Last updated: ${new Date(data.lastUpdate).toUTCString()}`;

      // Fade out current content
      releasesContainer.style.opacity = "0";

      // Wait for fade out
      await new Promise((resolve) => setTimeout(resolve, 300));

      // Update content
      releasesContainer.innerHTML = newContent;

      // Fade in new content
      releasesContainer.style.opacity = "1";

      // Setup new buttons
      setupRequestButtons();

      showNotification("Data refreshed successfully", "success");
    } else {
      throw new Error(data.error || "Refresh failed");
    }
  } catch (error) {
    console.error("Refresh failed:", error);
    showNotification("Failed to refresh data: " + error.message, "error");
  } finally {
    // Remove loading states
    button.disabled = false;
    button.textContent = "Refresh Data";
    button.classList.remove("refreshing");
    releasesContainer.classList.remove("releases-loading");
  }
}

function createReleaseCard(release) {
  return `
        <div class="release-card" data-id="${release.id}">
            <div class="image-wrapper">
                <img
                    src="${release.poster_url}"
                    alt="${release.title} poster"
                    loading="lazy"
                />
                <div class="tmdb-overlay">
                    <a
                        href="${release.tmdb_url}"
                        class="tmdb-link"
                        target="_blank"
                    >
                        View on TMDB
                    </a>
                </div>
            </div>
            <div class="release-info">
                <h3>${release.title}</h3>
                <p>Release Date: ${release.release_date}</p>
                <p>Type: ${release.media_type}</p>
                <p>
                    Rating: ${release.vote_average.toFixed(1)} (${release.vote_count}
                    votes)
                </p>
                <button
                    class="request-button"
                    data-media-type="${release.media_type}"
                    data-id="${release.id}"
                >
                    Request
                </button>
            </div>
        </div>
    `;
}

function setupRequestButtons() {
  document.querySelectorAll(".request-button").forEach((button) => {
    // Remove existing listeners to prevent duplicates
    button.replaceWith(button.cloneNode(true));

    // Get the fresh button reference after replacement
    const newButton = document.querySelector(
      `.request-button[data-id="${button.dataset.id}"]`,
    );

    // Add new listener with loading state
    newButton.addEventListener("click", async (event) => {
      const button = event.target;
      const card = button.closest(".release-card");
      const { mediaType, id } = button.dataset;

      try {
        button.disabled = true;
        button.textContent = "Requesting...";
        card.classList.add("loading");

        await requestMedia(mediaType, id);

        button.textContent = "Requested";
        button.classList.add("requested");
      } catch (error) {
        button.textContent = "Request Failed";
        console.error("Request failed:", error);
      } finally {
        card.classList.remove("loading");
      }
    });
  });
}
