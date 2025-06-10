import 'dotenv/config';
import Discourse from "discourse2";

console.log(process.env.DISCOURSE_API_HOST);

const discourse = new Discourse(process.env.DISCOURSE_API_HOST, {
  "Api-Key": process.env.DISCOURSE_API_KEY,
  "Api-Username": process.env.DISCOURSE_API_USERNAME,
});

const result = await discourse.listLatestTopics();
console.log(result);
