<script lang="ts">
    import { appConfigDir, join, resourceDir, appLocalDataDir, appCacheDir, documentDir } from "@tauri-apps/api/path";
    import { onMount } from "svelte";
    import { openPath } from "@tauri-apps/plugin-opener";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import * as schema from "$lib/db/schema";
    import { db } from "$lib/db/database";
    import Inspect from "svelte-inspect-value";
    import { Card, Spinner } from "flowbite-svelte";
    import {process_cooked} from "$lib";
    import {eq} from "drizzle-orm";
    import {users, posts} from "$lib/db/schema";
    import {goto} from "$app/navigation";
    let topic_posts = $state([]);

    async function load_topic_posts(topic_id) {
        console.log("finding topic with id", topic_id);
        await db.query.posts
            .findMany({
                limit: 100,
                where: eq(posts.topic_id, topic_id),
            })
            .execute()
            .then((results) => {
                console.log("🚀 ~ FindMany response from Drizzle:", results);
                topic_posts = results;
                return results;
            });
    }

</script>

<main class="container mx-auto flex flex-col gap-4">
    Topic {window.current_topic_id}
    {#await load_topic_posts(window.current_topic_id)}
        Loading...
        <Spinner class="me-3" size="4" color="teal" />
    {:then topic_posts}
        {#if !topic_posts}
            <p style="color: red">Topic not found or empty</p>
        {/if}
    {:catch error}
        <p style="color: red">Topic cannot be loaded with {error.message}</p>
    {/await}

    {#if topic_posts && topic_posts.length > 0}
        {#each topic_posts as post}
            {#if post}
                <div class="flex-grow justify-center primary-links dotted-ul prose dark:prose-invert">
                    <Card class="max-w-3xl" >
                        {#if post.title}
                            <div class="flex justify-center">
                                <h5 class="mb-2 text-2xl font-bold tracking-tight">{post.title}</h5>
                            </div>
                        {/if}
                        <div class="flex justify-between items-center mb-2">
                            <h6 class="mt-4 text-md font-bold tracking-tight">
                                {#if (post.updated_at - post.created_at) > 5 * 60 * 1000}
                                    <div>updated at: {(post.updated_at)}</div>
                                {:else}
                                    <div>created at: {(post.created_at)}</div>
                                {/if}
                            </h6>
                        </div>
                        {#if post.cooked}
                            {@html process_cooked(post.cooked)}
                        {:else if post.excerpt}
                            {@html process_cooked(post.excerpt + '<br>......')}
                        {/if}
                        <div class="flex justify-end items-center">
                            <a class="text-blue-800 dark:text-blue-500 text-xl mr-2" href={`/p/${post.id}`}># {post.post_number}</a>
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

</main>
