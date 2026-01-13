<script lang="ts">
    import { onMount } from 'svelte';
    import { env } from "$env/dynamic/public"

    let message: string | undefined; // Declare a variable to store the message

    onMount(async () => {
        try {
            // Make a fetch request to the specified URL
            const response = await fetch(env.PUBLIC_SERVER_URL);

            // Check if the request was successful
            if (!response.ok) {
                throw new Error(`HTTP error! status: ${response.status}`);
            }

            // Parse the response as JSON
            const data = await response.json();

            // Get the 'message' property from the JSON and assign it to the reactive variable
            message = data.message;
        } catch (error) {
            console.error("Failed to fetch message:", error);
            // Optionally, handle the error by setting an error message
            message = `Error: ${error instanceof Error ? error.message : String(error)}`;
        }
    });
</script>

<div>
  <h1>{message}</h1>
</div>