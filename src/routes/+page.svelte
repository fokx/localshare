<script lang="ts">
    // import {BaseDirectory, writeTextFile} from '@tauri-apps/plugin-fs';
    // import {enable, isEnabled} from '@tauri-apps/plugin-autostart';
    import {invoke, type PermissionState} from '@tauri-apps/api/core'
    import UrlInfo from '../components/UrlInfo.svelte';
    import {toast} from "@zerodevx/svelte-toast";
    import {A, Button, ButtonGroup, Checkbox, Heading, Input, InputAddon} from 'svelte-5-ui-lib';
    import {EyeOutline, EyeSlashOutline, GithubSolid} from 'flowbite-svelte-icons';

    let show_password = $state(false);

    // generate a random port number
    let server_port = $state(Math.floor(Math.random() * (65535 - 1024 + 1) + 1024));
    let require_auth = $state(true);
    let auth_user = $state("user");
    let serve_path = $state("/storage/emulated/0/");
    let auth_passwd = $state("User@1234");
    let allow_upload = $state(true);

    let server_running = $state(false);
    let server_host = $state("0.0.0.0");
    let toggle_disable = $state(false);
    let listening_urls: Array<string> = $state([]);

    interface Permissions {
        manageExternalStorage: PermissionState
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
            return;
        }
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
                .catch((e) => console.error(e));
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
                .catch((e) => console.error(e));
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

    // when using `"withGlobalTauri": true`, you may use
    // const { enable, isEnabled, disable } = window.__TAURI__.autostart;

    // onMount(async () => {

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
    // });

</script>

<main class="container">
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

            <Button class="toggle_button" disabled={toggle_disable} type="submit">Change</Button>
        </form>
        {#if server_running}
            server listening at:
            <p>{server_host}:{server_port}</p>
            use the following links to access (tap to copy link):
            {#each listening_urls as url}
                <div class="mt-4">
                    <UrlInfo url={url}/>
                </div>
            {/each}
        {/if}
        <Button class="ms-2" onclick={acquire_permission_android}>
            Acquire permission on Android
        </Button>
        <A href="https://github.com/fokx/localshare" target="_blank"  class="font-medium hover:underline">
            <GithubSolid class="ms-2 h-6 w-6 me-1"/>
            source code
        </A>
    </div>

</main>


<style>
    :root {
        font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
        /*font-size: 16px;*/
        /*line-height: 24px;*/
        /*font-weight: 400;*/

        color: #0f0f0f;
        background-color: #f6f6f6;

        /*font-synthesis: none;*/
        /*text-rendering: optimizeLegibility;*/
        /*-webkit-font-smoothing: antialiased;*/
        /*-moz-osx-font-smoothing: grayscale;*/
        /*-webkit-text-size-adjust: 100%;*/
    }

    @media (prefers-color-scheme: dark) {
        :root {
            color: #f6f6f6;
            background-color: #2f2f2f;
        }
    }

    .container {
        margin: 0;
        padding-top: 10vh;
        display: flex;
        flex-direction: column;
        justify-content: center;
        text-align: center;
    }

</style>
