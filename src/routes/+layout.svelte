<script lang="ts">
    import { onMount } from "svelte";
    import { applyTheme, loadAppSettings } from "$lib/appSettings";
    import { applyClientProfile } from "$lib/clientProfile";

    onMount(() => {
        applyTheme(loadAppSettings().uiTheme);
        const cleanupClientProfile = applyClientProfile();
        const refreshTheme = () => applyTheme(loadAppSettings().uiTheme);
        window.addEventListener("tepub-settings-updated", refreshTheme);
        return () => {
            cleanupClientProfile();
            window.removeEventListener("tepub-settings-updated", refreshTheme);
        };
    });
</script>

<slot />
