<script lang="ts">
    // import {BaseDirectory, writeTextFile} from '@tauri-apps/plugin-fs';
    // import {enable, isEnabled} from '@tauri-apps/plugin-autostart';
    import {invoke, type PermissionState, PluginListener} from '@tauri-apps/api/core'
    import {toast} from "@zerodevx/svelte-toast";
    import {A, Button, ButtonGroup, Checkbox, Heading, Input, InputAddon} from 'svelte-5-ui-lib';
    import {EyeOutline, EyeSlashOutline, GithubSolid} from 'flowbite-svelte-icons';
    import {load} from '@tauri-apps/plugin-store';
    import {onMount} from "svelte";
    import {listenForShareEvents, type ShareEvent} from 'tauri-plugin-sharetarget-api';
    import {exists, mkdir, readFile, writeFile} from "@tauri-apps/plugin-fs";
    import * as path from '@tauri-apps/api/path';
    import { warn, debug, trace, info, error } from '@tauri-apps/plugin-log';

    let peers_store;
    let peers = $state();
    onMount(async () => {
        peers_store = await load('peers.json');
        /*
        #[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Message {
    pub alias: String,
    pub version: String,
    pub device_model: Option<String>,
    pub device_type: Option<String>,
    pub fingerprint: String,
    pub port: u16,
    pub protocol: String,
    pub download: Option<bool>,
    pub announce: bool,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct PeerInfo {
    pub message: Message,
    pub remote_addrs: Vec<SocketAddr>,
}
         */
        const val = await peers_store.get<Array<{
            message: {
                alias: string;
                version: string;
                device_model: string|null;
                device_type: string|null;
                fingerprint: string;
                port: number;
                protocol: string;
                download: boolean|null;
                announce: boolean|null;
            },
            remote_addrs: Array<{
                ip: string;
                port: number;
            }>
        }>>('cfg');
        if (val !== undefined) {
            console.log(peers);
            debug(peers);
        }
        peers = val;
        // Enable autostart
        // await enable();
        // Check enable state
        // console.log(`registered for autostart? ${await isEnabled()}`);
        // Disable autostart
        // disable();
        // import tauriapi from '@tauri-apps/api';
        // const { taurishell } = tauriapi.shell;
        // const command = Command.sidecar('binaries/tcc-xapp-hhk', []);
        // const response = await command.execute();
        // console.log(response);
    })
</script>
<Heading tag="h2" class="text-primary-700 dark:text-primary-500">
LocalSend in Rust
    {$inspect(peers)}
</Heading>
