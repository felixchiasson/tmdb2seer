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

    // Create season options if it's a TV show
    let seasonOptions = "";
    if (release.media_type === "tv" && release.number_of_seasons) {
      // Add individual season options
      for (let i = 1; i <= release.number_of_seasons; i++) {
        seasonOptions += `<option value="${i}">Season ${i}</option>`;
      }
      // Add "All Seasons" option at the end
      seasonOptions += '<option value="all">All Seasons</option>';
    }

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
                  <p>Rating: ${formattedRating} (${release.vote_count} votes)</p>
                  ${
                    release.media_type === "tv" && release.number_of_seasons
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
}
