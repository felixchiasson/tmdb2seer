export class ReleaseManager {
  static HIDDEN_MEDIA_KEY = "hidden_media";

  static getHiddenMedia() {
    const hidden = localStorage.getItem(this.HIDDEN_MEDIA_KEY);
    return hidden ? JSON.parse(hidden) : [];
  }

  static addToHiddenMedia(mediaType, id) {
    const hidden = this.getHiddenMedia();
    hidden.push({ mediaType, id });
    localStorage.setItem(this.HIDDEN_MEDIA_KEY, JSON.stringify(hidden));
  }

  static isHidden(mediaType, id) {
    const hidden = this.getHiddenMedia();
    return hidden.some(
      (item) => item.mediaType === mediaType && item.id === id,
    );
  }

  static createReleaseCard(release) {
    const formattedRating = Number(release.vote_average).toFixed(1);

    // Determine rating class
    let ratingClass;
    if (formattedRating >= 8) {
      ratingClass = "rating-high";
    } else if (formattedRating >= 5) {
      ratingClass = "rating-medium";
    } else {
      ratingClass = "rating-low";
    }

    // Create season options if it's a TV show
    let seasonOptions = "";
    if (release.media_type === "tv" && release.number_of_seasons) {
      for (let i = 1; i <= release.number_of_seasons; i++) {
        seasonOptions += `<option value="${i}">Season ${i}</option>`;
      }
      seasonOptions += '<option value="all">All Seasons</option>';
    }

    const ratingsHtml = `
            <div class="ratings-container">
                ${
                  release.imdb_rating
                    ? `<div class="rating imdb">
                        <span class="rating-label">IMDb:</span>
                        <span class="rating-value">${release.imdb_rating}</span>
                    </div>`
                    : ""
                }
                ${
                  release.metascore
                    ? `<div class="rating metascore">
                        <span class="rating-label">Metascore:</span>
                        <span class="rating-value">${release.metascore}</span>
                    </div>`
                    : ""
                }
                ${
                  release.rotten_tomatoes
                    ? `<div class="rating rt">
                        <span class="rating-label">RT:</span>
                        <span class="rating-value">${release.rotten_tomatoes}</span>
                    </div>`
                    : ""
                }
            </div>`;

    const hasRatings =
      release.imdb_rating || release.metascore || release.rotten_tomatoes;
    const ratingsSection = hasRatings ? ratingsHtml : "";

    return `
            <div class="release-card ${ratingClass}" data-id="${release.id}">
                <div class="image-wrapper">
                    <div class="rating-overlay">
                        ${formattedRating}
                    </div>
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
                    <div class="content">
                        <h3>${release.title}</h3>
                        <p>Release Date: ${release.release_date}</p>
                        <div class="description-container">
                                            <p class="description-text">${release.overview || "No description available."}</p>
                                            <button class="description-toggle">Show More</button>
                                        </div>
                        ${ratingsSection}
                        ${
                          release.media_type === "tv" &&
                          release.number_of_seasons
                            ? `
                            <div class="season-selector">
                                <label for="season-${release.id}">Season:</label>
                                <select id="season-${release.id}" class="season-select">
                                    ${seasonOptions}
                                </select>
                            </div>
                            `
                            : ""
                        }
                    </div>
                    <div class="button-group">
                        <button class="request-button"
                                data-media-type="${release.media_type}"
                                data-id="${release.id}">
                            Request ${release.media_type === "tv" ? "Season" : "Movie"}
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

  static setupDescriptionToggles() {
    document.querySelectorAll(".description-container").forEach((container) => {
      const text = container.querySelector(".description-text");
      const toggle = container.querySelector(".description-toggle");

      if (text.scrollHeight > text.clientHeight) {
        toggle.style.display = "block";

        toggle.addEventListener("click", () => {
          const isExpanded = container.classList.contains("expanded");
          container.classList.toggle("expanded");
          toggle.textContent = isExpanded ? "Show More" : "Show Less";
        });
      } else {
        toggle.style.display = "none";
      }
    });
  }
}
