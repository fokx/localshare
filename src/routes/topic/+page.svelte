<script lang="ts">
    import { appConfigDir, join, resourceDir, appLocalDataDir, appCacheDir, documentDir } from "@tauri-apps/api/path";
    import { onMount } from "svelte";
    import { openPath } from "@tauri-apps/plugin-opener";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import * as schema from "$lib/db/schema";
    import { db } from "$lib/db/database";
    import Inspect from "svelte-inspect-value";
    import {Avatar, Card, Heading, Spinner, PaginationNav, Pagination, PaginationItem} from "flowbite-svelte";
    import {display_time, process_cooked} from "$lib";
    import {count, eq} from "drizzle-orm";
    import {users, topics, posts} from "$lib/db/schema";
    import {goto} from "$app/navigation";
    import {getUserById} from "$lib";
    import {ArrowLeftOutline, ArrowRightOutline, CaretLeftSolid, CaretRightSolid} from "flowbite-svelte-icons";
    import {platform} from "@tauri-apps/plugin-os";
    let current_topic_posts = $state([]);
    let current_topic = $state(null);
    let currentPage = $state(1);
    let totalPages = $state(9999);
    const NUM_POSTS_PER_PAGE = 30;
    let postsCount = $state();
    let currentPlatform;
    let visiblePagesTop = $state(4);
    let visiblePagesBottom = $state(7);
    let isDesktop = $state(false);

    function handlePageChange(page: number) {
        currentPage = page;
        console.log("window.current_topic_id", window.current_topic_id);
        console.log("Page changed to:", page);
        load_topic_posts(window.current_topic_id, (page-1)*NUM_POSTS_PER_PAGE)
        window.scrollTo({left: 0, top: 0, behavior: 'smooth'});
    }

    onMount(()=>{
        currentPlatform = platform();
        if (currentPlatform==="android"||currentPlatform==="ios"){
            visiblePagesTop=4;
            visiblePagesBottom=7;
        } else {
            isDesktop=true;
            visiblePagesTop=8;
            visiblePagesBottom=15;
        }
    });
    $effect(async ()=>{
        let tmp = await db.select({ count: count() }).from(schema.posts)
            .where(eq(schema.posts.topic_id, window.current_topic_id));
        if (tmp){
            let cur_postsCount = tmp[0].count;
            if (cur_postsCount !== postsCount) {
                postsCount = cur_postsCount;
                totalPages=Math.ceil(cur_postsCount/NUM_POSTS_PER_PAGE);
            }
        }
    });
    async function load_topic_posts(topic_id:number, offset:number) {
        console.log("finding topic with id", topic_id);
        db.query.topics
            .findFirst({
                where: {
                    id: topic_id
                }
            })
            .execute()
            .then((result) => {
                current_topic = result;
            });

        await db.query.posts
            .findMany({
                limit: NUM_POSTS_PER_PAGE,
                offset: parseInt(offset),
                where: {topic_id: topic_id},
            })
            .execute()
            .then((results) => {
                console.log("🚀 ~ FindMany response from Drizzle:", results);
                current_topic_posts = results;
                return results;
            });
    }

</script>

<!--<main class="container mx-auto flex flex-col gap-4">-->
{#await load_topic_posts(window.current_topic_id, 0)}
    Loading...
    <Spinner class="me-3" size="4" color="teal" />
{:then value}
    <!--{#if !value}-->
<!--            <p style="color: red">Topic not found or empty</p>-->
<!--        {/if}-->
{:catch error}
    <p style="color: red">Topic cannot be loaded with {error.message}</p>
{/await}

<div class="flex justify-between items-center">
    <Heading tag="h5" class="text-primary-700 dark:text-primary-500 mx-auto">
        {#if current_topic}
            {current_topic.title}
        {:else}
            Topic {window.current_topic_id}
        {/if}
    </Heading>
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
    <Avatar class="w-10 h-10" onclick={()=>{history.back();}}><CaretLeftSolid/></Avatar>
</div>

{#if current_topic_posts && current_topic_posts.length > 0}
    {#each current_topic_posts as post}
        {#if post}
            <div class="flex-grow justify-center primary-links dotted-ul prose dark:prose-invert">
                <Card class="max-w-[vw] p-6 ms-0.5 me-0.5 dark:text-gray-200" >
                    {#if post.title}
                        <div class="flex justify-center">
                            <h5 class="mb-2 text-2xl font-bold tracking-tight">{post.title}</h5>
                        </div>
                    {/if}
                    <div class="flex justify-between items-center mb-2">
                        <h6 class="mt-4 text-md font-bold tracking-tight">
                            <div>
                                {#if (post.updated_at - post.created_at) > 5 * 60 * 1000}
                                    updated at: {display_time(post.updated_at)}
                                {:else}
                                    created at: {display_time(post.created_at)}
                                {/if}
                                &nbsp;
                                {#await getUserById(post.user_id) then user}
                                    by {user.username}
                                {/await}
                            </div>

                        </h6>
                    </div>
                    {#if post.cooked}
                        {@html process_cooked(post.cooked)}
                    {:else if post.excerpt}
                        {@html process_cooked(post.excerpt + '<br>......')}
                    {/if}
                    <div class="flex justify-end items-center">
                        <span class="text-blue-800 dark:text-blue-500 text-xl mr-2"># {post.post_number}</span>
                        <button
                                class="block"
                                onclick={() => {}}
                                title={`Reply to post #${post.post_number}`}
                                aria-label={`Reply to post #${post.post_number}`}>
                            Reply
                        </button>
                    </div>
                </Card>
            </div>
        {:else}
            <p>Post not found</p>
        {/if}
    {/each}
{/if}
{#if totalPages>1}
    <div class="flex justify-center">
        <PaginationNav visiblePages={Math.min(visiblePagesBottom, totalPages)} class="sticky" {currentPage} {totalPages} onPageChange={handlePageChange} />
    </div>
{/if}
<!--</main>-->
