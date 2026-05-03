<script lang="ts">
    import { onMount } from "svelte";

    onMount(() => {
        let theme = "modern";
        try {
            const stored = localStorage.getItem("app-settings");
            if (stored) {
                const settings = JSON.parse(stored);
                if (settings.uiTheme === "classic" || settings.uiTheme === "dark") {
                    theme = settings.uiTheme;
                }
            }
        } catch (_) {
            // ignore parse errors
        }
        document.documentElement.setAttribute("data-theme", theme);
    });
</script>

<slot />
