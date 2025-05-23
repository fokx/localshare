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
    let a;
    import { Skeleton, ImagePlaceholder, BottomNav, BottomNavItem } from 'flowbite-svelte';
    import {
        HomeSolid,
        AdjustmentsHorizontalSolid,
        ShareNodesSolid,
        GlobeSolid,
    } from 'flowbite-svelte-icons';
    import { page } from '$app/state';

    let activeUrl = $state(page.url.pathname);

    $effect(() => {
        activeUrl = page.url.pathname;
    });
</script>

<!--<container-->
<!--    class="container">-->
<div class="dark:bg-gray-700 dark:text-gray-200">
    <div class="mx-auto flex flex-col gap-2 overflow-y-scroll min-h-screen">
    {@render children()}
    </div>
    <!--</container>-->
    <BottomNav {activeUrl} position="sticky" outerClass="bg-white bg:bg-dark-800" innerClass="grid-cols-4">
        <!--<BottomNav {activeUrl} position="absolute" innerClass="grid-cols-3">-->
        <BottomNavItem btnName="LocalSend" href="/localsend">
            <ShareNodesSolid />
        </BottomNavItem>
        <BottomNavItem btnName="Home" href="/">
            <HomeSolid />
        </BottomNavItem>
        <BottomNavItem btnName="Dufs" href="/dufs">
            <GlobeSolid />
        </BottomNavItem>
        <BottomNavItem btnName="Darkmode">
            <DarkMode></DarkMode>
        </BottomNavItem>
    </BottomNav>
    <SvelteToast {options} />
</div>
