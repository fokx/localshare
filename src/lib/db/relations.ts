import { defineRelations } from "drizzle-orm";
import * as schema from './schema'


export const relations = defineRelations(schema, (r) => ({
    topics: {
        posts: r.many.posts({
            from: r.topics.id,
            to: r.posts.topic_id,
        }),
    },
   posts: {
       likes: r.many.likes({
           from: r.posts.id,
           to: r.likes.post_id,
       }),
   },
    users: {
        posts: r.many.posts({
            from: r.users.id,
            to: r.posts.user_id,
        }),
        topics: r.many.topics({
            from: r.users.id,
            to: r.topics.user_id,
        }),
        topics_last_post: r.many.topics({
            from: r.users.id,
            to: r.topics.last_post_user_id,
        }),
        likes: r.many.likes({
            from: r.users.id,
            to: r.likes.user_id,
        }),
        sessions: r.many.sessions({
            from: r.users.id,
            to: r.sessions.user_id,
        })
    },

}));