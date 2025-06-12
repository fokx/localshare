<script lang="ts">
    // import {BaseDirectory, writeTextFile} from '@tauri-apps/plugin-fs';
    // import {enable, isEnabled} from '@tauri-apps/plugin-autostart';
    import {invoke, type PermissionState, PluginListener} from '@tauri-apps/api/core'
    import {toast} from "@zerodevx/svelte-toast";
    import {A, Card, ButtonGroup, Checkbox, Heading, Input, InputAddon, Button, Listgroup} from 'flowbite-svelte';
    import {load, type Store} from '@tauri-apps/plugin-store';
    import {onMount} from "svelte";
    import {listenForShareEvents, type ShareEvent} from 'tauri-plugin-sharetarget-api';
    import {exists, mkdir, readFile, writeFile, copyFile} from "@tauri-apps/plugin-fs";
    import * as path from '@tauri-apps/api/path';
    import {warn, debug, trace, info, error} from '@tauri-apps/plugin-log';
    import {emit, listen} from '@tauri-apps/api/event';
    import { open } from '@tauri-apps/plugin-dialog';
    import Database from 'plugin-sql';

    import {generateRandomString} from "$lib";
    let settings_store: Store<any>;
    let current_settings;
    let peers_store: Store<any>;
    let peers = $state([]);
    import {siteTitle} from "$lib";

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
    let incoming_sessionId = $state('');
    let incoming_request_files = $state([]);
    let incoming_request_peer = $state(null);
    onMount(async () => {
        settings_store = await load('settings.json', {autoSave: true});
        current_settings = await settings_store.get('localsend');
        savingDir = current_settings.savingDir;
        fingerprint = current_settings.fingerprint;
        siteTitle.set("LocalSend (" + fingerprint.substring(0, 8) + "...)");

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
            incoming_sessionId = payload.sessionId;

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

    async function reconfigure_localsend() {
        current_settings.savingDir = savingDir;
        await settings_store.set('localsend', current_settings);
        toast.push('Configuration saved');
    }
    let selected_files = $state([]);
    let selected_files_names = $state([]);
    async function select_files() {
        const files = await open({
            multiple: true,
            directory: false,
        });
        if (files === null) {
            return;
        }
        console.log(files);
        selected_files = files;
        selected_files_names = files.map((file) => {
            try{
                let tmp = file.substring(file.lastIndexOf('/') + 1);
                return decodeURIComponent(tmp)
            } catch (e) {
                return file;
            }
        });
    }
    let savingDir = $state("/storage/emulated/0/");
    let fingerprint = $state("");
    async function acquire_permission_android(event: Event) {
        event.preventDefault();
        invoke('acquire_permission_android')
            .then((res) =>
                console.log(res)
            )
            .catch((e) => console.error(e));
    }

</script>
<!--<Heading tag="h2" class="text-primary-700 dark:text-primary-500">-->
<!--     ({fingerprint.substring(0, 8)+"..."})-->
<!--</Heading>-->

    <div class="mb-3">
        {#if incoming_request_exist}
            <p>
                {incoming_request_peer} want to send file(s) to you:
            </p>
            <p>
                Session: {incoming_sessionId}
            </p>
            <ul>
                {#each incoming_request_files as file}
                    <li>{file}</li>
                {/each}
            </ul>
            <ButtonGroup>
                <Button color="green" onclick={async () => {
                    await reconfigure_localsend();
                    await invoke("handle_incoming_request", {sessionId: incoming_sessionId, accept: true});
                    // emit('accept-upload', {accept: true});
                    // emit('accept-upload', {sessionId: incoming_sessionId, accept: true});
                    toast.push('Incoming request accepted');
                    incoming_request_exist = false;
                }}>Accept</Button>
                <Button color="red" onclick={async () => {
                    await reconfigure_localsend();
                    await invoke("handle_incoming_request", {sessionId: incoming_sessionId, accept: false});
                    // emit('accept-upload', {accept: false});
                    // emit('accept-upload', {sessionId: incoming_sessionId, accept: false});
                    toast.push('Incoming request rejected');
                    incoming_request_exist = false;
                }}>Reject</Button>
            </ButtonGroup>
        {/if}
    </div>

    <form class="mb-4" onsubmit={reconfigure_localsend}>
        <div class="mb-6">
            <div>
                <label class="black dark:bg-black" for="server_port">Saving Directory</label>
                <Input id="server_port" type="text" placeholder="where to save incoming files"
                       bind:value={savingDir}/>
            </div>

        </div>
        <Button onclick={select_files}>Select File(s)</Button>
        <Button class="toggle_button" type="submit">Reconfigure</Button>
    </form>

    <div>
        {#if selected_files && selected_files.length > 0}
            <Heading tag="h4" >
                Selected Files
            </Heading>
        {/if}
        <div>
            <Listgroup items={(selected_files_names)} liClass="w-full"/>
        </div>

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
            <Card class="max-w-[vw] p-6 ms-0.5 me-0.5 dark:text-gray-200">
                <div class="flex items-center space-x-4 py-2 rtl:space-x-reverse ">
                    <div class="min-w-0 flex-1">
                        <h5 class="font-bold">
                            {p.message.alias} ( {p.message.fingerprint.substring(0, 8)+"..."} )
                        </h5>
                        <p class="leading-tight font-normal">
                            {#each p.remote_addrs as addr}
                                {addr} <br>
                            {/each}
                        </p>
                    </div>
                    <div class="inline-flex items-center text-base font-semibold">
                        {#if selected_files && selected_files.length > 0}
                            <Button onclick={async () => {
                                await invoke("send_file_to_peer", {peer_fingerprint: p.message.fingerprint, files: selected_files});
                                toast.push('File(s) sharing reqeust sent');
                            }}>Send</Button>
                        {/if}
                    </div>
                </div>
            </Card>
        {/each}
    {/if}


<div>
    <hr>
    <p>
        On Android, you have to acquire permission before first use:
        <Button class="ms-2" onclick={acquire_permission_android}>
            Acquire Permission
        </Button>
    </p>

</div>



