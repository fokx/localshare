<script lang="ts">
    import {Avatar, Button, Card, Heading, Spinner, Input, Listgroup, ListgroupItem, Badge} from "flowbite-svelte";
    import {siteTitle} from "$lib";
    import {invoke} from '@tauri-apps/api/core';
    import {emit, listen} from '@tauri-apps/api/event';

    import {onMount, onDestroy} from 'svelte';
    import {toast} from "@zerodevx/svelte-toast";
    import Fa from 'svelte-fa';
    import {faPaperPlane, faCircle} from '@fortawesome/free-solid-svg-icons';
    import {load, type Store} from '@tauri-apps/plugin-store';


    // Types
    interface ChatMessage {
        id: string;
        sender_fingerprint: string;
        sender_alias: string;
        receiver_fingerprint: string;
        content: string;
        timestamp: string;
        read: boolean;
    }

    interface ChatSession {
        peer_fingerprint: string;
        peer_alias: string;
        last_message: ChatMessage | null;
        unread_count: number;
        color: string;
    }

    interface ChatSessions {
        sessions: Record<string, ChatSession>;
    }

    interface ChatHistory {
        messages: ChatMessage[];
    }

    // State
    let chatSessions = $state<ChatSessions>({ sessions: {} });
    let selectedPeer = $state<string | null>(null);
    let chatHistory = $state<ChatMessage[]>([]);
    let newMessage = $state('');
    let peers = $state([]);
    let isLoading = $state(true);
    let announce_btn_disable = $state(false);
    async function announce_once() {
        // change the button color gradully to gray and then back to blue
        announce_btn_disable = true;
        setTimeout(() => {
            announce_btn_disable = false;
        }, 1000);
        await invoke("announce_once");
    }
    let settings_store: Store<any>;
    let current_settings;
    let savingDir = $state("/storage/emulated/0/");
    let fingerprint = $state("");

    // Add this flag as a state
    let skipSessionReload = $state(false);

   // Reference to chat container and textarea
   let chatContainer;
   let textareaElement;

   // Scroll to bottom function
   function scrollToBottom() {
       if (chatContainer) {
           chatContainer.scrollTop = chatContainer.scrollHeight;
       }
   }

   // Auto-resize textarea function
   function autoResize() {
       if (textareaElement) {
           // Reset height to auto to get the correct scrollHeight
           textareaElement.style.height = 'auto';

           // Set the height to the scrollHeight (content height)
           const newHeight = Math.min(textareaElement.scrollHeight, 150); // 150px is approximately 5 lines
           textareaElement.style.height = `${newHeight}px`;
       }
   }

   // Load chat sessions and listen for new messages
    onMount(async () => {
        // Scroll to bottom when chat is mounted
        scrollToBottom();

        // Initialize textarea height if it exists
        setTimeout(() => {
            if (textareaElement) {
                autoResize();
            }
        }, 0);

        // Set the bottom nav height CSS variable
        const setBottomNavHeight = () => {
            // Check if we're on a large screen (sm breakpoint in Tailwind is 640px)
            const isLargeScreen = window.innerWidth >= 640;

            // On large screens, always set bottom-nav-height to 0
            if (isLargeScreen) {
                document.documentElement.style.setProperty('--bottom-nav-height', '0px');
                console.log('Large screen detected, setting bottom-nav-height to 0');
                return;
            }

            // For small screens, continue with the existing logic
            // Try different possible selectors for the bottom navbar
            const bottomNav = document.querySelector('.bottom-nav, [class*="bottom-nav"], nav[class*="bottom"], .flowbite-bottom-nav, div[class*="flowbite-bottom-nav"], div[class*="bottom-nav"], nav.sticky, nav.fixed, nav[class*="sm:hidden"]');

            // Debug: Log all elements that match our selectors individually
            console.log('Debugging bottom nav selectors:');
            ['.bottom-nav', '[class*="bottom-nav"]', 'nav[class*="bottom"]', '.flowbite-bottom-nav', 
             'div[class*="flowbite-bottom-nav"]', 'div[class*="bottom-nav"]', 'nav.sticky', 'nav.fixed', 
             'nav[class*="sm:hidden"]'].forEach(selector => {
                const elements = document.querySelectorAll(selector);
                console.log(`Selector "${selector}" matched ${elements.length} elements`);
                elements.forEach((el, i) => {
                    console.log(`  Element ${i+1}: tag=${el.tagName}, classes=${el.className}, height=${el.offsetHeight}`);
                });
            });

            if (bottomNav) {
                const bottomNavHeight = bottomNav.offsetHeight;
                document.documentElement.style.setProperty('--bottom-nav-height', `${bottomNavHeight}px`);
                console.log('Bottom nav found:', bottomNav);
                console.log('Bottom nav height set to:', bottomNavHeight);
                console.log('Bottom nav classes:', bottomNav.className);
            } else {
                // On small screens, use a default value of 60px if we can't find the bottom navbar
                document.documentElement.style.setProperty('--bottom-nav-height', '60px');
                console.log('Bottom nav not found on small screen, using default height of 60px');

                // Try a more direct approach - look for the BottomNav in the layout
                const allNavs = document.querySelectorAll('nav');
                console.log(`Found ${allNavs.length} nav elements on the page:`);
                allNavs.forEach((nav, i) => {
                    console.log(`  Nav ${i+1}: classes=${nav.className}, height=${nav.offsetHeight}`);
                });

                let bottomNavItems = document.querySelectorAll('[id="bottom-nav-bar"]');
                if (bottomNavItems.length > 0) {
                    console.log(`Found ${bottomNavItems.length} bottom nav items`);
                    // Find the parent element that might be the bottom nav
                    const possibleBottomNav = bottomNavItems[0];
                    console.log('possibleBottomNav', possibleBottomNav);
                    if (possibleBottomNav) {
                        const bottomNavHeight = possibleBottomNav.offsetHeight;
                        document.documentElement.style.setProperty('--bottom-nav-height', `${bottomNavHeight}px`);
                        console.log('Possible bottom nav found via bottom nav items:', possibleBottomNav);
                        console.log('Bottom nav height set to:', bottomNavHeight);
                        console.log('Bottom nav classes:', possibleBottomNav.className);
                    }
                }
            }
        };

        // Initial setting
        setTimeout(setBottomNavHeight, 100); // Small delay to ensure DOM is fully rendered

        // Update on resize
        window.addEventListener('resize', setBottomNavHeight);

        // Add a mutation observer to detect when the bottom nav might be added/removed from DOM
        const observer = new MutationObserver(setBottomNavHeight);
        observer.observe(document.body, { childList: true, subtree: true });

        try {
            settings_store = await load('settings.json', {autoSave: true});
            current_settings = await settings_store.get('localsend');
            savingDir = current_settings.savingDir;
            fingerprint = current_settings.fingerprint;
            siteTitle.set("Chat (" + fingerprint.substring(0, 8) + "...)");

            // Load peers from the peers.json store
            const peersStore = await load('peers.json');
            const peerKeys = await peersStore.keys();
            peers = [];

            for (const key of peerKeys) {
                const peerValue = await peersStore.get(key);
                if (peerValue) {
                    peers.push(peerValue);
                }
            }
            console.log('loadiing peers');
            $state.snapshot(peers);

            // Discover more peers
            await refreshPeers();

            // Load chat sessions
            await loadChatSessions();

            // Listen for new messages
            const unlistenChatMessage = await listen('chat-message-received', (event) => {
                const message = event.payload as ChatMessage;
                console.log('Received chat message:', message);

                // If the message is from the currently selected peer, add it to the chat history
                if (selectedPeer && message.sender_fingerprint === selectedPeer) {
                    chatHistory = [...chatHistory, message];
                    // Mark the message as read
                    markMessagesAsRead(selectedPeer);
                }

                // Refresh chat sessions to update unread counts
                loadChatSessions();

                // Show a toast notification
                toast.push(`New message from ${message.sender_alias}`);
            });

            // Listen for peer discovery events
            const unlistenRefreshPeers = await listen('refresh-peers', async () => {
                // Load updated peers from the store
                const peersStore = await load('peers.json');
                const peerKeys = await peersStore.keys();
                peers = [];

                for (const key of peerKeys) {
                    const peerValue = await peersStore.get(key);
                    if (peerValue) {
                        peers.push(peerValue);
                    }
                }

                console.log('Updated peers:');
                $state.snapshot(peers);
            });

            isLoading = false;

            return () => {
                // Clean up event listeners
                window.removeEventListener('resize', setBottomNavHeight);
                observer.disconnect();

                // Clean up message listeners
                unlistenChatMessage();
                unlistenRefreshPeers();
            };
        } catch (error) {
            console.error('Error in onMount:', error);
            toast.push('Error loading chat data');
            isLoading = false;
        }
    });

    // Load chat sessions from the backend
    async function loadChatSessions() {
        try {
            chatSessions = await invoke('get_chat_sessions');
            console.log('Chat sessions:');
            $state.snapshot(chatSessions);

            // If a peer is selected, load its chat history
            if (selectedPeer) {
                await loadChatHistory(selectedPeer);
            }
        } catch (error) {
            console.error('Error loading chat sessions:', error);
            toast.push('Error loading chat sessions');
        }
    }

    // Load chat history for a specific peer
    async function loadChatHistory(peerFingerprint: string) {
        try {
            const history = await invoke('get_chat_history', { peerFingerprint });
            chatHistory = history.messages;
            console.log('Chat history:');
            $state.snapshot(chatHistory);

            // Mark messages as read
            await markMessagesAsRead(peerFingerprint);
            scrollToBottom(); // Scroll when chat is mounted
        } catch (error) {
            console.error('Error loading chat history:', error);
            toast.push('Error loading chat history');
        }
    }

   // Mark messages as read without reloading sessions
   async function markMessagesAsRead(peerFingerprint: string) {
       try {
           skipSessionReload = true; // Temporarily skip session reload to avoid the loop
           await invoke('mark_messages_as_read', { peerFingerprint });

           // Optionally, update session data in memory without fully reloading
           const session = chatSessions.sessions[peerFingerprint];
           if (session) {
               session.unread_count = 0;
           }

           skipSessionReload = false; // Allow session reload when needed
       } catch (error) {
           console.error('Error marking messages as read:', error);
           skipSessionReload = false;
       }
   }

    // Send a message to a peer
    async function sendMessage() {
        console.log('pre sending ', newMessage, ' to ', selectedPeer);
        if (!selectedPeer || !newMessage.trim()) {
            return;
        }
        console.log('sending ', newMessage, ' to ', selectedPeer);
        try {
            await invoke('send_chat_message', {
                peerFingerprint: selectedPeer,
                content: newMessage
            });

            // Clear the input field
            newMessage = '';

            // Reload chat history to show the sent message
            await loadChatHistory(selectedPeer);
        } catch (error) {
            console.error('Error sending message:', error);
            toast.push('Error sending message');
        }
    }

    // Select a peer to chat with
    async function selectPeer(peerFingerprint: string) {
        selectedPeer = peerFingerprint;
        await loadChatHistory(peerFingerprint);
    }

    // Refresh the list of peers
    async function refreshPeers() {
        try {
            // Announce to discover peers
            await invoke('announce_once');

            // We don't have a direct get_peers function, so we'll listen for the refresh-peers event
            // The peers will be updated through the refresh-peers event listener we set up in onMount
            toast.push('Discovering peers...');
        } catch (error) {
            console.error('Error refreshing peers:', error);
            toast.push('Error discovering peers');
        }
    }

    // Format timestamp for display
    function formatTimestamp(timestamp: string): string {
        const date = new Date(timestamp.secs_since_epoch * 1000);
        return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
    }

    // Handle Enter key in the message input
    function handleKeydown(event: KeyboardEvent) {
        if (event.key === 'Enter' && !event.shiftKey) {
            event.preventDefault();
            sendMessage();
        }
    }

    // Find a peer by fingerprint
    function findPeerByFingerprint(fingerprint: string) {
        return peers.find(p => p.message?.fingerprint === fingerprint);
    }
