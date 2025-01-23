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
