<script lang="ts">
    import "../app.css";
    import { SvelteToast } from '@zerodevx/svelte-toast';
    let { children } = $props();
    import { Modal,Label, Input, Checkbox,Button, DarkMode, Navbar, NavBrand, NavLi, NavUl, NavHamburger, Skeleton, ImagePlaceholder, BottomNav,
        BottomNavItem, Avatar, Dropdown, DropdownItem, DropdownHeader, DropdownGroup  } from "flowbite-svelte";
    const options = {
        duration: 1000,       // duration of progress bar tween to the `next` value
        initial: 1,           // initial progress bar value
        next: 0,              // next progress value
        pausable: false,      // pause progress bar tween on mouse hover
        dismissable: true,    // allow dismiss with close button
        reversed: false,      // insert new toast to bottom of stack
        intro: { x: 256 },    // toast intro fly animation settings
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
    import { page } from '$app/state';

    let activeUrl = $state(page.url.pathname);
    let SITE_TITLE = "XAP";
    import { derived } from 'svelte/store';
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
    import { fly } from "svelte/transition";
    let formModal = $state(false);
</script>

<!--<container-->
<!--    class="container">-->
<div class="bg-gray-50 text-gray-950 dark:bg-gray-700 dark:text-gray-200">
    <Navbar>
        <NavBrand href="/">
            <img src="/favicon.png" class="me-3 h-5 sm:h-6" alt="App Logo" />
            <span class="self-center font-semibold whitespace-nowrap dark:text-white">{currentSiteTitle}</span>
        </NavBrand>
        <div class="flex items-center md:order-2">
            <button onclick={()=>{formModal=true}}>Login</button>
            <DarkMode />
            {#if $isLoading}
                <div class="flex justify-center my-4 ml-2 mr-2">
                    <Spinner size="4" color="teal" />
                </div>
            {:else}
                <Avatar class="w-10 h-10 mr-2" onclick={()=>{location.reload();}}>
                    <Fa icon={faRotateRight} />
                </Avatar>
            {/if}
            <Avatar class="w-10 h-10 mr-2" onclick={()=>{history.back();}}>
                <Fa icon={faCaretLeft} />
            </Avatar>
            <Avatar class="w-10 h-10" onclick={()=>{window.scrollTo({left: 0, top: document.body.scrollHeight, behavior: 'smooth'});}}>            <Fa icon={faCaretDown} />
            </Avatar>
            <Avatar id="avatar-menu" src="/favicon.png" class="rotate-80 me-1 ms-3 size-6"/>
            <NavHamburger />
        </div>
        <Dropdown placement="bottom" triggeredBy="#avatar-menu">
            <DropdownHeader>
                <a class="block text-sm" onclick={()=>{formModal=true}}>Login</a>
                <span class="block truncate text-sm font-medium">NotImplemented Yet</span>
            </DropdownHeader>
            <DropdownGroup>
                <DropdownItem>Dashboard</DropdownItem>
                <DropdownItem>Settings</DropdownItem>
            </DropdownGroup>
            <DropdownHeader>Sign out</DropdownHeader>
        </Dropdown>
        <NavUl {activeUrl} transition={fly} transitionParams={{ y: -20, duration: 250 }}>
            <NavLi href="/">Home</NavLi>
            <NavLi href="/localsend">LocalSend</NavLi>
            <NavLi href="/dufs">Dufs</NavLi>
            <NavLi href="/chat">Chat</NavLi>
            <NavLi href="/discourse">Discourse</NavLi>
        </NavUl>
    </Navbar>

    <Modal bind:open={formModal} size="xs">
        <form class="flex flex-col space-y-6" method="dialog" action="#">
            <h3 class="mb-4 text-xl font-medium text-gray-900 dark:text-white">Sign in with xjtu.app account</h3>
            <Label class="space-y-2">
                <span>Email</span>
                <Input type="email" name="email" placeholder="name@company.com" required />
            </Label>
            <Label class="space-y-2">
                <span>Your password</span>
                <Input type="password" name="password" placeholder="•••••" required />
            </Label>
            <div class="flex items-start">
                <Checkbox>Remember me</Checkbox>
                <a href="/" class="text-primary-700 dark:text-primary-500 ms-auto text-sm hover:underline">Lost password?</a>
            </div>
            <Button type="submit" class="w-full1">Login to your account</Button>
            <div class="text-sm font-medium text-gray-500 dark:text-gray-300">
                Not registered? <a href="/" class="text-primary-700 dark:text-primary-500 hover:underline">Create account</a>
            </div>
        </form>
    </Modal>
    <div class="mx-auto flex flex-col gap-2 overflow-y-scroll min-h-screen">
    {@render children()}
    </div>
    <!--</container>-->
    <BottomNav {activeUrl} position="sticky" outerClass="bg-white bg:bg-dark-800 sm:hidden" innerClass="grid-cols-5">
        <!--<BottomNav {activeUrl} position="absolute" innerClass="grid-cols-3">-->
        <BottomNavItem btnName="Discourse" onclick={event => window.location = "http://127.0.0.1:4805/"}>
            <Fa icon={faDiscourse} />
        </BottomNavItem>

        <BottomNavItem btnName="Chat" href="/chat">
            <Fa icon={faMessage} />
        </BottomNavItem>

        <BottomNavItem btnName="Home" href="/">
            <Fa icon={faHouse} />
        </BottomNavItem>

        <BottomNavItem btnName="LocalSend" href="/localsend">
            <Fa icon={faShare} />
        </BottomNavItem>

        <BottomNavItem btnName="Dufs" href="/dufs">
            <Fa icon={faEarthAmericas} />
        </BottomNavItem>

    </BottomNav>
    <SvelteToast {options} />
</div>
