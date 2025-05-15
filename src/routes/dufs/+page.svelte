<script lang="ts">
    // import {BaseDirectory, writeTextFile} from '@tauri-apps/plugin-fs';
    // import {enable, isEnabled} from '@tauri-apps/plugin-autostart';
    import {invoke, type PermissionState, PluginListener} from '@tauri-apps/api/core'
    import UrlInfo from '../../components/UrlInfo.svelte';
    import {toast} from "@zerodevx/svelte-toast";
    import {A, Button, ButtonGroup, Checkbox, Heading, Input, InputAddon} from 'flowbite-svelte';
    import {EyeOutline, EyeSlashOutline, GithubSolid} from 'flowbite-svelte-icons';
    import {load} from '@tauri-apps/plugin-store';
    import {onMount} from "svelte";
    import {listenForShareEvents, type ShareEvent} from 'tauri-plugin-sharetarget-api';
    import {exists, mkdir, readFile, writeFile} from "@tauri-apps/plugin-fs";
    import * as path from '@tauri-apps/api/path';

    let show_password = $state(false);

    // generate a random port number
    let server_port = $state(0);
    let serve_path = $state("/storage/emulated/0/");
    let require_auth = $state(true);
    let auth_user = $state("user");
    let auth_passwd = $state("User*1234");
    let allow_upload = $state(true);

    let server_running = $state(false);
    let server_host = $state("0.0.0.0");
    let toggle_disable = $state(false);
    let listening_urls: Array<string> = $state([]);

    interface Permissions {
        manageExternalStorage: PermissionState
    }

    function share_files() {

    }

    function share_folder() {

    }

    async function get_nic_info() {
        invoke('get_nic_info')
            .then((res) => {
                    // dedup
                    res = [...new Set(res)];
                    // show 192.168.x.x if exists first
                    res = res.sort((a, b) => {
                        if (a.includes('192.168') && !b.includes('192.168')) {
                            return -1;
                        } else if (!a.includes('192.168') && b.includes('192.168')) {
                            return 1;
                        } else {
                            return 0;
                        }
                    });
                    res = res.map(v => v + ":" + server_port);
                    console.log(res);
                    listening_urls = res;
                }
            )
            .catch((e) => console.error(e));
    }

    function is_str_invalid(s: string) {
        return s.includes("@") || s.includes(":") || s.includes(" ") || s.includes("/") || s.includes("\\");
    }

    async function reconfigure_server(event: Event) {
        event.preventDefault();
        toast.push('configuration saved');
        toggle_disable = true;
        if (server_port < 1024 || server_port > 65535) {
            toast.push('port number must be between 1024 and 65535', {
                theme: {
                    '--toastBackground': 'red',
                    '--toastColor': 'black',
                }
            });
        } else if (is_str_invalid(auth_user)) {
            toast.push('auth_user cannot contain @: /\\', {
                theme: {
                    '--toastBackground': 'red',
                    '--toastColor': 'black',
                }
            });
        } else if (is_str_invalid(auth_passwd)) {
            toast.push('auth_passwd cannot contain @: /\\', {
                theme: {
                    '--toastBackground': 'red',
                    '--toastColor': 'black',
                }
            });
        } else {
            await store.set('cfg', {
                server_port: server_port,
                serve_path: serve_path,
                require_auth: require_auth,
                auth_user: auth_user,
                auth_passwd: auth_passwd,
                allow_upload: allow_upload
            });
            if (server_running) {
                invoke('toggle_server', {
                    server_port: server_port,
                    serve_path: serve_path,
                    require_auth: require_auth,
                    auth_user: auth_user,
                    auth_passwd: auth_passwd, allow_upload: allow_upload
                })
                    .then((res) => {
                        console.log('res', res);
                        if (res === 'started') {
                            server_running = true;
                        } else if (res === 'stopped') {
                            server_running = false;
                        } else {
                            console.error('unknown response from server');
                        }
                    })
                    .catch((e) => console.error(e))
                    .finally(() => {
                        toggle_disable = false;

                    });
                // sleep 500 ms for server to shut down
                await new Promise(resolve => setTimeout(resolve, 500));
                invoke('toggle_server', {
                    server_port: server_port,
                    serve_path: serve_path,
                    require_auth: require_auth,
                    auth_user: auth_user,
                    auth_passwd: auth_passwd, allow_upload: allow_upload
                })
                    .then((res) => {
                        console.log('res', res);
                        if (res === 'started') {
                            server_running = true;
                        } else if (res === 'stopped') {
                            server_running = false;
                        } else {
                            console.error('unknown response from server');
                        }
                    })
                    .catch((e) => console.error(e))
                    .finally(() => {
                        toggle_disable = false;
                    });
            }
        }
        toggle_disable = false;
    }

    async function toggle_server(event: Event) {
        await get_nic_info();
        toggle_disable = true;
        event.preventDefault();
        invoke('toggle_server', {
            server_port: server_port,
            serve_path: serve_path,
            require_auth: require_auth,
            auth_user: auth_user,
            auth_passwd: auth_passwd, allow_upload: allow_upload
        })
            .then((res) => {
                console.log('res', res);
                if (res === 'started') {
                    server_running = true;
                } else if (res === 'stopped') {
                    server_running = false;
                } else {
                    console.error('unknown response from server');
                }
            })
            .catch((e) => console.error(e));
        toggle_disable = false;
    }

    async function acquire_permission_android(event: Event) {
        event.preventDefault();
        invoke('acquire_permission_android')
            .then((res) =>
                console.log(res)
            )
            .catch((e) => console.error(e));

// when using `"withGlobalTauri": true`, you may use
// const { open } = window.__TAURI__.dialog;

// Open a dialog
//     const file = await open({
//       multiple: false,
//       directory: false,
//     });
//     console.log(file);
// Prints file path or URI

        // Prints boolean to the console
        // Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
        // greetMsg += await invoke("collect_sys_info");
        //
        // const contents = await readFile(file);

        // greetMsg += file;
        // greetMsg += '\n';
        // greetMsg += contents.slice(0, 100).toString();
        // let ret = await invoke("greet", {server_port});
        // console.log('ret', ret);
        // greetMsg += ret;


// Open a file selection dialog
//     const filePath = await open({
//       directory: false,
//       multiple: false,
//     });

        // invoke('file_picker_example')
        // check permission state
        // const permission = await invoke<Permissions>('plugin:tauri_plugin_android_fs|checkPermissions')

        // if (permission.manageExternalStorage === 'prompt-with-rationale') {
        //   show information to the user about why permission is needed
        // }

        // request permission
        // if (permission.manageExternalStorage.includes('prompt')) {
        //   const state = await invoke<Permissions>('plugin:tauri_plugin_android_fs|requestPermissions', { permissions: ['manageExternalStorage'] })
        // }

        // write(file);
    }

    // let greetInputEl: HTMLInputElement | null;

    async function write(message: string) {
        // await writeTextFile('test.txt', message, {baseDir: BaseDirectory.Home});
    }

    let store;
    let file = $state<File | null>(null);
    let log_disp = $state("");
    // when using `"withGlobalTauri": true`, you may use
    // const { enable, isEnabled, disable } = window.__TAURI__.autostart;
    $effect(() => {
        let listener: PluginListener;
        const setupListener = async () => {
            listener = await listenForShareEvents(async (intent: ShareEvent) => {
                if (intent.stream) {
                    const contents = await readFile(intent.stream).catch((error: Error) => {
                        console.warn('fetching shared content failed:');
                        throw error;
                    });
                    // use date as folder name
                    const tmp_dir_name = await path.join(await path.cacheDir(), `single_file_share_${(new Date().toISOString())}`);
                    // console.log(await path.appConfigDir());
                    // console.log(await path.appDataDir());
                    // console.log(await path.appLocalDataDir());
                    // console.log(await path.cacheDir());
                    // console.log(await path.configDir());
                    // console.log('1');
                    // console.log(await path.dataDir());
                    // console.log(await path.localDataDir());
                    // console.log(await path.homeDir());
                    // console.log(await path.pictureDir());
                    // console.log(await path.resourceDir());
                    // console.log('2');
                    // console.log(await path.tempDir());
                    log_disp += "\n----------------------------------\n" + tmp_dir_name;
                    const dirExists = await exists(tmp_dir_name);
                    if (!dirExists) {
                        // await remove(tmp_dir_name, {
                        //     recursive: true,
                        // });
                        await mkdir(tmp_dir_name);
                    }
                    await writeFile(await path.join(tmp_dir_name, intent.name), contents);
                    // file = new File([contents], intent.name, { type: intent.content_type });
                    toggle_disable = true;
                    if (server_running) {
                        invoke('toggle_server', {
                            server_port: server_port,
                            serve_path: tmp_dir_name,
                            require_auth: require_auth,
                            auth_user: auth_user,
                            auth_passwd: auth_passwd, allow_upload: allow_upload
                        })
                            .then((res) => {
                                console.log('res', res);
                                if (res === 'started') {
                                    server_running = true;
                                } else if (res === 'stopped') {
                                    server_running = false;
                                } else {
                                    console.error('unknown response from server');
                                }
                            })
                            .catch((e) => console.error(e))
                            .finally(() => {
                                toggle_disable = false;

                            });
                    }
                    // sleep 500 ms for server to shut down
                    await new Promise(resolve => setTimeout(resolve, 500));
                    await get_nic_info();
                    invoke('toggle_server', {
                        server_port: server_port,
                        serve_path: tmp_dir_name,
                        require_auth: require_auth,
                        auth_user: auth_user,
                        auth_passwd: auth_passwd, allow_upload: allow_upload
                    })
                        .then((res) => {
                            console.log('res', res);
                            if (res === 'started') {
                                server_running = true;
                            } else if (res === 'stopped') {
                                server_running = false;
                            } else {
                                console.error('unknown response from server');
                            }
                        })
                        .catch((e) => console.error(e))
                        .finally(() => {
                            toggle_disable = false;
                        });
                } else {
                    // This intent contains no binary bundle.
                    log_disp = 'unable to share:\n' + intent.uri;
                    console.warn('unused share intent', intent.uri);
                }
                log_disp += "\n----------------------------------\n" + intent.name;
                log_disp += "\n----------------------------------\n" + intent.stream;
                log_disp += "\n----------------------------------\n" + intent.content_type;
                log_disp += "\n----------------------------------\n" + intent.uri;
            });
        };
        setupListener();
        return () => {
            // if a teardown function is provided, it will run
            // a) immediately before the effect re-runs
            // b) when the component is destroyed
            listener?.unregister();
        };
    });
    onMount(async () => {
        store = await load('settings.json', {autoSave: true});
        const val = await store.get<{
            server_port: number,
            serve_path: string,
            require_auth: boolean,
            auth_user: string,
            auth_passwd: string,
            allow_upload: boolean
        }>('cfg');
        if (val === undefined) {
            let _server_port = Math.floor(Math.random() * (65535 - 1024 + 1) + 1024);
            let _serve_path = "/storage/emulated/0/";
            let _require_auth = true;
            let _auth_user = "user";
            let _auth_passwd = "User*1234";
            let _allow_upload = true;
            await store.set('cfg', {
                server_port: _server_port,
                serve_path: _serve_path,
                require_auth: _require_auth,
                auth_user: _auth_user,
                auth_passwd: _auth_passwd,
                allow_upload: _allow_upload
            });
            server_port = _server_port;
            serve_path = _serve_path;
            require_auth = _require_auth;
            auth_user = _auth_user;
            auth_passwd = _auth_passwd;
            allow_upload = _allow_upload;
            await store.save()
        } else {
            server_port = val.server_port;
            serve_path = val.serve_path;
            require_auth = val.require_auth;
            auth_user = val.auth_user;
            auth_passwd = val.auth_passwd;
            allow_upload = val.allow_upload;
        }
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
    });

