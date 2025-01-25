import { API } from "./modules/API.js";
import { NotificationManager } from "./modules/NotificationManager.js";
import { ReleaseManager } from "./modules/ReleaseManager.js";

class App {
  constructor() {
    this.setupEventListeners();
    this.initializeReleases();
    this.formatInitialDate();
  }

  setupEventListeners() {
    document.addEventListener("DOMContentLoaded", () => {
      this.setupRefreshButton();
      this.initializeCards();
      this.setupReleaseCards();
    });
  }

  formatInitialDate() {
    const lastUpdateEl = document.getElementById("lastUpdate");
    if (lastUpdateEl) {
      const currentText = lastUpdateEl.textContent;
      const dateString = currentText.replace("Last updated: ", "");
      const date = new Date(dateString);
      lastUpdateEl.textContent = `Last updated: ${date.toLocaleString()}`;
    }
  }

  initializeCards() {
    const container = document.getElementById("releases-container");
    if (!container) return;

    try {
      // Get the releases data from the data attribute
      const releasesData = JSON.parse(container.dataset.releases || "[]");

      // Set CSRF token for later use
      window.CSRF_TOKEN = container.dataset.csrfToken;

      // Filter out hidden releases
      const visibleReleases = releasesData.filter(
        (release) => !ReleaseManager.isHidden(release.media_type, release.id),
      );

      // Create and insert cards
      container.innerHTML = visibleReleases
        .map((release) => ReleaseManager.createReleaseCard(release))
        .join("");

      // Setup button handlers
      this.setupReleaseCards();
    } catch (error) {
      console.error("Failed to initialize cards:", error);
      NotificationManager.show("Failed to initialize cards", "error");
    }
  }

  setupRefreshButton() {
    const refreshButton = document.getElementById("refreshButton");
    if (refreshButton) {
      refreshButton.addEventListener("click", () => this.refreshData());
    }
  }

  async refreshData() {
    const button = document.getElementById("refreshButton");
    const lastUpdateEl = document.getElementById("lastUpdate");
    const releasesContainer = document.getElementById("releases-container");

    button.disabled = true;
    button.textContent = "Refreshing...";

    try {
      const data = await API.refreshData();

      if (data.success) {
        const visibleReleases = data.releases.filter(
          (release) => !ReleaseManager.isHidden(release.media_type, release.id),
        );

        lastUpdateEl.textContent = `Last updated: ${new Date(data.lastUpdate).toLocaleString()}`;
        releasesContainer.innerHTML = visibleReleases
          .map((release) => ReleaseManager.createReleaseCard(release))
          .join("");

        this.setupReleaseCards();
        NotificationManager.show("Data refreshed successfully", "success");
      } else {
        throw new Error(data.error || "Refresh failed");
      }
    } catch (error) {
      console.error("Refresh failed:", error);
      NotificationManager.show(
        "Failed to refresh data: " + error.message,
        "error",
      );
    } finally {
      button.disabled = false;
      button.textContent = "Refresh Data";
    }
  }

  setupReleaseCards() {
    this.setupRequestButtons();
    this.setupHideButtons();
  }

  setupRequestButtons() {
    document.querySelectorAll(".request-button").forEach((button) => {
      button.addEventListener("click", async (event) => {
        const button = event.target;
        const card = button.closest(".release-card");
        const { mediaType, id } = button.dataset;

        try {
          button.disabled = true;
          button.textContent = `Requesting ${mediaType === "tv" ? "Season" : "Movie"}...`;

          let seasons = null;
          if (mediaType === "tv") {
            const seasonSelect = card.querySelector(".season-select");
            if (seasonSelect) {
              seasons = seasonSelect.value;
            }
          }

          await API.requestMedia(mediaType, parseInt(id), seasons);
          NotificationManager.show(
            `${mediaType === "tv" ? "TV Show" : "Movie"} requested successfully`,
            "success",
          );

          card.style.transition = "all 0.3s ease";
          card.style.opacity = "0";
          card.style.transform = "scale(0.9)";

          setTimeout(() => card.remove(), 300);
        } catch (error) {
          console.error("Request failed:", error);
          button.disabled = false;
          button.textContent = `Request ${mediaType === "tv" ? "Season" : "Movie"}`;
          NotificationManager.show(
            `Failed to request ${mediaType === "tv" ? "TV Show" : "Movie"}: ${error.message}`,
            "error",
          );
        }
      });
    });
  }

  setupHideButtons() {
    document.querySelectorAll(".hide-button").forEach((button) => {
      button.addEventListener("click", async (event) => {
        const button = event.target;
        const card = button.closest(".release-card");
        const { mediaType, id } = button.dataset;

        try {
          button.disabled = true;
          button.textContent = "Hiding...";

          ReleaseManager.addToHiddenMedia(mediaType, parseInt(id));
          NotificationManager.show("Media hidden successfully", "success");

          card.style.transition = "all 0.3s ease";
          card.style.opacity = "0";
          card.style.transform = "scale(0.9)";

          setTimeout(() => card.remove(), 300);
        } catch (error) {
          console.error("Hide failed:", error);
          button.disabled = false;
          button.textContent = "Hide";
        }
      });
    });
  }

  initializeReleases() {
    const releasesContainer = document.getElementById("releases-container");
    if (releasesContainer) {
      const cards = Array.from(
        releasesContainer.getElementsByClassName("release-card"),
      );

      cards.forEach((card) => {
        const requestButton = card.querySelector(".request-button");
        const id = parseInt(card.dataset.id);
        const mediaType = requestButton.dataset.mediaType;

        if (ReleaseManager.isHidden(mediaType, id)) {
          card.remove();
        }
      });
    }
  }
}

// Initialize the app
new App();
