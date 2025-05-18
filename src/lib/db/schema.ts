import {eq, sql} from "drizzle-orm";
import {index, integer, sqliteTable, sqliteView, text, uniqueIndex} from 'drizzle-orm/sqlite-core';

const common_timestamps = {
    created_at: integer('created_at', {mode: 'timestamp'}).default(sql`(current_timestamp)`),
    deleted_at: integer('deleted_at', {mode: 'timestamp'}),
    updated_at: integer('updated_at', {mode: 'timestamp'}).$onUpdate(() => new Date())
};

export const users = sqliteTable('users', {
    id: integer('id').primaryKey().notNull(),
    username: text('username',).notNull().unique(),
    // passwordHash: text('password_hash').notNull(),
    name: text('name'),
    admin: integer('admin', {mode: 'boolean'}),
    moderator: integer('moderator', {mode: 'boolean'}),
    trust_level: integer('trust_level'),
    avatar_template: text('avatar_template'),
    title: text('title'),
    groups: text('groups', { mode: 'json' })
        .$type<string[]>()
        .default(sql`(json_array())`),
    locale: text('locale'),
    silenced_till: integer('silenced_till', {mode: 'timestamp'}),
    staged: integer('staged', {mode: 'boolean'}),
    active: integer('active', {mode: 'boolean'}),
    created_at: integer('created_at', {mode: 'timestamp'}),
    updated_at: integer('updated_at', {mode: 'timestamp'}),
    // ...common_timestamps
});

export const topics = sqliteTable('topics', {
    id: integer('id').primaryKey().notNull(),
    category_name: text('category_name'),
    category_id: integer('category_id'),
    title: text('title'),
    excerpt: text('excerpt'),
    created_at: integer('created_at', {mode: 'timestamp'}),
    last_posted_at: integer('last_posted_at', {mode: 'timestamp'}),
    updated_at: integer('updated_at', {mode: 'timestamp'}),
    views: integer('views'),
    posts_count: integer('posts_count'),
    like_count: integer('like_count'),
    user_id: integer('user_id').references(() => users.id),
    last_post_user_id: integer('last_post_user_id').references(() => users.id),
    tags: text('tags', { mode: 'json' })
        .$type<string[]>()
        .default(sql`(json_array())`),
});

function generateRandomString(length: number) {
    let result = '';
    const characters = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789';
    const charactersLength = characters.length;
    let counter = 0;
    while (counter < length) {
        result += characters.charAt(Math.floor(Math.random() * charactersLength));
        counter += 1;
    }
    return result;
}

function GeneratePostId() {
    return generateRandomString(16);
}

export const posts = sqliteTable('posts', {
    id: integer('id').primaryKey().notNull(),
    raw: text('raw'),
    cooked: text('cooked'),
    post_number: integer('post_number'),
    topic_id: integer('topic_id').references(() => topics.id),
    user_id: integer('user_id').references(() => users.id),
    // username: text('username').notNull(),
    created_at: integer('created_at', {mode: 'timestamp'}),
    updated_at: integer('updated_at', {mode: 'timestamp'}),
    reply_to_post_number: integer('reply_to_post_number'),
    reply_to_user_id: integer('reply_to_user_id').references(() => users.id),
    reply_count: integer('reply_count'),
    like_count: integer('like_count'),
    word_count: integer('word_count'),
    // deleted: integer('deleted', { mode: 'boolean' }).$default(() => false),
    // is_main_post: integer('is_main_post', { mode: 'boolean' }),
    // main_post_id: text('main_post_id'),
    // reply_to_post_id: text('reply_to_post_id'),
}, (table) => [
    index("idxTopic").on(table.topic_id, table.post_number)
]);

export const likes = sqliteTable('likes', {
    post_id: integer('post_id').references(() => posts.id),
    user_id: integer('user_id').references(() => users.id),
    created_at: integer('created_at', {mode: 'timestamp'}).notNull()
}, (table) => [
    uniqueIndex("idxLikes").on(table.post_id, table.user_id)
]);

export const sessions = sqliteTable('session', {
    id: text('id').primaryKey(),
    user_id: integer('user_id')
        .notNull()
        .references(() => users.id),
    expires_at: integer('expires_at', {mode: 'timestamp'}).notNull()
});

export type User = typeof users.$inferSelect;
export type Topic = typeof topics.$inferSelect;
export type Post = typeof posts.$inferSelect;
export type Like = typeof likes.$inferSelect;
export type Session = typeof sessions.$inferSelect;

// export const topics_view = sqliteView('topics_view').as((qb) =>
//     qb.select().from(posts).where(eq(posts.post_number, 1))
// );

