<script lang="ts">
    import { appConfigDir, join, resourceDir, appLocalDataDir, appCacheDir, documentDir } from "@tauri-apps/api/path";
    import { onMount } from "svelte";
    import { openPath } from "@tauri-apps/plugin-opener";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import * as schema from "$lib/db/schema";
    import { db } from "$lib/db/database";
    import Inspect from "svelte-inspect-value";
    import {Avatar, Card, Heading, Spinner, PaginationNav, Pagination, PaginationItem} from "flowbite-svelte";
    import {dbb, display_time, process_cooked} from "$lib";
    import {count, eq} from "drizzle-orm";
    import {users, topics, posts} from "$lib/db/schema";
    import {goto} from "$app/navigation";
    import {getUserById, emoji, isLoading} from "$lib";
    import Fa from 'svelte-fa';
    import {faArrowLeft, faArrowRight, faCaretLeft, faCaretUp, faCaretDown} from '@fortawesome/free-solid-svg-icons';
    import {platform} from "@tauri-apps/plugin-os";
    import { fetch } from '@tauri-apps/plugin-http';

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
    import { siteTitle } from '$lib';

    async function fetchLatestTopicPosts() {
        try {
            isLoading.set(true);
            let topic_id = window.current_topic_id;
            let url = `http://127.0.0.1:4805/t/${topic_id}.json?print=true`; // with print=true, will fetch at most 1000 posts
            let json = await fetch(url).then(r => r.json()).catch(e => console.error(e));
            if (!json || !json.post_stream) {
                console.error(`Invalid response from ${url}`);
                return Promise.resolve();
            }

            let posts = json?.post_stream?.posts || [];

            // Process all posts in one batch
            for (const item of posts) {
                let _item = {
                    id: item.id,
                    raw: item.raw || null,
                    cooked: item.cooked || null,
                    post_number: item.post_number || null,
                    topic_id: item.topic_id || null,
                    user_id: item.user_id || null,
                    created_at: item.created_at ? new Date(item.created_at) : null,
                    updated_at: item.updated_at ? new Date(item.updated_at) : null,
                    reply_to_post_number: item.reply_to_post_number || null,
                    reply_to_user_id: item.reply_to_user_id || null,
                    reply_count: item.reply_count || null,
                    like_count: item.like_count || null, // not found in json
                    word_count: item.word_count || null // not found in json
                }
                await db.insert(schema.posts).values(_item)
                    .onConflictDoUpdate({target: schema.posts.id, set: _item});
            }
            
            // Only call load_topic_posts once after all posts are processed
            await load_topic_posts(window.current_topic_id);
        } catch (error) {
            console.error('Error fetching topic posts:', error);
        } finally {
            isLoading.set(false);
        }
    }
    /*
              {
        "id": 55854,
        "name": "wyhlele",
        "username": "wyhao",
        "avatar_template": "/letter_avatar_proxy/v4/letter/w/ce73a5/{size}.png",
        "created_at": "2025-06-10T08:28:28.984Z",
        "cooked": "\u003cp data-ln=\"0\"\u003eRUST 课程自主命题项目，正值校庆 130 周年宣传，故制作此游戏，现发布于 github，欢迎大家来体验。\u003c/p\u003e\n\u003cp data-ln=\"2\"\u003e游戏下载链接：\u003c/p\u003e\u003caside class=\"onebox githubrepo\" data-onebox-src=\"https://github.com/wyhlele/I-wanna-be-XJTUer\"\u003e\n  \u003cheader class=\"source\"\u003e\n\n      \u003ca href=\"https://github.com/wyhlele/I-wanna-be-XJTUer\" target=\"_blank\" rel=\"noopener nofollow ugc\"\u003egithub.com\u003c/a\u003e\n  \u003c/header\u003e\n\n  \u003carticle class=\"onebox-body\"\u003e\n    \u003cdiv class=\"github-row\" data-github-private-repo=\"false\"\u003e\n  \u003cimg width=\"690\" height=\"344\" class=\"thumbnail\" src=\"//xjtu.app/uploads/default/original/3X/5/7/5712cb3f84ecdd34a0f3bdfacd7690617c7cc760.png\" data-dominant-color=\"F4F2F3\"\u003e\n\n  \u003ch3\u003e\u003ca href=\"https://github.com/wyhlele/I-wanna-be-XJTUer\" target=\"_blank\" rel=\"noopener nofollow ugc\"\u003eGitHub - wyhlele/I-wanna-be-XJTUer\u003c/a\u003e\u003c/h3\u003e\n\n    \u003cp\u003e\u003cspan class=\"github-repo-description\"\u003e通过在 GitHub 上创建帐户来为 wyhlele/I-wanna-be-XJTUer 开发做出贡献。\u003c/span\u003e\u003c/p\u003e\n\u003c/div\u003e\n\n  \u003c/article\u003e\n\n  \u003cdiv class=\"onebox-metadata\"\u003e\n    \n    \n  \u003c/div\u003e\n\n  \u003cdiv style=\"clear: both\"\u003e\u003c/div\u003e\n\u003c/aside\u003e\n",
        "external_id": "UI6WNqgfuw6O4jnT",
        "post_number": 1,
        "post_type": 1,
        "posts_count": 14,
        "updated_at": "2025-06-10T08:28:28.984Z",
        "reply_count": 0,
        "reply_to_post_number": null,
        "quote_count": 0,
        "incoming_link_count": 0,
        "reads": 43,
        "readers_count": 42,
        "score": 188.6,
        "yours": false,
        "topic_id": 14751,
        "topic_slug": "topic",
        "display_username": "wyhlele",
        "primary_group_name": null,
        "flair_name": null,
        "flair_url": null,
        "flair_bg_color": null,
        "flair_color": null,
        "flair_group_id": null,
        "badges_granted": [],
        "version": 1,
        "can_edit": false,
        "can_delete": false,
        "can_recover": false,
        "can_see_hidden_post": false,
        "can_wiki": false,
        "link_counts": [
          {
            "url": "https://github.com/wyhlele/I-wanna-be-XJTUer",
            "internal": false,
            "reflection": false,
            "title": "GitHub - wyhlele/I-wanna-be-XJTUer",
            "clicks": 13
          }
        ],
        "read": true,
        "user_title": "",
        "bookmarked": false,
        "actions_summary": [
          {
            "id": 2,
            "count": 8
          }
        ],
        "moderator": false,
        "admin": false,
        "staff": false,
        "user_id": 6032,
        "hidden": false,
        "trust_level": 1,
        "deleted_at": null,
        "user_deleted": false,
        "edit_reason": null,
        "can_view_edit_history": false,
        "wiki": false,
        "mentioned_users": [],
        "post_url": "/t/topic/14751/1",
        "animated_avatar": null,
        "calendar_details": [],
        "journal": null,
        "reactions": [
          {
            "id": "heart",
            "type": "emoji",
            "count": 6
          },
          {
            "id": "grey_question",
            "type": "emoji",
            "count": 2
          }
        ],
        "current_user_reaction": null,
        "reaction_users_count": 8,
        "current_user_used_main_reaction": false,
        "user_signature": null,
        "can_accept_answer": false,
        "can_unaccept_answer": false,
        "accepted_answer": false,
        "topic_accepted_answer": null,
        "retorts": [],
        "my_retorts": [],
        "can_retort": false,
        "can_remove_retort": false
      }
         */

    function handlePageChange(page: number) {
        if ($isLoading) return; // Prevent multiple calls while loading
        
        isLoading.set(true);
        dbb.browse_history.update(window.current_topic_id, {
            topic_id: window.current_topic_id,
            page_number: page,
        });
        currentPage = page;
        console.log("window.current_topic_id", window.current_topic_id);
        console.log("Page changed to:", page);
        
        // Only load posts, don't fetch new data on page change
        load_topic_posts(window.current_topic_id)
            .then(() => {
                isLoading.set(false);
                window.scrollTo({left: 0, top: 0, behavior: 'smooth'});
            });
    }

    onMount(async () => {
        isLoading.set(true);
        // First load existing posts from database
        await load_topic_posts(window.current_topic_id);
        // Then fetch latest posts from API
        await fetchLatestTopicPosts();
        
        currentPlatform = platform();
        if (currentPlatform === "android" || currentPlatform === "ios") {
            visiblePagesTop = 4;
            visiblePagesBottom = 7;
        } else {
            isDesktop = true;
            visiblePagesTop = 8;
            visiblePagesBottom = 15;
        }
        isLoading.set(false);
    });
    
    $effect(async () => {
        let tmp = await db.select({ count: count() }).from(schema.posts)
            .where(eq(schema.posts.topic_id, window.current_topic_id));
        if (tmp) {
            let cur_postsCount = tmp[0].count;
            if (cur_postsCount !== postsCount) {
                postsCount = cur_postsCount;
                totalPages = Math.ceil(cur_postsCount/NUM_POSTS_PER_PAGE);
            }
        }
    });
    
    async function load_topic_posts(topic_id: number) {
        let tmp = await dbb.browse_history.get({topic_id: topic_id});
        if (tmp) {
            currentPage = tmp.page_number;
            console.log("get currentPage from dexie", currentPage);
        } else {
            currentPage = 1;
            await dbb.browse_history.add({
                topic_id: window.current_topic_id,
                page_number: 1,
            });
        }
        
        let offset = (currentPage-1)*NUM_POSTS_PER_PAGE;
        console.log("finding topic with id", topic_id);
        
        // Run these queries in parallel with Promise.all
        await Promise.all([
            db.query.topics
                .findFirst({
                    where: {
                        id: topic_id
                    }
                })
                .execute()
                .then((result) => {
                    if (result) {
                        siteTitle.set(emoji.replace_colons(result.title));
                        current_topic = result;
                    }

                }),
                
            db.query.posts
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
                })
        ]);
    }
