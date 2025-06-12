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
    import {getUserById, emoji, isLoading} from "$lib";
    import { count, sql } from 'drizzle-orm';
    import Fa from 'svelte-fa';
    import {faArrowLeft, faArrowRight} from '@fortawesome/free-solid-svg-icons';
    import {faDiscourse} from '@fortawesome/free-brands-svg-icons';
    import { platform } from '@tauri-apps/plugin-os';
    import {migrations} from "$lib/db/migrations";
    import { fetch } from '@tauri-apps/plugin-http';

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
    let visiblePagesTop = $state(3);
    let visiblePagesBottom = $state(7);
    let isDesktop = $state(false);

    async function loadTopics() {
        let offset = (currentPage-1)*NUM_TOPICS_PER_PAGE;
        console.log('loading topics with offset ',offset);

        // Return the promise so it can be chained
        return db.query.topics
            .findMany({
                limit: NUM_TOPICS_PER_PAGE,
                offset: parseInt(offset),
                orderBy: { last_posted_at: "dsc" },
            })
            .execute()
            .then((results) => {
                // console.log("🚀 ~ FindMany response from Drizzle:", results);
                current_topics = results;
                console.log("window.currentTopicPageScrollY", window.currentTopicPageScrollY);
                return results;
            });
    }
    async function fetchLatest() {
        // Return a promise that resolves when both operations are complete
        return Promise.all([
            fetchLatestTopics(),
            fetchLatestPosts()
        ]);
    }
    async function fetchLatestPosts() {
        try {
            let json = await fetch('http://127.0.0.1:4805/posts.json').then(r => r.json()).catch(e => console.error(e));
            console.log('json', json);

            if (!json || !json.latest_posts) {
                console.error('Invalid response from posts.json');
                return;
            }

            let latest_posts = json.latest_posts;
            /*
            {
            "id": 56004,
            "name": "                ",
            "username": "Revan",
            "avatar_template": "/letter_avatar_proxy/v4/letter/r/87869e/{size}.png",
            "created_at": "2025-06-11T02:27:48.826Z",
            "cooked": "<p data-ln=\"0\"><div class=\"lightbox-wrapper\"><a class=\"lightbox\" href=\"https://xjtu.app/uploads/default/original/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87.jpeg\" data-download-href=\"https://xjtu.app/uploads/default/5cfd628ded8a3417514f4f53975f03f34af2fe87\" title=\"IMG_6134\"><img src=\"https://xjtu.app/uploads/default/optimized/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87_2_667x500.jpeg\" alt=\"IMG_6134\" data-base62-sha1=\"dgCNJfhQxsEBbjUcM44xLgSCffV\" width=\"667\" height=\"500\" srcset=\"https://xjtu.app/uploads/default/optimized/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87_2_667x500.jpeg, https://xjtu.app/uploads/default/original/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87.jpeg 1.5x, https://xjtu.app/uploads/default/original/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87.jpeg 2x\" data-dominant-color=\"F0F0F0\"><div class=\"meta\"><svg class=\"fa d-icon d-icon-far-image svg-icon\" aria-hidden=\"true\"><use href=\"#far-image\"></use></svg><span class=\"filename\">IMG_6134</span><span class=\"informations\">854×640 37.7 KB</span><svg class=\"fa d-icon d-icon-discourse-expand svg-icon\" aria-hidden=\"true\"><use href=\"#discourse-expand\"></use></svg></div></a></div></p>",
            "external_id": "z76DKUCtlQRcPHx4",
            "post_number": 29,
            "post_type": 1,
            "posts_count": 29,
            "updated_at": "2025-06-11T02:27:48.826Z",
            "reply_count": 0,
            "reply_to_post_number": null,
            "quote_count": 0,
            "incoming_link_count": 0,
            "reads": 1,
            "readers_count": 0,
            "score": 0,
            "yours": false,
            "topic_id": 14752,
            "topic_slug": "topic",
            "topic_title": "【破防楼】期末周限定",
            "topic_html_title": "【破防楼】期末周限定",
            "category_id": 4,
            "display_username": "                ",
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
            "user_title": "黄金",
            "title_is_group": false,
            "bookmarked": false,
            "raw": "![IMG_6134|667x500](upload://dgCNJfhQxsEBbjUcM44xLgSCffV.jpeg)",
            "actions_summary": [],
            "moderator": false,
            "admin": false,
            "staff": false,
            "user_id": 1590,
            "hidden": false,
            "trust_level": 3,
            "deleted_at": null,
            "user_deleted": false,
            "edit_reason": null,
            "can_view_edit_history": false,
            "wiki": false,
            "excerpt": "<a class=\"lightbox\" href=\"https://xjtu.app/uploads/default/original/3X/5/c/5cfd628ded8a3417514f4f53975f03f34af2fe87.jpeg\" data-download-href=\"https://xjtu.app/uploads/default/5cfd628ded8a3417514f4f53975f03f34af2fe87\" title=\"IMG_6134\">[IMG_6134]</a>",
            "truncated": true,
            "mentioned_users": [],
            "post_url": "/t/topic/14752/29",
            "animated_avatar": null,
            "journal": null,
            "reactions": [],
            "current_user_reaction": null,
            "reaction_users_count": 0,
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

            // Process all posts in one batch
            const insertPromises = latest_posts.map(item => {
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
                    like_count: item.like_count || null,
                    word_count: item.word_count || null // not found in json
                };
                return db.insert(schema.posts).values(_item)
                    .onConflictDoUpdate({ target: schema.posts.id, set: _item });
            });

            // Wait for all inserts to complete
            return Promise.all(insertPromises);
        } catch (error) {
            console.error('Error fetching latest posts:', error);
            return Promise.resolve(); // Return a resolved promise even on error
        }
    }
    async function fetchLatestTopics() {
        try {
            let url = 'http://127.0.0.1:4805/latest.json';
            if (currentPage != 1) {
                url += `?no_definitions=true&page=${currentPage-1}`
            }

            let json = await fetch(url).then(r => r.json()).catch(e => console.error(e));
            console.log('json', json);

            if (!json || !json.users || !json.topic_list) {
                console.error('Invalid response from latest.json');
                return Promise.resolve();
            }

            let users = json.users;
            /*
            {
            "id": 5905,
            "username": "PipaQinse233",
            "name": "PipaQinse233",
            "avatar_template": "/user_avatar/xjtu.app/pipaqinse233/{size}/11241_2.png",
            "trust_level": 3,
            "animated_avatar": null
            },
             */

            // Process all users in one batch
            const userPromises = users.map(item => {
                let _item = {
                    id: item.id,
                    username: item.username,
                    name: item.name,
                    avatar_template: item.avatar_template,
                    trust_level: item.trust_level,
                };
                return db.insert(schema.users).values(_item)
                    .onConflictDoUpdate({ target: schema.users.id, set: _item });
            });

            // Wait for all user inserts to complete
            await Promise.all(userPromises);

            //   "more_topics_url": "/latest?no_definitions=true&page=1",
            //    "per_page": 30,
            // const NUM_POSTS_PER_PAGE = 30;
            let topic_list = json.topic_list;
            let categories = topic_list.categories || [];
            /*
                  {
            "id": 4,
            "name": "闲聊吹水",
            "slug": "general",
            "color": "25AAE2",
            "text_color": "FFFFFF",
            "style_type": "emoji",
            "icon": "droplet",
            "emoji": "ocean",
            "read_restricted": false
            },
             */
            let topics = topic_list.topics || [];
            /*
                  {
            "id": 14752,
            "title": "【破防楼】期末周限定",
            "fancy_title": "【破防楼】期末周限定",
            "slug": "topic",
            "posts_count": 28,
            "external_id": "JX3lSIjfOKgsJUAs",
            "reply_count": 14,
            "highest_post_number": 28,
            "image_url": null,
            "created_at": "2025-06-10T15:43:31.277Z",
            "last_posted_at": "2025-06-11T02:22:59.557Z",
            "bumped": true,
            "bumped_at": "2025-06-11T02:22:59.557Z",
            "archetype": "regular",
            "unseen": false,
            "pinned": false,
            "unpinned": null,
            "excerpt": "从 怎么办？｜一个愿为他人知晓的日记楼 (Part 2) 继续讨论： \n期末破防楼，后续等期末考完再说（） \n俺不中嘞，我先来：还有十个小时就考高数，而现在才看到重积分，我现在要 \n:sanguosha::sanguosha:&hellip;",
            "visible": true,
            "closed": false,
            "archived": false,
            "bookmarked": null,
            "liked": null,
            "tags": [],
            "tags_descriptions": {},
            "views": 57,
            "like_count": 78,
            "has_summary": false,
            "last_poster_username": "CR450BF-5033",
            "category_id": 4,
            "pinned_globally": false,
            "featured_link": null,
            "journal": null,
            "has_accepted_answer": false,
            }
             */

            // Process all topics in one batch
            const topicPromises = topics.map(item => {
                let _item = {
                    id: item.id,
                    category_id: item.category_id,
                    category_name: categories.find(c => c.id === item.category_id)?.name,
                    title: item.title,
                    excerpt: item.excerpt,
                    created_at: new Date(item.created_at),
                    last_posted_at: new Date(item.last_posted_at),
                    updated_at: new Date(item.bumped_at),
                    views: item.views,
                    posts_count: item.posts_count,
                    like_count: item.like_count,
                    user_id: item.posters?.find(p => p.description === "原始发帖人")?.user_id,
                    last_post_user_id: item.posters?.find(p => p.description === "最新发帖人")?.user_id,
                    tags: item.tags,
                };
                return db.insert(schema.topics).values(_item)
                    .onConflictDoUpdate({ target: schema.topics.id, set: _item });
            });

            // Wait for all topic inserts to complete and return the promise
            await Promise.all(topicPromises);
            await loadTopics();
        } catch (error) {
            console.error('Error fetching latest topics:', error);
            return Promise.resolve(); // Return a resolved promise even on error
        }
    }

    function handlePageChange(page: number) {
        if ($isLoading) return; // Prevent multiple calls while loading

        isLoading.set(true);
        currentPage = page;
        window.currentTopicPage = page;
        console.log("Page changed to:", page);

        // Load topics and then fetch latest data
        loadTopics()
            .then(() => fetchLatest())
            .then(() => {
                isLoading.set(false);
                window.scrollTo({left: 0, top: 0, behavior: 'smooth'});
            });
    }
    import { tick } from 'svelte';
    let scrollHandler;

    onMount(async () => {
        isLoading.set(true);
        const container = document.querySelector("#container");
        console.log("component mounted");

        // First load existing topics from database
        await loadTopics();

        // Then fetch latest data from API
        await fetchLatest();

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
            // console.log("Window scroll:", window.scrollY);
            if (window.scrollY > 100) {
                window.currentTopicPageScrollY = window.scrollY;
            }
        };

        window.addEventListener("scroll", scrollHandler);

        isLoading.set(false);

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


<!--    <div class="flex justify-center">-->
<!--       <span class="me-4"> <strong>Topic Browser</strong></span>-->
<!--        {#if totalPages>1}-->
<!--        <PaginationNav visiblePages={Math.min(visiblePagesTop, totalPages)} {currentPage} {totalPages} onPageChange={handlePageChange}>-->
<!--            {#snippet prevContent()}-->
<!--                <span class="sr-only">Previous</span>-->
<!--                <Fa icon={faArrowLeft} />-->
<!--            {/snippet}-->
<!--            {#snippet nextContent()}-->
<!--                <span class="sr-only">Next</span>-->
<!--                <Fa icon={faArrowRight} />-->
<!--            {/snippet}-->
<!--        </PaginationNav>-->
<!--        {/if}-->
<!--    </div>-->

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
                                {#if user?.username}
                                    <p>by {user?.username}</p>
                                {/if}
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
