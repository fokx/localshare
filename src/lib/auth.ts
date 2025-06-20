import {invoke} from '@tauri-apps/api/core';
import {load} from '@tauri-apps/plugin-store';


export interface OauthUser {
    id: string;
    name: string;
    email: string;
    avatar_url?: string;
    provider: string;
    accessToken: string;
    username: string;
    admin: boolean;
    moderator: boolean;
    groups: Array<string>;
    user_global_api_key: string;
}


let currentUser: OauthUser | null = null;
let store: Awaited<ReturnType<typeof load>> | null = null;


async function getStore() {
    try {
        if (!store) {
            store = await load('user-store.json', {autoSave: true});
        }
        return store;
    } catch (error) {
        console.error('Failed to get store:', error);
        throw error;
    }

}


export async function login(provider: 'google' | 'github' | 'discourse'): Promise<OauthUser> {
    try {
        console.log('calling login_with_provider', currentUser);

        const userInfo = await invoke<{
            id: string;
            name: string;
            email: string;
            avatar_url: string | null;
            provider: string;
            access_token: string;
            username: string;
            admin: boolean;
            moderator: boolean;
            groups: Array<string>;
            user_global_api_key: string;
        }>('login_with_provider', {provider});
        console.log('called login_with_provider', currentUser);

        currentUser = {
            id: userInfo.id,
            name: userInfo.name,
            email: userInfo.email,
            avatar_url: userInfo.avatar_url || undefined,
            provider: userInfo.provider as 'google' | 'github' | 'discourse',
            accessToken: userInfo.access_token,
            username: userInfo.username,
            admin: userInfo.admin,
            moderator: userInfo.moderator,
            groups: userInfo.groups,
            user_global_api_key: userInfo.user_global_api_key,
        };
        console.log('currentUser', currentUser);

        // Store user in Tauri Store
        const store = await getStore();
        await store.set('user', currentUser);
        await store.save();
        console.log('User logged in:', currentUser);

        return currentUser;
    } catch (error) {
        console.error('Login failed:', error);
        throw error;
    }
}


export async function getCurrentUser(): Promise<OauthUser | null> {
    if (!currentUser) {
        try {
            const store = await getStore();
            currentUser = await store.get<OauthUser>('user') || null;
        } catch (error) {
            console.error('Failed to get stored user:', error);
        }
    }
    return currentUser;
}


export async function logout(): Promise<void> {
    currentUser = null;
    const store = await getStore();
    await store.delete('user');
    await store.save();
    console.log('User logged out');
}