</script>

<Heading tag="h2" class="text-primary-700 dark:text-primary-500"
>Share file locally
    <Button class="ms-2" onclick={toggle_server} disabled={toggle_disable}>
        {#if server_running}stop{:else}start{/if} sharing
    </Button>
</Heading>

<div class="my-3">
    <form class="mb-4" onsubmit={reconfigure_server}>
        <div class="mb-6 grid grid-cols-2">
            <div>
                <label class="black dark:bg-black" for="server_port">Port</label>
                <Input disabled={toggle_disable} id="server_port" type="number" placeholder="Change server port"
                       bind:value={server_port}/>
            </div>
            <div>
                <label for="allow_upload">Allow upload</label>
                <Checkbox disabled={toggle_disable} id="allow_upload" type="checkbox"
                          bind:checked={allow_upload}/>
            </div>
            <div>
                <label for="serve_path">Serve Path</label>
                <Input disabled={toggle_disable} id="serve_path" type="text" placeholder="Change auth user" required
                       bind:value={serve_path}/>
            </div>
            <div>
                <label for="require_auth">Require Authentication</label>
                <Checkbox disabled={toggle_disable} id="require_auth"
                          bind:checked={require_auth}/>
            </div>
            <div>
                <label for="auth_user">Auth User</label>
                <Input disabled={toggle_disable} id="auth_user" type="text" placeholder="Change auth user" required
                       bind:value={auth_user}/>
            </div>
            <div>
                <label for="auth_passwd">Auth Password</label>
                <ButtonGroup size="lg">
                    <InputAddon size="sm">
                        <Button size="xs" class="mx-0" onclick={() => (show_password = !show_password)}>
                            {#if show_password}
                                <EyeOutline/>
                            {:else}
                                <EyeSlashOutline/>
                            {/if}
                        </Button>
                    </InputAddon>
                    <Input
                            id="auth_passwd"
                            type={show_password ? 'text' : 'password'}
                            autocomplete="new-password"
                            placeholder="Change auth password"
                            disabled={toggle_disable}
                            required
                            bind:value={auth_passwd}
                    />
                </ButtonGroup>
            </div>
        </div>

        <Button class="toggle_button" disabled={toggle_disable} type="submit">Reconfigure</Button>
    </form>
    {#if server_running}
        server listening at:
        <p>{server_host}:{server_port}</p>
        use the following links to access (tap to copy link):
        {#each listening_urls as url}
            <div class="mt-4">
                <UrlInfo url={url} require_auth={require_auth} auth_user={auth_user} auth_passwd={auth_passwd}/>
            </div>
        {/each}
    {/if}
    <!--        <Button class="ms-2" onclick={share_files}>-->
    <!--            Share file(s)-->
    <!--        </Button>-->
    <!--        <Button class="ms-2" onclick={share_folder}>-->
    <!--            Share a folder-->
    <!--        </Button>-->
    <Button class="ms-2" onclick={acquire_permission_android}>
        Acquire permission on Android
    </Button>
    <A href="https://github.com/fokx/localshare" target="_blank" class="font-medium hover:underline">
        <GithubSolid class="ms-2 h-6 w-6 me-1"/>
        source code
    </A>

    <p>log_disp</p>
    <p>{log_disp}</p>
    <p>{file}</p>
</div>



