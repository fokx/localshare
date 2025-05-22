import dayjs from 'dayjs';
import relativeTime from 'dayjs/plugin/relativeTime';
import {db} from "$lib/db/database";
import Dexie from 'dexie';

export const dbb = new Dexie('xap');

dbb.version(1).stores({
    browse_history: '&topic_id',
});


export function generateRandomString(length: number) {
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


// https://icons.getbootstrap.com/icons/check2-square/
const html_checked_square = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="display: inline bi bi-check2-square" viewBox="0 0 16 16">
  <path d="M3 14.5A1.5 1.5 0 0 1 1.5 13V3A1.5 1.5 0 0 1 3 1.5h8a.5.5 0 0 1 0 1H3a.5.5 0 0 0-.5.5v10a.5.5 0 0 0 .5.5h10a.5.5 0 0 0 .5-.5V8a.5.5 0 0 1 1 0v5a1.5 1.5 0 0 1-1.5 1.5z"/>
  <path d="m8.354 10.354 7-7a.5.5 0 0 0-.708-.708L8 9.293 5.354 6.646a.5.5 0 1 0-.708.708l3 3a.5.5 0 0 0 .708 0"/>
</svg>`;
// https://icons.getbootstrap.com/icons/square/
const html_unchecked_square = `<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" class="display: inline bi bi-square" viewBox="0 0 16 16">
  <path d="M14 1a1 1 0 0 1 1 1v12a1 1 0 0 1-1 1H2a1 1 0 0 1-1-1V2a1 1 0 0 1 1-1zM2 0a2 2 0 0 0-2 2v12a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V2a2 2 0 0 0-2-2z"/>
</svg>`;

export function process_cooked(cooked: string) {
    if (cooked === undefined || cooked === null || cooked === '' ) {
        return '';
    }
    cooked = cooked.replaceAll(`<span class="chcklst-box checked fa fa-square-check-o fa-fw">`,html_checked_square+`<span class="chcklst-box checked">`);
    cooked = cooked.replaceAll(`<span class="chcklst-box fa fa-square-o fa-fw">`,html_unchecked_square+`<span class="chcklst-box unchecked">`);
    // console.log(cooked);
    // const dom = htmlparser2.parseDocument(cooked);
    return cooked

    // const preElements = domutils.findAll((elem) => elem.tagName === 'pre', dom.children);
    // preElements.forEach((preElem) => {
    // 	const codeElements = domutils.findAll((elem) => elem.tagName === 'code', [preElem]);
    // 	if (codeElements.length > 0) {
    // 		// let removed = dom_render(preElem, { encodeEntities : 'utf8' });
    // 		// let new_el= hljs.highlightElement(preElem).value;
    // 		// domutils.replaceElement(preElem, new_el);
    // 	}
    // });

    // let html = dom_render(dom, { encodeEntities : 'utf8' });

    // console.log(html);
    // return html;
}

export function display_time(d) {
    dayjs.extend(relativeTime);
    return dayjs(dayjs(d)).fromNow();
}

export async function getUserById(user_id: number) {
    let user = await db.query.users.findFirst({
        where: {id: user_id}
    });
    // console.log(user);
    return user;
}
