export class API {
  static async fetchFromAPI(endpoint, options = {}) {
    const defaultOptions = {
      headers: {
        "X-CSRF-Token": window.CSRF_TOKEN,
        "Content-Type": "application/json",
      },
    };

    const finalOptions = {
      ...defaultOptions,
      ...options,
      headers: {
        ...defaultOptions.headers,
        ...(options.headers || {}),
      },
    };

    const response = await fetch(endpoint, finalOptions);

    if (!response.ok) {
      throw new Error(`API call failed: ${response.status}`);
    }

    return response.json();
  }

  static async requestMedia(mediaType, id) {
    return this.fetchFromAPI(`/api/request/${mediaType}/${id}`, {
      method: "POST",
    });
  }

  static async refreshData() {
    return this.fetchFromAPI("/api/refresh", {
      method: "POST",
    });
  }
}
