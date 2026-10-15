<script lang="ts">
    import { onMount } from "svelte";
    import { applyTheme, loadAppSettings } from "$lib/appSettings";
    import { applyClientProfile } from "$lib/clientProfile";
    import { platform } from "$lib/platform";
    import { listenForLaunchFiles } from "$lib/launchFiles";
    import { base } from "$app/paths";

    onMount(() => {
        applyTheme(loadAppSettings().uiTheme);
        const cleanupClientProfile = applyClientProfile();
        const refreshTheme = () => applyTheme(loadAppSettings().uiTheme);
        let disposed = false;
        let stopLaunchListener: (() => void) | undefined;
        void listenForLaunchFiles(platform, base)
            .then((cleanup) => {
                if (disposed) cleanup();
                else stopLaunchListener = cleanup;
            })
            .catch((error) => console.error("启动文件监听失败:", error));
        window.addEventListener("tepub-settings-updated", refreshTheme);
        return () => {
            disposed = true;
            stopLaunchListener?.();
            cleanupClientProfile();
            window.removeEventListener("tepub-settings-updated", refreshTheme);
        };
    });
</script>

<slot />
