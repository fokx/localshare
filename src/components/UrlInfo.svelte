<script lang="ts">

    import {writeText} from "@tauri-apps/plugin-clipboard-manager";
    import {openUrl} from "@tauri-apps/plugin-opener";
    import QRCode from '@castlenine/svelte-qrcode';
    import {toast} from '@zerodevx/svelte-toast';
    import {Button, Hr, P} from 'flowbite-svelte';

    let props = $props();
    let url = $derived(props.url);
    let require_auth = $derived(props.require_auth);
    let auth_user = $derived(props.auth_user);
    let auth_passwd = $derived(props.auth_passwd);
    let show_qr = $state(false);
</script>
<div>
    <P>
        <Button size="xs" color="lime" onclick={()=>{writeText(url); toast.push('link copied to clipboard'); }}>{url}</Button>
        <Button size="xs" onclick={()=>{openUrl(url);}}>Open</Button>
        <Button size="xs" onclick={()=>{show_qr=!show_qr;}}>Toggle QR Code</Button>
    </P>
    {#if show_qr}
        <div class="flex justify-center">
            {#if require_auth}
            <QRCode data={url.replace("http://", `http://${auth_user}:${auth_passwd}@`).replace("https://", `https://${auth_user}:${auth_passwd}@`)}/>
            {:else }
            <QRCode data={url}/>
                {/if}
        </div>
    {/if}
</div>
