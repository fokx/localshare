// import * as fs from 'node:fs';
import {promises as fs} from 'node:fs';
import Database from 'better-sqlite3';
import {drizzle} from 'drizzle-orm/better-sqlite3';
// import { drizzle } from "drizzle-orm/sqlite-proxy";
import * as schema from './schema';
import {eq} from 'drizzle-orm';
import path from 'path';
const client = new Database("../../../src-tauri/res/xap.db");

export const db = drizzle(client, {schema});

async function import_sth(filename: string, table, fields_convert_date: Array<string>) {
    console.log(`importing ${filename} to "${table}";`);
    let filePath = path.join('../../../src-tauri/res/', filename);
    await fs.readFile(filePath, 'utf8').then(async (data) => {
        const items = JSON.parse(data);
        const batchSize = 2000;
        const length = items.length;
        let batch = [];
        // Process date fields for all items at once
        // items.forEach((item) => {
        //     fields_convert_date.forEach((field) => {
        //         item[field] = new Date(item[field]);
        //
        //     });
        //     // item.created_at = new Date(item.created_at);
        //     // item.updated_at = new Date(item.updated_at);
        //     // item.silenced_till = new Date(item.silenced_till);
        // });
        const updatedItems = items.map(item => {
            return {
                ...item,
                ...Object.fromEntries(
                    fields_convert_date.map(field => [field, new Date(item[field])])
                )
            };
        });

        // db.transaction((trx) => {
            for (let i = 0; i < length; i++) {
                const item = updatedItems[i];
                batch.push(item);

                if (batch.length === batchSize || i === length - 1) {
                    await db.insert(table).values(batch);//.onConflictDoUpdate({ target: schema.users.id, set: item });
                    batch = [];
                }
            }
        // });

        // db.insert(schema.users)
        //     .values(items)
        //     .run();
    })
}

function import_users() {
    let filePath = '../../../res/users.json';
    fs.readFile(filePath, 'utf8').then((data) => {
        const items = JSON.parse(data);
        const batchSize = 2000; // Adjust batch size as needed
        const length = items.length;
        let batch = [];
        // Process date fields for all items at once
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
            item.updated_at = new Date(item.updated_at);
            item.silenced_till = new Date(item.silenced_till);
        });

        db.transaction((trx) => {
            for (let i = 0; i < length; i++) {
                const item = items[i];
                batch.push(item);

                if (batch.length === batchSize || i === length - 1) {
                    trx.insert(schema.users).values(batch);//.onConflictDoUpdate({ target: schema.users.id, set: item });
                    batch = [];
                }
            }
        });

        // db.insert(schema.users)
        //     .values(items)
        //     .run();
    });
}

function import_topics() {
    let filePath = '../../../res/topics.json';
    fs.readFile(filePath, 'utf8').then(async (data) => {
        const items = JSON.parse(data);
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
            item.updated_at = new Date(item.updated_at);
            item.last_posted_at = new Date(item.last_posted_at);
            // item.tags = null;
        });
        for (const item of items) {
            try {
                await db.insert(schema.topics).values(item).onConflictDoUpdate({target: schema.topics.id, set: item});
            } catch (error) {
                console.error("Insert failed:", {
                    error,
                    data,
                    errorName: error.name,
                    errorCode: error.code,
                    errorMessage: error.message,
                    stack: error.stack
                });
            }
        }
    });
}

function import_posts() {
    let filePath = '../../../res/posts.json';
    fs.readFile(filePath, 'utf8').then(async (data) => {
        let empty_user_ids = [];
        let empty_topic_ids = [];
        const items = JSON.parse(data);
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
            item.updated_at = new Date(item.updated_at);
        });
        let omitted_posts=  [];
        for (const item of items) {
            // let post = await db.select().from(schema.posts).where(eq(schema.posts.id, item.id));
            // if (post.length === 0) {
            //     omitted_posts.push(item);
            // }
            // continue

            // if (item.id==16938){
            //     console.log()
            // }
            // if (item.topic_id){
            //     let topic = await db.select().from(schema.users).where(eq(schema.users.id, item.topic_id));
            //     if (topic.length === 0) {
            //         console.log("topic not found")
            //         empty_topic_ids.push(item.id);
            //     }
            // }
            // if (item.reply_to_user_id){
            //     let user = await db.select().from(schema.users).where(eq(schema.users.id, item.user_id));
            //     if (user.length === 0) {
            //         console.log("user not found");
            //         empty_user_ids.push(item.id);
            //     }
            // }

            // console.log(topic, user, reply_to_user);
           try {
                // await db.transaction(async (tx) => {
                //     // Insert the post inside the transaction
                //     await tx.insert(schema.posts).values(item).run();
                //
                //     // Log the result (optional)
                //     console.log("Post inserted:", result);
                // });

               await db.insert(schema.posts).values(item).onConflictDoUpdate({target: schema.posts.id, set: item}); }
             catch (error) {
                 console.log(error);

               }
            //     console.error("Insert failed:", {
            //         error,
            //         data,
            //         errorName: error.name,
            //         errorCode: error.code,
            //         errorMessage: error.message,
            //         stack: error.stack
            //     });
            //  }
        }
        // console.log("omitted_posts", omitted_posts);
        // console.log("empty_topic_ids", empty_topic_ids);
        // console.log("empty_user_ids", empty_user_ids);
    });

}


function import_likes() {
    let filePath = '../../../res/likes.json';
    fs.readFile(filePath, 'utf8').then(async (data) => {
        const items = JSON.parse(data);
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
        });
        for (const item of items) {
            try {
                await db.insert(schema.likes).values(item);
            } catch (error) {
                console.error("Insert failed:", {
                    error,
                    data,
                    errorName: error.name,
                    errorCode: error.code,
                    errorMessage: error.message,
                    stack: error.stack
                });
            }
        }
    });
}

// because of foreign key constraints, the data import order is:
await import_sth('users.json', schema.users, ['created_at', 'updated_at', 'silenced_till']);
await import_sth('topics.json', schema.topics, ['created_at', 'updated_at', 'last_posted_at']);
await import_sth('posts.json', schema.posts, ['created_at', 'updated_at']);
await import_sth('likes.json', schema.likes, ['created_at']);
// import_users()
// import_topics()
// import_posts()
// import_likes()