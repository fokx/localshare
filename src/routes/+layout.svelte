<script lang="ts">
    import "../app.css";
    import {SvelteToast, toast} from '@zerodevx/svelte-toast';
    import {login, type OauthUser, logout} from '$lib/auth';

    let {children} = $props();
    import {
        Modal,
        Label,
        Input,
        Checkbox,
        Button,
        DarkMode,
        Navbar,
        NavBrand,
        NavLi,
        NavUl,
        NavHamburger,
        Skeleton,
        ImagePlaceholder,
        BottomNav,
        BottomNavItem,
        Avatar,
        Dropdown,
        DropdownItem,
        DropdownHeader,
        DropdownGroup
    } from "flowbite-svelte";

    const options = {
        duration: 1000,       // duration of progress bar tween to the `next` value
        initial: 1,           // initial progress bar value
        next: 0,              // next progress value
        pausable: false,      // pause progress bar tween on mouse hover
        dismissable: true,    // allow dismiss with close button
        reversed: false,      // insert new toast to bottom of stack
        intro: {x: 256},    // toast intro fly animation settings
        theme: {},            // css var overrides
        classes: []           // user-defined classes
    }
    import Fa from 'svelte-fa';
    import {
        faHouse,
        faMessage,
        faShare,
        faEarthAmericas,
        faCaretLeft,
        faRotateRight,
        faCaretDown
    } from '@fortawesome/free-solid-svg-icons';
    import {faDiscourse} from '@fortawesome/free-brands-svg-icons';
    import {page} from '$app/state';

    import {getCurrentUser} from '$lib/auth';

    let activeUrl = $state(page.url.pathname);
    let SITE_TITLE = "XAP";
    import {derived} from 'svelte/store';
    import {isLoading, siteTitle} from '$lib';
    import {Spinner} from "../../../flowbite-svelte/dist/spinner";

    let currentSiteTitle = $state(SITE_TITLE);

    // const siteTitleStore = derived(siteTitle,
    //     ($a, set) => {
    //         set($a);
    //     },
    // );

    $effect(() => {
        activeUrl = page.url.pathname;
        if (activeUrl === '/') {
            currentSiteTitle = SITE_TITLE;
        } else {
            currentSiteTitle = $siteTitle;
        }
    });

    function toggleDarkMode(event) {
        let target = event.target;
        let child = target.querySelector('.darkmode-button-in-bottom-navbar');
        child?.click();
    }


    function process_title(post_title: string, truncate_to_length: number = 40) {
        let title: string;
        if (page.url.pathname === '/') {
            title = SITE_TITLE;
            document.title = SITE_TITLE;
        } else {
            title = page.url.pathname.replaceAll('/', ' ');
            if (post_title && (page.url.pathname.startsWith('/t/')
                || page.url.pathname.startsWith('/chat')
                || page.url.pathname.startsWith('/p/'))) {
                title = post_title;
            }
            document.title = title.slice(0, truncate_to_length) + ` - ${SITE_TITLE}`;
        }
        console.log('processed title', title)
        return title;
    }

    import {fly} from "svelte/transition";
    import {onMount} from "svelte";
    import {goto} from "$app/navigation";

    let loginModal = $state(false);
    let user: OauthUser | null = $state(null);

    onMount(async () => {
        try {
            let _user = await getCurrentUser();
            console.log(_user);
            user = _user;
        } catch (error) {
            console.error('Authentication check failed:', error);
        } finally {
            isLoading.set(false);
        }
    });

    let isDiscourseLoading = $state(false);
    let errorMessage = $state('');

    async function loginWithDiscourse() {
        try {
            errorMessage = '';
            isDiscourseLoading = true;
            await login('discourse');
            user = await getCurrentUser();
            loginModal = false;
            invalidateAll();
            toast.push('login successfully');
        } catch (error) {
            console.error('Discourse login failed:', error);
            errorMessage = 'Discourse login failed, please retry';
        } finally {
            isDiscourseLoading = false;
        }
    }

    import {invalidateAll} from '$app/navigation';

    async function handleLogout() {
        await logout();
        user = null;
        invalidateAll();
        toast.push('logout successfully');
        goto('/');
    }

</script>

