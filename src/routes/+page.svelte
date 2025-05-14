<script lang="ts">
    // import {BaseDirectory, writeTextFile} from '@tauri-apps/plugin-fs';
    // import {enable, isEnabled} from '@tauri-apps/plugin-autostart';
    import {invoke, type PermissionState, PluginListener} from '@tauri-apps/api/core'
    import {toast} from "@zerodevx/svelte-toast";
    import {A, Card, ButtonGroup, Checkbox, Heading, Input, InputAddon, Button} from 'svelte-5-ui-lib';
    import {EyeOutline, EyeSlashOutline, GithubSolid} from 'flowbite-svelte-icons';
    import {load} from '@tauri-apps/plugin-store';
    import {onMount} from "svelte";
    import {listenForShareEvents, type ShareEvent} from 'tauri-plugin-sharetarget-api';
    import {exists, mkdir, readFile, writeFile} from "@tauri-apps/plugin-fs";
    import * as path from '@tauri-apps/api/path';
    import {warn, debug, trace, info, error} from '@tauri-apps/plugin-log';
    import { listen } from '@tauri-apps/api/event';

    let peers_store;
    let peers = $state([]);
    let announce_btn_disable = $state(false);
    async function refresh_peers() {
        let _peers = [];
        const keys = await peers_store.keys();
        for (const key of keys) {
            const val = await peers_store.get<Array<{
                message: {
                    alias: string;
                    version: string;
                    device_model: string | null;
                    device_type: string | null;
                    fingerprint: string;
                    port: number;
                    protocol: string;
                    download: boolean | null;
                    announce: boolean | null;
                },
                remote_addrs: Array<string>
            }>>(key);
            if (val !== undefined) {
                _peers.push(val);
                console.log(val);
            }
        }
        peers = _peers;
    }

    let incoming_request_exist = $state(false);
    let incoming_session = $state('');
    let incoming_request_files = $state([]);
    let incoming_request_peer = $state(null);
    onMount(async () => {
        peers_store = await load('peers.json');
        await refresh_peers();
        const unlisten_refresh_peers = await listen('refresh-peers', async (event) => {
            console.log('event: refresh-peers', event);
            await refresh_peers();
        });
        const unlisten_prepare_upload = await listen('prepare-upload', (event) => {
            console.log('event: prepare-upload', event);
            incoming_request_exist = true;
            let payload = event.payload;
            incoming_session = payload.sessionId;

            let prepareUploadRequest = payload.prepareUploadRequest;
            incoming_request_peer = prepareUploadRequest.info.alias + " (" + prepareUploadRequest.info.fingerprint.substring(0, 8) + "...)";
            const files = Object.values(prepareUploadRequest.files);
            console.log(files);
            incoming_request_files = files.map((file) => {
                console.log(file);
                // convert size to human readable format
                let size = file.size;
                let i = 0;
                const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
                while (size >= 1024 && i < sizes.length - 1) {
                    size /= 1024;
                    i++;
                }
                size = Math.round(size * 100) / 100;
                size = size + " " + sizes[i];
                let name = file.fileName;
                // if file length > 20, then omit middle and show suffix, limit to 25 chars
                if (name.length > 30) {
                    name = name.substring(0, 20) + "..." + name.substring(name.length - 10);
                }
                return `${name} (${size})`;
            });
        });
        return () => {
            unlisten_refresh_peers();
            unlisten_prepare_upload();
        };
    });

    async function announce_once() {
        // change the button color gradully to gray and then back to blue
        announce_btn_disable = true;
        setTimeout(() => {
            announce_btn_disable = false;
        }, 1000);
        await invoke("announce_once");
    }


</script>
<Heading tag="h2" class="text-primary-700 dark:text-primary-500">
    LocalSend in Rust
</Heading>

<div>
    <div class="mb-3">
        {#if incoming_request_exist}
            <p>
                {incoming_request_peer} want to send file(s) to you:
            </p>
            <p>
                Session: {incoming_session}
            </p>
            <ul>
                {#each incoming_request_files as file}
                    <li>{file}</li>
                {/each}
            </ul>
            <ButtonGroup>
                <Button color="green" onclick={() => {
                    invoke("accept_incoming_request", {peer: incoming_request_peer});
                    toast.push('Incoming request accepted');
                    incoming_request_exist = false;
                }}>Accept</Button>
                <Button color="red" onclick={() => {
                    invoke("reject_incoming_request", {peer: incoming_request_peer});
                    toast.push('Incoming request rejected');
                    incoming_request_exist = false;
                }}>Reject</Button>
            </ButtonGroup>
        {/if}
    </div>



    <Heading tag="h4" class="text-primary-700 dark:text-primary-500">
        Peer List
        <Button onclick={announce_once} disabled={announce_btn_disable}>Discover peers</Button>
    </Heading>
    {#if peers.length === 0}
        <p class="text-gray-500 dark:text-gray-400">
            No peers found
        </p>
    {:else}
        {#each peers as p}
            <Card padding="xs">
                <h5 class="font-bold">
                    {p.message.alias}
                </h5>
                <p class="leading-tight font-normal text-gray-700 dark:text-gray-400">
                    {#each p.remote_addrs as addr}
                        {addr} <br>
                    {/each}
                </p>
            </Card>
        {/each}
    {/if}
</div>


