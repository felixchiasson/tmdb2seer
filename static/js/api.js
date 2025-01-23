const HIDDEN_MEDIA_KEY = "hidden_media";

function getHiddenMedia() {
  const hidden = localStorage.getItem(HIDDEN_MEDIA_KEY);
  return hidden ? JSON.parse(hidden) : [];
}

function addToHiddenMedia(mediaType, id) {
  const hidden = getHiddenMedia();
  hidden.push({ mediaType, id });
  localStorage.setItem(HIDDEN_MEDIA_KEY, JSON.stringify(hidden));
}

function isHidden(mediaType, id) {
  const hidden = getHiddenMedia();
  return hidden.some((item) => item.mediaType === mediaType && item.id === id);
}

async function hideMedia(mediaType, id) {
  try {
    addToHiddenMedia(mediaType, id);
    showNotification("Media hidden successfully", "success");
    return true;
  } catch (error) {
    console.error("Failed to hide media:", error);
    showNotification("Failed to hide media", "error");
    return false;
  }
}

async function fetchFromAPI(endpoint, options = {}) {
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

async function requestMedia(mediaType, id) {
  const response = await fetchFromAPI(`/api/request/${mediaType}/${id}`, {
    method: "POST",
  });

  if (response.success) {
    showNotification(
      response.message || "Media requested successfully",
      "success",
    );
  } else {
    throw new Error(response.error || "Request failed");
  }
}
