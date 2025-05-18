// import * as fs from 'node:fs';
import {promises as fs} from 'node:fs';
import Database from 'better-sqlite3';
import {drizzle} from 'drizzle-orm/better-sqlite3';
// import { drizzle } from "drizzle-orm/sqlite-proxy";
import * as schema from './schema';
import {eq} from "drizzle-orm";

const client = new Database("../../../sqlite:test.db");

export const db = drizzle(client, {schema});
function import_users() {
    let filePath = '../../../res/users.json';
    fs.readFile(filePath, 'utf8').then(async (data) => {
        const items = JSON.parse(data);
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
            item.updated_at = new Date(item.updated_at);
            item.silenced_till = new Date(item.silenced_till);
        });
        for (const item of items) {
            await db.insert(schema.users).values(item).onConflictDoUpdate({target: schema.users.id, set: item});
        }
    });
}

function import_topics() {
    let filePath = '../../../res/topics.json';
    fs.readFile(filePath, 'utf8').then(async (data) => {
        const items = JSON.parse(data);
        items.forEach((item) => {
            item.created_at = new Date(item.created_at);
            item.last_posted_at = new Date(item.last_posted_at);
            item.updated_at = new Date(item.updated_at);
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

import_users()
import_topics()
import_posts()
import_likes()