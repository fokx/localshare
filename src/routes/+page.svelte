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
    onMount(async () => {
        peers_store = await load('peers.json');
        refresh_peers();
        const unlisten = await listen('refresh-peers', (event) => {
            console.log('event: refresh-peers', event);
            refresh_peers();
        });
        return () => {
            unlisten();
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
    <Button onclick={announce_once} disabled={announce_btn_disable}>Discover peers</Button>
    <Heading tag="h4" class="text-primary-700 dark:text-primary-500">
        Peer List:
    </Heading>
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
</div>


