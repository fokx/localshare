<script lang="ts">

    import {writeText} from "@tauri-apps/plugin-clipboard-manager";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import QrCode from "svelte-qrcode";
    import {toast} from '@zerodevx/svelte-toast';
    import {Button, Hr, P} from 'svelte-5-ui-lib';

    let props = $props();
    let url = $derived(props.url);
    let show_qr = $state(false);
</script>
<P>
    <Button size="xs" color="lime" onclick={()=>{writeText(url); toast.push('link copied to clipboard'); }}>{url}</Button>
    <Button size="xs" onclick={()=>{openUrl(url);}}>Open</Button>
    <Button size="xs" onclick={()=>{show_qr=!show_qr;}}>Toggle QR Code</Button>
</P>
{#if show_qr}
    <div class="link-qr">
        <QrCode value={url}/>
    </div>
{/if}