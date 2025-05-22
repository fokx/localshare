<script lang="ts">
    import { appConfigDir, join, resourceDir, appLocalDataDir, appCacheDir, documentDir } from "@tauri-apps/api/path";
    import { onMount } from "svelte";
    import { openPath } from "@tauri-apps/plugin-opener";
    import { writeText } from "@tauri-apps/plugin-clipboard-manager";
    import * as schema from "$lib/db/schema";
    import { db } from "$lib/db/database";
    import Inspect from "svelte-inspect-value";
    import {Card, Heading} from "flowbite-svelte";
    import {display_time, process_cooked} from "$lib";
    import {eq} from "drizzle-orm";
    import {users} from "$lib/db/schema";
    import {goto} from "$app/navigation";

    let appConfigPath = $state("");
    let dbPath = $state("");
    let nameInput = $state("");
    let topics = $state<
        { id: number; created_at: string | null; raw: string | null }[]
    >([]);
    onMount(async () => {
        const path = await documentDir();
        appConfigPath = path;
        dbPath = await join(path, "xap.db");
        loadTopics();
    });
    const loadTopics = async () => {
        db.query.topics
            .findMany({
                limit: 50,
            })
            .execute()
            .then((results) => {
                console.log("🚀 ~ FindMany response from Drizzle:", results);
                topics = results;
            });
    };

    async function addTopic() {
        await db.insert(schema.topics).values({ name: nameInput });
        nameInput = "";
        loadTopics();
    }
    async function getUserName(user_id: number) {
        let user = await db.query.users.findFirst({
            where: {id: user_id}
        });
        console.log(user);
        return user;
    }
</script>

<main class="container mx-auto flex flex-col gap-4">
<!--    <div class="flex gap-2">-->
<!--        <button-->
<!--                class="font-mono text-sm text-blue-400 hover:text-blue-500 hover:underline cursor-pointer text-left"-->
<!--                onclick={() => {-->
<!--        openPath(appConfigPath)-->
<!--          .then(() => {-->
<!--            console.log("opened");-->
<!--          })-->
<!--          .catch((err) => {-->
<!--            console.error(err);-->
<!--          });-->
<!--      }}-->
<!--        >-->
<!--            {dbPath}-->
<!--        </button>-->
<!--        <button-->
<!--                type="button"-->
<!--                class="btn preset-filled btn-sm"-->
<!--                onclick={async () => {-->
<!--        await writeText(dbPath);-->
<!--      }}-->
<!--        >-->
<!--            Copy-->
<!--        </button>-->
<!--    </div>-->

<!--    <form-->
<!--            onsubmit={(e) => {-->
<!--      e.preventDefault();-->
<!--      addTopic();-->
<!--    }}-->
<!--    >-->
<!--        <label class="label">-->
<!--            <span class="label-text">Name</span>-->
<!--            <div class="flex gap-2">-->
<!--                <input-->
<!--                        bind:value={nameInput}-->
<!--                        class="input"-->
<!--                        type="text"-->
<!--                        placeholder="Enter a name..."-->
<!--                />-->
<!--                <button type="submit" class="btn preset-filled">-->
<!--                    Add name to the db-->
<!--                </button>-->
<!--            </div>-->
<!--        </label>-->
<!--    </form>-->
    <Heading tag="h5" class="text-primary-700 dark:text-primary-500">
        Topic Browser
    </Heading>
    {#if topics}
        {#each topics as topic}
            <div class="flex-grow justify-center dotted-ul prose dark:prose-invert">
                <Card class="max-w-3xl mb-2 bg-gray-500" onclick={()=>{window.current_topic_id=topic.id; goto("/topic"); console.log(window.current_topic_id)}}>
                    {#if topic.title}
                        <div class="flex justify-center">
                            <h5 class="me-6 mb-2 text-2xl font-bold tracking-tight">{topic.title}</h5>
                            in {topic.category_name}
                            &nbsp;
                            {#await getUserName(topic.user_id) then user}
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
</main>
