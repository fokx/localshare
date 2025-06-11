<script lang="ts">
    import "../app.css";
    import { SvelteToast } from '@zerodevx/svelte-toast';
    let { children } = $props();

    import { DarkMode } from "flowbite-svelte";

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
    import { Skeleton, ImagePlaceholder, BottomNav, BottomNavItem } from 'flowbite-svelte';
    import Fa from 'svelte-fa';
    import {faHouse, faMessage, faShare, faEarthAmericas} from '@fortawesome/free-solid-svg-icons';
    import {faDiscourse} from '@fortawesome/free-brands-svg-icons';
    import { page } from '$app/state';

    let activeUrl = $state(page.url.pathname);

    $effect(() => {
        activeUrl = page.url.pathname;
    });

    function toggleDarkMode(event) {
        let target = event.target;
        let child = target.querySelector('.darkmode-button-in-bottom-navbar');
        child?.click();
    }

</script>

<!--<container-->
<!--    class="container">-->
<div class="bg-gray-50 text-gray-950 dark:bg-gray-700 dark:text-gray-200">
    <div class="mx-auto flex flex-col gap-2 overflow-y-scroll min-h-screen">
    {@render children()}
    </div>
    <!--</container>-->
    <BottomNav {activeUrl} position="sticky" outerClass="bg-white bg:bg-dark-800" innerClass="grid-cols-6">
        <!--<BottomNav {activeUrl} position="absolute" innerClass="grid-cols-3">-->
        <BottomNavItem btnName="LocalSend" href="/localsend">
            <Fa icon={faShare} />

        </BottomNavItem>

        <BottomNavItem btnName="Chat" href="/chat">
            <Fa icon={faMessage} />

        </BottomNavItem>

        <BottomNavItem btnName="Home" href="/">
            <Fa icon={faHouse} />
        </BottomNavItem>

        <BottomNavItem btnName="Discourse" onclick={event => window.location = "http://127.0.0.1:4805/"}>
            <Fa icon={faDiscourse} />
        </BottomNavItem>

        <BottomNavItem btnName="Dufs" href="/dufs">
            <Fa icon={faEarthAmericas} />

        </BottomNavItem>

        <BottomNavItem btnName="Darkmode" onclick={event => toggleDarkMode(event)}>
            <DarkMode class="darkmode-button-in-bottom-navbar" />
        </BottomNavItem>
    </BottomNav>
    <SvelteToast {options} />
</div>