<!--<container-->
<!--    class="container">-->
<div class="bg-gray-50 text-gray-950 dark:bg-gray-700 dark:text-gray-200">
    <Navbar>
        <NavBrand href="/">
            <img src="/favicon.png" class="me-3 h-5 sm:h-6" alt="App Logo"/>
            <span class="self-center font-semibold whitespace-nowrap dark:text-white">{currentSiteTitle}</span>
        </NavBrand>
        <div class="flex items-center md:order-2">
            {#if !user}
                <Button color="primary" onclick={()=>{loginModal=true}}>Login</Button>
            {/if}
            <DarkMode/>
            {#if $isLoading}
                <div class="flex justify-center my-4 ml-2 mr-2">
                    <Spinner size="4" color="teal"/>
                </div>
            {:else}
                <Avatar class="w-10 h-10 mr-2" onclick={()=>{location.reload();}}>
                    <Fa icon={faRotateRight}/>
                </Avatar>
            {/if}
            <Avatar class="w-10 h-10 mr-2" onclick={()=>{history.back();}}>
                <Fa icon={faCaretLeft}/>
            </Avatar>
            <Avatar class="w-10 h-10"
                    onclick={()=>{window.scrollTo({left: 0, top: document.body.scrollHeight, behavior: 'smooth'});}}>
                <Fa icon={faCaretDown}/>
            </Avatar>
            <Avatar id="avatar-menu" src={user?.avatar_url ? user?.avatar_url : "/favicon.png"} class="me-1 ms-3 size-6"/>
            <NavHamburger/>
        </div>
        <Dropdown placement="bottom" triggeredBy="#avatar-menu">
            {#if user}
                <DropdownGroup>
                    <DropdownItem>Dashboard</DropdownItem>
                    <!--                <span class="block truncate text-sm font-medium">NotImplemented Yet</span>-->
                    <DropdownItem>Settings</DropdownItem>
                </DropdownGroup>
                <DropdownHeader>
                    <Button class="block text-sm" onclick={handleLogout}>Logout</Button>
                </DropdownHeader>
                <DropdownHeader>
                    {user?.name}
                    {user?.email}
                    {user?.avatar_url}
                    {user?.id}
                    {user?.provider}
                    {user?.username}
                    {user?.admin}
                    {user?.moderator}
                    {user?.groups}
                    {user?.user_global_api_key}
                </DropdownHeader>
            {:else }
                <DropdownHeader>
                    <Button class="block text-sm" onclick={()=>{loginModal=true}}>Login</Button>
                </DropdownHeader>
            {/if}

        </Dropdown>
        <NavUl {activeUrl} transition={fly} transitionParams={{ y: -20, duration: 250 }}>
            <NavLi href="/">Home</NavLi>
            <NavLi href="/chat">Chat</NavLi>
            <NavLi href="/localsend">LocalSend</NavLi>
            <NavLi href="/dufs">Dufs</NavLi>
            <NavLi href="/discourse">Discourse</NavLi>
        </NavUl>
    </Navbar>

    <Modal bind:open={loginModal} size="xs">
        <h3 class="mb-4 text-xl font-medium text-gray-900 dark:text-white">Sign in with xjtu.app account</h3>
        <Button onclick={loginWithDiscourse}>
            {#if !isDiscourseLoading}
                <svg class="mr-2 h-4 w-4" xmlns="http://www.w3.org/2000/svg" viewBox="0 -1 104 106" width="16">
                    <path fill="#231f20"
                          d="M51.87 0C23.71 0 0 22.83 0 51v52.81l51.86-.05c28.16 0 51-23.71 51-51.87S80 0 51.87 0Z"/>
                    <path fill="#fff9ae"
                          d="M52.37 19.74a31.62 31.62 0 0 0-27.79 46.67l-5.72 18.4 20.54-4.64a31.61 31.61 0 1 0 13-60.43Z"/>
                    <path fill="#00aeef"
                          d="M77.45 32.12a31.6 31.6 0 0 1-38.05 48l-20.54 4.7 20.91-2.47a31.6 31.6 0 0 0 37.68-50.23Z"/>
                    <path fill="#00a94f"
                          d="M71.63 26.29A31.6 31.6 0 0 1 38.8 78l-19.94 6.82 20.54-4.65a31.6 31.6 0 0 0 32.23-53.88Z"/>
                    <path fill="#f15d22"
                          d="M26.47 67.11a31.61 31.61 0 0 1 51-35 31.61 31.61 0 0 0-52.89 34.3l-5.72 18.4Z"/>
                    <path fill="#e31b23"
                          d="M24.58 66.41a31.61 31.61 0 0 1 47.05-40.12 31.61 31.61 0 0 0-49 39.63l-3.76 18.9Z"/>
                </svg>
            {:else}
                <span class="animate-spin mr-2">⟳</span>
            {/if}
            Discourse Login
        </Button>
        {#if errorMessage}
            <p class="text-red-500 text-center mt-2">{errorMessage}</p>
        {/if}
    </Modal>
    <div class="mx-auto flex flex-col gap-2 overflow-y-scroll min-h-screen">
        {@render children()}
    </div>
    <!--</container>-->
    <BottomNav {activeUrl} position="sticky" outerClass="bg-white bg:bg-dark-800 sm:hidden" innerClass="grid-cols-5">
        <!--<BottomNav {activeUrl} position="absolute" innerClass="grid-cols-3">-->
        <BottomNavItem btnName="Discourse" onclick={event => window.location = "http://127.0.0.1:4805/"}>
            <Fa icon={faDiscourse}/>
        </BottomNavItem>

        <BottomNavItem btnName="Chat" href="/chat">
            <Fa icon={faMessage}/>
        </BottomNavItem>

        <BottomNavItem btnName="Home" href="/">
            <Fa icon={faHouse}/>
        </BottomNavItem>

        <BottomNavItem btnName="LocalSend" href="/localsend">
            <Fa icon={faShare}/>
        </BottomNavItem>

        <BottomNavItem btnName="Dufs" href="/dufs">
            <Fa icon={faEarthAmericas}/>
        </BottomNavItem>

    </BottomNav>
    <SvelteToast {options}/>
</div>