</script>

<!--<div class="flex justify-between items-center">-->
<!--    <Heading tag="h5" class="text-primary-700 dark:text-primary-500 mx-auto">-->
<!--        {#if current_topic}-->
<!--            {emoji.replace_colons(current_topic.title)}-->
<!--        {:else}-->
<!--            Topic {window.current_topic_id}-->
<!--        {/if}-->
<!--    </Heading>-->

    <!--{#if totalPages>1}-->
    <!--    <PaginationNav visiblePages={Math.min(visiblePagesTop, totalPages)} {currentPage} {totalPages} onPageChange={handlePageChange}>-->
    <!--        {#snippet prevContent()}-->
    <!--            <span class="sr-only">Previous</span>-->
    <!--            <Fa icon={faArrowLeft} />-->
    <!--        {/snippet}-->
    <!--        {#snippet nextContent()}-->
    <!--            <span class="sr-only">Next</span>-->
    <!--            <Fa icon={faArrowRight} />-->

    <!--        {/snippet}-->
    <!--    </PaginationNav>-->
    <!--{/if}-->

<!--    </div>-->

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
                                    {#if user}
                                        by {user.username}
                                    {/if}
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
    <div class="flex justify-between items-center">
        <div class="mx-auto">
            <PaginationNav visiblePages={Math.min(visiblePagesBottom, totalPages)} class="sticky" {currentPage} {totalPages} onPageChange={handlePageChange} />
        </div>
        <div class="flex">
            <Avatar class="w-10 h-10 mr-2" onclick={()=>{history.back();}}>
                <Fa icon={faCaretLeft} />

            </Avatar>
            <Avatar class="w-10 h-10" onclick={()=>{window.scrollTo({left: 0, top: 0, behavior: 'smooth'});}}><Fa icon={faCaretUp} /></Avatar>
        </div>
    </div>

{/if}