</script>

<div class="flex flex-col h-screen">
    <div class="flex flex-1 overflow-hidden">
        <!-- Chat sessions sidebar -->
        <div class="w-1/4 border-r border-gray-200 dark:border-gray-700 overflow-y-auto">
            <div class="p-4">
                <Heading tag="h5" class="mb-4">Chat Sessions</Heading>

                {#if isLoading}
                    <div class="flex justify-center my-4">
                        <Spinner size="6" />
                    </div>
                {:else if Object.keys(chatSessions.sessions).length === 0}
                    <p class="text-gray-500 dark:text-gray-400 text-center">
                        No chat sessions yet. Start a chat with a peer.
                    </p>
                {:else}
                    <div class="space-y-2">
                        {#each Object.values(chatSessions.sessions) as session}
                            <div 
                                class="p-3 rounded-lg cursor-pointer transition-colors duration-200 hover:bg-gray-100 dark:hover:bg-gray-700"
                                style="background-color: {session.peer_fingerprint === selectedPeer ? session.color : 'transparent'}; opacity: {session.peer_fingerprint === selectedPeer ? '0.7' : '1'}"
                                onclick={() => selectPeer(session.peer_fingerprint)}
                            >
                                <div class="flex justify-between items-center">
                                    <div class="font-medium">{session.peer_alias}</div>
                                    {#if session.unread_count > 0}
                                        <Badge color="red">{session.unread_count}</Badge>
                                    {/if}
                                </div>
                                {#if session.last_message}
                                    <div class="text-sm text-gray-500 dark:text-gray-400 truncate">
                                        {session.last_message.content}
                                    </div>
                                {/if}
                            </div>
                        {/each}
                    </div>
                {/if}
            </div>

            <div class="p-4 border-t border-gray-200 dark:border-gray-700">
                <Heading tag="h5" class="mb-4">Available Peers</Heading>
                <Button class="w-full mb-4" disabled={announce_btn_disable} onclick={refreshPeers}>Discover Peers</Button>

                {#if peers.length === 0}
                    <p class="text-gray-500 dark:text-gray-400 text-center">
                        No peers found. Click "Discover Peers" to find peers.
                    </p>
                {:else}
                    <div class="space-y-2">
                        {#each peers as peer}
                            {#if peer.message}
                                <div 
                                    class="p-3 rounded-lg cursor-pointer transition-colors duration-200 hover:bg-gray-100 dark:hover:bg-gray-700"
                                    onclick={() => selectPeer(peer.message.fingerprint)}
                                >
                                    <div class="font-medium">{peer.message.alias}</div>
                                    <div class="text-xs text-gray-500 dark:text-gray-400">
                                        {peer.message.fingerprint.substring(0, 8)}...
                                    </div>
                                </div>
                            {/if}
                        {/each}
                    </div>
                {/if}
            </div>
        </div>

        <!-- Chat window -->
        <div class="flex-1 flex flex-col">
            {#if selectedPeer}
                {#if chatSessions.sessions[selectedPeer]}
                    <div class="p-4 border-b border-gray-200 dark:border-gray-700" style="background-color: {chatSessions.sessions[selectedPeer].color}; opacity: 0.7">
                        <Heading tag="h3">{chatSessions.sessions[selectedPeer].peer_alias}</Heading>
                        <div class="text-sm text-gray-600 dark:text-gray-400">
                            {selectedPeer.substring(0, 8)}...
                        </div>
                    </div>
                {:else}
                    <div class="p-4 border-b border-gray-200 dark:border-gray-700">
                        <Heading tag="h3">New Chat</Heading>
                        <div class="text-sm text-gray-600 dark:text-gray-400">
                            {selectedPeer.substring(0, 8)}...
                        </div>
                    </div>
                {/if}

                <!-- Chat messages -->
                <div class="flex flex-col h-full relative">
    <!-- Chat messages container -->
    <div class="flex-1 overflow-y-auto p-4 pb-40" bind:this={chatContainer}>
        {#if chatHistory.length === 0}
            <p class="text-center text-gray-500 dark:text-gray-400">
                No messages yet. Start the conversation!
            </p>
        {:else}
            <div class="space-y-4">
                {#each chatHistory as message}
                    <div class="flex {message.sender_fingerprint === selectedPeer ? 'justify-start' : 'justify-end'}">
                        <div 
                            class="max-w-[70%] p-3 rounded-lg {message.sender_fingerprint === selectedPeer ? 'bg-gray-200 dark:bg-gray-700' : 'bg-blue-500 text-white'}"
                            style={message.sender_fingerprint === selectedPeer && chatSessions.sessions[selectedPeer] ? `background-color: ${chatSessions.sessions[selectedPeer].color}; opacity: 0.7` : ''}
                        >
                            <div class="text-sm font-medium">
                                {message.sender_alias}
                            </div>
                            <div class="mt-1">
                                {message.content}
                            </div>
                            <div class="text-xs text-right mt-1 {message.sender_fingerprint === selectedPeer ? 'text-gray-500 dark:text-gray-400' : 'text-blue-100'}">
                                {formatTimestamp(message.timestamp)}
                            </div>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </div>

    <!-- Chat input box (always visible) -->
    <div class="p-4 border-t border-gray-200 dark:border-gray-700 bg-white dark:bg-gray-800 fixed left-0 right-0 z-50" style="bottom: var(--bottom-nav-height, 0);">
        <div class="flex">
            <div class="flex-1 relative">
                <textarea
                    class="w-full px-3 py-2 text-sm text-gray-900 bg-white rounded-lg border border-gray-300 focus:ring-blue-500 focus:border-blue-500 dark:bg-gray-700 dark:border-gray-600 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
                    placeholder="Type a message..."
                    bind:value={newMessage}
                    onkeydown={handleKeydown}
                    oninput={autoResize}
                    rows="1"
                    style="min-height: 38px; max-height: 150px; resize: none; overflow-y: auto;"
                    bind:this={textareaElement}
                ></textarea>
            </div>
            <Button class="ml-2" color="blue" onclick={sendMessage}>
                <Fa icon={faPaperPlane} />
            </Button>
        </div>
    </div>
</div>
            {:else}
                <div class="flex-1 flex items-center justify-center">
                    <div class="text-center">
                        <Heading tag="h3" class="mb-4">Select a peer to start chatting</Heading>
                        <p class="text-gray-500 dark:text-gray-400">
                            Choose a peer from the sidebar to start a conversation
                        </p>
                    </div>
                </div>
            {/if}
        </div>
    </div>
</div>
