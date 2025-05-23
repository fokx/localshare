<script lang="ts">
    import { appConfigDir, join, resourceDir, appLocalDataDir, appCacheDir, documentDir } from "@tauri-apps/api/path";
    import { onMount } from "svelte";
    import { openPath } from "@tauri-apps/plugin-opener";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import * as schema from "$lib/db/schema";
    import { db } from "$lib/db/database";
    import Inspect from "svelte-inspect-value";
    import { PaginationNav, Pagination, PaginationItem, Card, Heading, Button, Avatar} from "flowbite-svelte";
    import {display_time, process_cooked} from "$lib";
    import {eq} from "drizzle-orm";
    import {users} from "$lib/db/schema";
    import {goto} from "$app/navigation";
    import {getUserById, emoji} from "$lib";
    import { count, sql } from 'drizzle-orm';
    import { ArrowLeftOutline, ArrowRightOutline, CaretRightSolid, CaretLeftSolid } from "flowbite-svelte-icons";
    import { platform } from '@tauri-apps/plugin-os';
    let current_topics = $state<
        { id: number; created_at: string | null; raw: string | null }[]
    >([]);
    let currentPage = $state(1);
    if (window.currentTopicPage) {
        currentPage = window.currentTopicPage;
    }
    let totalPages = $state(9999);
    const NUM_TOPICS_PER_PAGE = 20;
    let topicsCount = $state();
    let currentPlatform;
    let visiblePagesTop = $state(4);
    let visiblePagesBottom = $state(7);
    let isDesktop = $state(false);

    async function loadTopics() {
        let offset = (currentPage-1)*NUM_TOPICS_PER_PAGE;
        console.log('loading topics with offset ',offset);
        db.query.topics
            .findMany({
                limit: NUM_TOPICS_PER_PAGE,
                offset: parseInt(offset),
                orderBy: { last_posted_at: "dsc" },
            })
            .execute()
            .then((results) => {
                // console.log("🚀 ~ FindMany response from Drizzle:", results);
                current_topics = results;
            });
        console.log("window.currentTopicPageScrollY", window.currentTopicPageScrollY);

    }

    function handlePageChange(page: number) {
        currentPage = page;
        loadTopics();
        window.scrollTo({left: 0, top: 0, behavior: 'smooth'});
        window.currentTopicPage = page;
        console.log("Page changed to:", page);
    }
    import { tick } from 'svelte';
    let scrollHandler;

    onMount(()=> {
        const container = document.querySelector("#container");
        console.log("component mounted")
        loadTopics();
        currentPlatform = platform();
        if (currentPlatform==="android"||currentPlatform==="ios"){
            visiblePagesTop=4;
            visiblePagesBottom=7;
        } else {
            isDesktop=true;
            visiblePagesTop=8;
            visiblePagesBottom=15;
        }
        scrollHandler = () => {
            console.log("Window scroll:", window.scrollY);
            if (window.scrollY > 100) {
                window.currentTopicPageScrollY = window.scrollY;
            }
        };


        window.addEventListener("scroll", scrollHandler);
        return () => {
            // window.currentTopicPageScrollY = window.scrollY;
            window.removeEventListener("scroll", scrollHandler);
            console.log("component destroyed", window.currentTopicPageScrollY);
        };
    });

    $effect.pre(() => {
        console.log('the component is about to update');
        tick().then(() => {
            setTimeout(() => {
                if (window.currentTopicPageScrollY !== undefined) {
                    console.log("Scrolling after delay to:", window.currentTopicPageScrollY);
                    window.scrollTo(0, window.currentTopicPageScrollY);
                }
            }, 50);
            console.log('the component just updated');
        });
    });

    $effect(async ()=>{
        let tmp = await db.select({ count: count() }).from(schema.topics);
        if (tmp){
            let cur_topicsCount = tmp[0].count;
            if (cur_topicsCount !== topicsCount) {
                topicsCount = cur_topicsCount;
                totalPages=Math.ceil(cur_topicsCount/NUM_TOPICS_PER_PAGE);
            }
        }
    });
</script>


    <div class="flex justify-center">
       <span class="me-4"> <strong>Topic Browser</strong></span>
        {#if totalPages>1}
        <PaginationNav visiblePages={Math.min(visiblePagesTop, totalPages)} {currentPage} {totalPages} onPageChange={handlePageChange}>
            {#snippet prevContent()}
                <span class="sr-only">Previous</span>
                <ArrowLeftOutline class="h-5 w-5" />
            {/snippet}
            {#snippet nextContent()}
                <span class="sr-only">Next</span>
                <ArrowRightOutline class="h-5 w-5" />
            {/snippet}
        </PaginationNav>
        {/if}
    </div>

    {#if current_topics}
        {#each current_topics as topic}
            <div class="flex justify-center dotted-ul prose dark:prose-invert">
                <Card class="max-w-[vw] p-6 ms-0.5 me-0.5" contentClass="dark:bg-gray-500" onclick={()=>{window.current_topic_id=topic.id; goto("/topic"); console.log(window.current_topic_id)}}>
                    {#if topic.title}
                        <div class="flex justify-center">
                            <h5 class="me-6 mb-2 text-2xl font-bold tracking-tight">{emoji.replace_colons(topic.title)}</h5>
                            in {topic.category_name}
                            &nbsp;
                            {#await getUserById(topic.user_id) then user}
                                <p>by {user.username}</p>
                            {/await}
                        </div>
                    {/if}
                    <div class="flex justify-between items-center mb-2">

                        <h6 class="me-4 mt-4 text-md font-bold tracking-tight">
                            last posted: {display_time(topic.last_posted_at)}
                        </h6>

                    </div>
                    <div class="primary-links">
                        {@html process_cooked(topic.excerpt)}
                    </div>
                </Card>
            </div>
            {/each}
    {/if}

{#if totalPages > 1}
    <div class="flex justify-center">
        <PaginationNav visiblePages={Math.min(visiblePagesBottom, totalPages)} class="sticky" {currentPage} {totalPages} onPageChange={handlePageChange} />
    </div>
{/if}