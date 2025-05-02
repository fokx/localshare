<script lang="ts">

import {writeText} from "@tauri-apps/plugin-clipboard-manager";
import {openUrl} from "@tauri-apps/plugin-opener";
import QrCode from "svelte-qrcode";
import { toast } from '@zerodevx/svelte-toast';

let props = $props();
let url = $derived(props.url);
let show_qr = $state(false);
</script>
    <p>
        <button onclick={()=>{writeText(url); toast.push('link copied to clipboard'); }}>{url}</button>
        <button onclick={()=>{openUrl(url);}}>Open</button>
        <button onclick={()=>{writeText(url); toast.push('link copied to clipboard');}}>Copy</button>
        <button onclick={()=>{show_qr=!show_qr;}}>Toggle QR Code</button>
    </p>
{#if show_qr}
    <div class="link-qr">
        <QrCode value={url} />
    </div>
{/if}