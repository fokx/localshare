<script lang="ts">
    import "../app.css";
    import { SvelteToast } from '@zerodevx/svelte-toast';
    let { children } = $props();

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

<main
    class="container">
    <div id="mainContent" class="mt-[1vh]">
        {@render children()}
    </div>
</main>
<div class="">
    <BottomNav {activeUrl} position="absolute" innerClass="grid-cols-3">
        <BottomNavItem btnName="LocalSend" href="/localsend">
            <ShareNodesSolid />
        </BottomNavItem>
        <BottomNavItem btnName="Home" href="/">
            <HomeSolid />
        </BottomNavItem>
        <BottomNavItem btnName="Dufs" href="/dufs">
            <GlobeSolid />
        </BottomNavItem>
    </BottomNav>
    <SvelteToast {options} />
</div>


<style>
    .container {
        /*margin: 0;*/
        /*padding-top: 10vh;*/
        display: flex;
        /*flex-direction: column;*/
        justify-content: center;
        text-align: center;
    }
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


</style>